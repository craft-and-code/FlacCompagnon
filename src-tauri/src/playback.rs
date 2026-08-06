//! Single-track audio playback for the results table's hover-to-preview
//! button — entirely separate from the detection/analysis engine, which
//! never touches this module. Decoding reuses
//! `flaccompagnon_core::decode::PcmStreamDecoder` (Symphonia); output goes
//! through `cpal`.
//!
//! Decoding is progressive: `build_stream` only reads the container header
//! (via `PcmStreamDecoder::open`) before opening the audio device, then
//! hands the rest of the decoding to a background thread that feeds a
//! growing sample buffer while the stream is already playing from it — a
//! long or hi-res file starts sounding almost immediately instead of after
//! its whole body has decoded. The one exception is the resampling
//! fallback (see `build_stream`'s second half), which still decodes the
//! whole file up front — see the comment there for why.
//!
//! `cpal::Stream` is not safely movable between threads on every backend, so
//! it never leaves the thread that created it: one dedicated "audio thread",
//! started once from Tauri's `setup` hook, owns every `Stream` for its
//! entire lifetime and is only ever talked to through a channel of [`Cmd`]s.
//! `play`/`stop` (called from the `play_track`/`stop_playback` Tauri
//! commands, inside `spawn_blocking`) just send a command and block on a
//! one-shot reply.
//!
//! Playback is intentionally simple: play or stop, nothing else — no pause,
//! seek or volume control. When a track finishes on its own, a
//! `playback://finished` event lets the frontend decide whether to advance
//! to the next row; the queue itself lives entirely in the frontend, which
//! already knows the table's display order.
//!
//! # Size
//!
//! Over CLAUDE.md's 300-line ceiling, deliberately. This is one subsystem
//! held together by a thread-ownership rule, not a collection of functions: a
//! `cpal::Stream` is not safely `Send` on every backend, so it must never
//! leave the thread that built it. Every piece here — the command channel,
//! the audio thread's loop, `build_stream`, the shared buffer the decode
//! thread fills — exists to keep that invariant true. Splitting them across
//! files would put the rule and the code it constrains in different places,
//! which is precisely how such an invariant gets broken by a later edit that
//! looked local and safe.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

enum Cmd {
    Play(PathBuf, u64, mpsc::Sender<Result<(), String>>),
    Stop,
}

static SENDER: OnceLock<mpsc::Sender<Cmd>> = OnceLock::new();
static REQUEST_SEQ: AtomicU64 = AtomicU64::new(0);

/// Payload of the `playback://finished` event. `request_id` lets the
/// frontend ignore a stale notification from a track that was already
/// superseded by a newer `play_track` call before this one's buffer drained.
#[derive(Clone, Serialize)]
pub struct Finished {
    pub request_id: u64,
}

/// Payload of the `playback://level` event, driving the results table's
/// equalizer bars for whichever row is playing. `level` is a rough perceptual
/// RMS of the samples this callback is about to hand to the output device —
/// enough to look "in sync" with the music, not a calibrated loudness
/// measurement. Same `request_id` convention as [`Finished`].
#[derive(Clone, Serialize)]
pub struct Level {
    pub request_id: u64,
    pub level: f32,
}

/// How often the output callback is allowed to emit a `playback://level`
/// event. The callback itself runs far more often than this (every few
/// milliseconds, at the mercy of the device's buffer size) — throttling here
/// keeps the IPC traffic to something a UI update actually needs, rather than
/// firing an event per callback.
const LEVEL_EMIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(60);

/// A currently-open output stream plus the means to tell its background
/// decode thread (if it has one) to give up early — used when the track is
/// stopped or superseded by a new one before it finished decoding on its
/// own.
struct PlayingTrack {
    // Never read directly — its only job is to stay alive for as long as
    // playback should continue. Dropping it (replacing/stopping the track)
    // tears down the `cpal` stream and stops audio output.
    #[allow(dead_code)]
    stream: cpal::Stream,
    cancel_decode: Arc<AtomicBool>,
}

/// Start the dedicated audio thread. Called once from Tauri's `setup` hook;
/// harmless (a no-op) if called again.
pub fn init(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<Cmd>();
    if SENDER.set(tx).is_err() {
        return; // already initialized
    }
    thread::spawn(move || audio_thread(rx, app));
}

fn audio_thread(rx: mpsc::Receiver<Cmd>, app: AppHandle) {
    // The `cpal::Stream` lives here, and only here, for as long as something
    // is playing — dropping it stops the hardware stream immediately.
    let mut current: Option<PlayingTrack> = None;
    for cmd in rx {
        match cmd {
            Cmd::Stop => {
                if let Some(track) = current.take() {
                    track.cancel_decode.store(true, Ordering::SeqCst);
                }
            }
            Cmd::Play(path, request_id, reply) => {
                // Stop whatever was playing — and tell its decode thread to
                // stop feeding a buffer nothing is reading anymore — before
                // starting the next one.
                if let Some(track) = current.take() {
                    track.cancel_decode.store(true, Ordering::SeqCst);
                }
                match build_stream(&path, app.clone(), request_id) {
                    Ok((stream, cancel_decode)) => match stream.play() {
                        Ok(()) => {
                            current = Some(PlayingTrack {
                                stream,
                                cancel_decode,
                            });
                            let _ = reply.send(Ok(()));
                        }
                        Err(e) => {
                            cancel_decode.store(true, Ordering::SeqCst);
                            let _ = reply.send(Err(e.to_string()));
                        }
                    },
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
        }
    }
}

/// Ask the audio thread to play `path`, blocking for the outcome. Returns
/// the request id used to match the eventual `playback://finished` event.
pub fn play(path: PathBuf) -> Result<u64, String> {
    let sender = SENDER.get().ok_or("Playback engine not started.")?;
    let request_id = REQUEST_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    let (reply_tx, reply_rx) = mpsc::channel();
    sender
        .send(Cmd::Play(path, request_id, reply_tx))
        .map_err(|_| "Playback engine is not running.".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "Playback engine did not respond.".to_string())??;
    Ok(request_id)
}

/// Ask the audio thread to stop whatever is currently playing.
pub fn stop() -> Result<(), String> {
    let sender = SENDER.get().ok_or("Playback engine not started.")?;
    sender
        .send(Cmd::Stop)
        .map_err(|_| "Playback engine is not running.".to_string())
}

/// Sample data feeding the output callback: either a buffer a background
/// thread is still appending to (the common, progressive-decode path) or a
/// plain, already-complete buffer (the resampling fallback, which has
/// nothing left to decode by the time the stream is built).
enum SampleSource {
    Streaming(Arc<StreamBuffer>),
    Static(Arc<Vec<f32>>),
}

impl SampleSource {
    /// Copy up to `out.len()` samples starting at `pos` into `out`,
    /// zero-filling anything not yet available. Returns `(n_copied,
    /// fully_drained)` — `fully_drained` is true once decoding has finished
    /// *and* every decoded sample has been copied out, i.e. genuinely
    /// nothing more will ever arrive (not just "buffer momentarily empty").
    fn read(&self, pos: usize, out: &mut [f32]) -> (usize, bool) {
        let (n, total, done) = match self {
            SampleSource::Streaming(buf) => {
                let samples = buf.samples.lock().unwrap();
                let total = samples.len();
                let end = (pos + out.len()).min(total);
                let n = end.saturating_sub(pos);
                if n > 0 {
                    out[..n].copy_from_slice(&samples[pos..end]);
                }
                (n, total, buf.done.load(Ordering::SeqCst))
            }
            SampleSource::Static(samples) => {
                let total = samples.len();
                let end = (pos + out.len()).min(total);
                let n = end.saturating_sub(pos);
                if n > 0 {
                    out[..n].copy_from_slice(&samples[pos..end]);
                }
                (n, total, true)
            }
        };
        if n < out.len() {
            for s in &mut out[n..] {
                *s = 0.0;
            }
        }
        (n, done && pos + n >= total)
    }
}

/// Growing sample buffer shared between a background decode thread (the
/// producer, appending one packet's worth of samples at a time) and the
/// audio output callback (the consumer, reading from wherever it last left
/// off). `done` marks that decoding has stopped — successfully finished,
/// hit an error, or was cancelled — and no more samples are coming.
struct StreamBuffer {
    samples: Mutex<Vec<f32>>,
    done: AtomicBool,
    cancel: Arc<AtomicBool>,
}

/// Decode `path` on a new background thread, appending each packet's samples
/// to `buf` as they're ready. Stops early if `buf.cancel` is set (the track
/// was stopped or superseded before reaching the end on its own).
fn spawn_decode_thread(mut decoder: flaccompagnon_core::decode::PcmStreamDecoder, buf: Arc<StreamBuffer>) {
    thread::spawn(move || {
        loop {
            if buf.cancel.load(Ordering::SeqCst) {
                break;
            }
            match decoder.next_chunk() {
                Ok(Some(chunk)) => {
                    buf.samples.lock().unwrap().extend_from_slice(&chunk);
                }
                Ok(None) => break,
                Err(e) => {
                    eprintln!("FlacCompagnon: streaming decode error: {e}");
                    break;
                }
            }
        }
        buf.done.store(true, Ordering::SeqCst);
    });
}

fn build_stream(
    path: &Path,
    app: AppHandle,
    request_id: u64,
) -> Result<(cpal::Stream, Arc<AtomicBool>), String> {
    if matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("dsf") | Some("dff")
    ) {
        return Err("Playback preview is not available for DSD files yet.".to_string());
    }

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "No audio output device found.".to_string())?;

    // Open just the container header — fast, no packets decoded yet — so
    // the file's own sample rate/channel count are known without waiting on
    // a full decode.
    let decoder =
        flaccompagnon_core::decode::PcmStreamDecoder::open(path).map_err(|e| e.to_string())?;
    let native_channels = decoder.channels.max(1);
    let native_config = cpal::StreamConfig {
        channels: native_channels as u16,
        sample_rate: decoder.sample_rate,
        buffer_size: cpal::BufferSize::Default,
    };

    // First choice: open the stream at the file's own rate/channels. Shared-
    // mode audio on both macOS (CoreAudio) and Windows (WASAPI) generally
    // accepts an arbitrary requested rate and handles any device-side
    // conversion itself — with a proper resampler, unlike the linear
    // interpolation below — so this both avoids unnecessary resampling *and*
    // is what makes progressive decoding possible: the stream can start
    // before the file is even half-decoded, because nothing needs to look
    // ahead across the whole buffer the way resampling does.
    let cancel = Arc::new(AtomicBool::new(false));
    let stream_buf = Arc::new(StreamBuffer {
        samples: Mutex::new(Vec::new()),
        done: AtomicBool::new(false),
        cancel: cancel.clone(),
    });
    if let Ok(stream) = try_build_stream(
        &device,
        native_config,
        SampleSource::Streaming(stream_buf.clone()),
        app.clone(),
        request_id,
    ) {
        spawn_decode_thread(decoder, stream_buf);
        return Ok((stream, cancel));
    }

    // Fallback: the device rejected the file's native configuration (common
    // when the OS output is locked to a fixed rate, e.g. macOS with "sample
    // rate switching" left off) — decode the whole file up front and
    // resample/remap it to whatever the device does accept. This one path
    // stays non-progressive: linear-interpolating from one packet into the
    // next needs the samples on both sides of that boundary, so a proper
    // streaming version would need a stateful resampler carried across
    // chunks — more machinery than this rare fallback (an already-decoded
    // buffer, just not at the device's rate) has warranted so far. See
    // `resample_and_remap`.
    let pcm = flaccompagnon_core::decode::decode_to_pcm(path).map_err(|e| e.to_string())?;
    let default_config = device
        .default_output_config()
        .map_err(|e| format!("No usable output configuration: {e}"))?;
    let out_channels = default_config.channels() as usize;
    let out_rate = default_config.sample_rate();
    let out_samples = Arc::new(resample_and_remap(&pcm, out_rate, out_channels));
    let config = cpal::StreamConfig {
        channels: out_channels as u16,
        sample_rate: out_rate,
        buffer_size: cpal::BufferSize::Default,
    };
    let fallback_cancel = Arc::new(AtomicBool::new(false)); // nothing decodes in the background here
    try_build_stream(
        &device,
        config,
        SampleSource::Static(out_samples),
        app,
        request_id,
    )
    .map(|stream| (stream, fallback_cancel))
    .map_err(|e| format!("Could not open the audio output device: {e}"))
}

fn try_build_stream(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    source: SampleSource,
    app: AppHandle,
    request_id: u64,
) -> Result<cpal::Stream, cpal::Error> {
    let mut pos: usize = 0; // only ever touched from the audio callback thread
    let mut finished_emitted = false;
    // `Instant::now()` is a monotonic clock read (no syscall on the platforms
    // this targets), cheap enough to call every callback just to check the
    // throttle — the actual RMS pass only runs once it trips.
    let mut last_level_emit = std::time::Instant::now();

    device.build_output_stream(
        config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let (n, drained) = source.read(pos, data);
            pos += n;

            if last_level_emit.elapsed() >= LEVEL_EMIT_INTERVAL {
                last_level_emit = std::time::Instant::now();
                let sum_sq: f32 = data.iter().map(|s| s * s).sum();
                let rms = if data.is_empty() {
                    0.0
                } else {
                    (sum_sq / data.len() as f32).sqrt()
                };
                // A plain fixed gain rather than any calibration: this only
                // has to look lively for typical program material, not report
                // an accurate level.
                let level = (rms * 4.0).min(1.0);
                let _ = app.emit("playback://level", Level { request_id, level });
            }

            if drained && !finished_emitted {
                finished_emitted = true;
                let _ = app.emit("playback://finished", Finished { request_id });
            }
        },
        move |err| eprintln!("FlacCompagnon: audio output error: {err}"),
        None,
    )
}

/// Convert `pcm` from its own sample rate / channel count to the device's
/// output rate and channel count.
///
/// This is a preview-listen convenience, not part of the analysis engine
/// (which never resamples anything): plain linear interpolation for the
/// sample rate, and a simple nearest-channel remap. Good enough to recognize
/// a track by ear — not a claim about audio fidelity.
fn resample_and_remap(
    pcm: &flaccompagnon_core::decode::PcmAudio,
    out_rate: u32,
    out_channels: usize,
) -> Vec<f32> {
    let in_channels = pcm.channels.max(1);
    let in_frames = pcm.samples.len() / in_channels;

    let remap = |frame: &[f32], out: &mut [f32]| {
        if in_channels == out_channels {
            out.copy_from_slice(&frame[..out_channels]);
        } else if in_channels == 1 {
            out.iter_mut().for_each(|o| *o = frame[0]);
        } else {
            for (i, o) in out.iter_mut().enumerate() {
                *o = frame[i.min(in_channels - 1)];
            }
        }
    };

    if pcm.sample_rate == out_rate {
        let mut out = vec![0.0f32; in_frames * out_channels];
        for f in 0..in_frames {
            let frame = &pcm.samples[f * in_channels..f * in_channels + in_channels];
            remap(
                frame,
                &mut out[f * out_channels..f * out_channels + out_channels],
            );
        }
        return out;
    }

    let ratio = pcm.sample_rate as f64 / out_rate as f64;
    let out_frames = ((in_frames as f64) / ratio).floor().max(0.0) as usize;
    let mut out = vec![0.0f32; out_frames * out_channels];
    let mut mixed = vec![0.0f32; in_channels];
    for of in 0..out_frames {
        let src_pos = of as f64 * ratio;
        let i0 = (src_pos.floor() as usize).min(in_frames.saturating_sub(1));
        let i1 = (i0 + 1).min(in_frames.saturating_sub(1));
        let t = (src_pos - i0 as f64) as f32;
        let f0 = &pcm.samples[i0 * in_channels..i0 * in_channels + in_channels];
        let f1 = &pcm.samples[i1 * in_channels..i1 * in_channels + in_channels];
        for c in 0..in_channels {
            mixed[c] = f0[c] + (f1[c] - f0[c]) * t;
        }
        remap(
            &mixed,
            &mut out[of * out_channels..of * out_channels + out_channels],
        );
    }
    out
}
