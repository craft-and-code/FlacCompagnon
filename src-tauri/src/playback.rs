//! Single-track audio playback for the results table's hover-to-preview
//! button and the app's footer transport — entirely separate from the
//! detection/analysis engine, which never touches this module. Decoding
//! reuses `flaccompagnon_core::decode::PcmStreamDecoder` (Symphonia); output
//! goes through `cpal`.
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
//! `play`/`stop`/`pause`/`resume`/`seek` (called from the matching
//! `*_playback` Tauri commands, inside `spawn_blocking`) just send a command
//! and block on a one-shot reply.
//!
//! Pause and resume are `cpal::Stream::pause()`/`play()` — the stream itself
//! stays open, so resuming is instant and the decode buffer keeps whatever
//! it already had. Seeking moves a shared playback position (in frames,
//! behind a `Mutex` the output callback also reads every buffer) rather than
//! rebuilding the stream; volume and mute are two global atomics the same
//! callback multiplies into every sample it hands to the device, after
//! reading them and before the level meter measures them — muted or quiet
//! audio should look quiet on the results table's equalizer bars too, not
//! just sound quiet. None of this touches the decode thread or the
//! `StreamBuffer` it fills: seeking past what has decoded so far just plays
//! silence until the buffer catches up, exactly like the normal startup
//! buffering does.
//!
//! When a track finishes on its own, a `playback://finished` event lets the
//! frontend decide whether to advance to the next row — the queue itself
//! lives entirely in the frontend, which already knows the table's display
//! order. `playback://position` reports the current playhead on the same
//! throttled cadence as `playback://level`, driving the footer's seek bar.
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
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

enum Cmd {
    Play(PathBuf, u64, mpsc::Sender<Result<(), String>>),
    Stop,
    Pause(mpsc::Sender<Result<(), String>>),
    Resume(mpsc::Sender<Result<(), String>>),
    Seek(f64, mpsc::Sender<Result<(), String>>),
}

static SENDER: OnceLock<mpsc::Sender<Cmd>> = OnceLock::new();
static REQUEST_SEQ: AtomicU64 = AtomicU64::new(0);

/// Playback volume, 0.0–1.0, applied as a plain gain multiplier in the
/// output callback. Stored as the bit pattern of an `f32` (`AtomicU32` has no
/// `f32` counterpart) — `to_bits`/`from_bits` are lossless and don't reorder
/// the value, so this is exactly the float it looks like, just parked in an
/// atomic that can be shared with the callback without a lock.
static VOLUME_BITS: AtomicU32 = AtomicU32::new(1.0f32.to_bits());

/// Independent of `VOLUME_BITS` so un-muting restores the exact volume the
/// user had set, rather than the mute button having to remember and
/// reapply it.
static MUTED: AtomicBool = AtomicBool::new(false);

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

/// Payload of the `playback://position` event, driving the footer's seek
/// bar. `position_secs` is the playhead measured against the stream's own
/// rate (the file's native rate on the common path, the device's rate on the
/// resampling fallback — see `BuiltStream::rate`), so it stays accurate
/// either way.
#[derive(Clone, Serialize)]
pub struct Position {
    pub request_id: u64,
    pub position_secs: f64,
}

/// How often the output callback is allowed to emit a `playback://level` /
/// `playback://position` pair. The callback itself runs far more often than
/// this (every few milliseconds, at the mercy of the device's buffer size) —
/// throttling here keeps the IPC traffic to something a UI update actually
/// needs, rather than firing an event per callback.
const EMIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(60);

/// A currently-open output stream plus everything a command arriving later
/// (pause, resume, seek, stop) needs to act on it.
struct PlayingTrack {
    // Read directly by `Cmd::Pause`/`Cmd::Resume` (`stream.pause()`/
    // `.play()`) — dropping it (replacing/stopping the track) tears down the
    // `cpal` stream and stops audio output.
    stream: cpal::Stream,
    cancel_decode: Arc<AtomicBool>,
    /// Current playhead, in frames — shared with the output callback, which
    /// advances it every buffer, and with `Cmd::Seek`, which overwrites it.
    /// Both go through this same `Mutex` rather than an atomic: a seek has to
    /// fully happen-before or happen-after one callback's "read, then
    /// advance" pair, never in the middle of it, or the advance would
    /// silently undo the seek.
    position: Arc<Mutex<usize>>,
    /// The rate `position` is measured against — the file's own rate on the
    /// streaming path, the device's on the resampling fallback. (Only the
    /// rate is needed here: `Cmd::Seek` converts seconds to *frames*, and the
    /// frames-to-flat-sample-index step that actually needs the channel
    /// count happens inside the output callback itself, from its own local
    /// `channels` — see `try_build_stream`.)
    rate: u32,
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
            Cmd::Pause(reply) => {
                let result = match &current {
                    Some(track) => track.stream.pause().map_err(|e| e.to_string()),
                    None => Err("Nothing is playing.".to_string()),
                };
                let _ = reply.send(result);
            }
            Cmd::Resume(reply) => {
                let result = match &current {
                    Some(track) => track.stream.play().map_err(|e| e.to_string()),
                    None => Err("Nothing is playing.".to_string()),
                };
                let _ = reply.send(result);
            }
            Cmd::Seek(seconds, reply) => {
                let result = match &current {
                    Some(track) => {
                        // `as usize` on a float is a saturating cast in Rust
                        // (stable since 1.45, never UB): a negative or NaN
                        // input (already floored to 0.0 by `.max(0.0)`, but
                        // defence in depth since this ultimately comes from
                        // the frontend) lands on 0, an absurdly large one on
                        // `usize::MAX` — never a panic on a value this
                        // command doesn't control.
                        let frame = seconds.max(0.0) * track.rate as f64;
                        *track.position.lock().unwrap() = frame as usize;
                        Ok(())
                    }
                    None => Err("Nothing is playing.".to_string()),
                };
                let _ = reply.send(result);
            }
            Cmd::Play(path, request_id, reply) => {
                // Stop whatever was playing — and tell its decode thread to
                // stop feeding a buffer nothing is reading anymore — before
                // starting the next one.
                if let Some(track) = current.take() {
                    track.cancel_decode.store(true, Ordering::SeqCst);
                }
                match build_stream(&path, app.clone(), request_id) {
                    Ok(built) => match built.stream.play() {
                        Ok(()) => {
                            current = Some(PlayingTrack {
                                stream: built.stream,
                                cancel_decode: built.cancel_decode,
                                position: built.position,
                                rate: built.rate,
                            });
                            let _ = reply.send(Ok(()));
                        }
                        Err(e) => {
                            built.cancel_decode.store(true, Ordering::SeqCst);
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

fn round_trip(
    cmd_of_reply: impl FnOnce(mpsc::Sender<Result<(), String>>) -> Cmd,
) -> Result<(), String> {
    let sender = SENDER.get().ok_or("Playback engine not started.")?;
    let (reply_tx, reply_rx) = mpsc::channel();
    sender
        .send(cmd_of_reply(reply_tx))
        .map_err(|_| "Playback engine is not running.".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "Playback engine did not respond.".to_string())?
}

/// Pause the currently playing track in place (the stream stays open —
/// resuming is instant and picks up exactly where playback left off).
pub fn pause() -> Result<(), String> {
    round_trip(Cmd::Pause)
}

/// Resume a track paused with [`pause`].
pub fn resume() -> Result<(), String> {
    round_trip(Cmd::Resume)
}

/// Move the currently playing track's playhead to `seconds` from its start.
/// Seeking past what has decoded so far plays silence until decoding catches
/// up, the same way startup buffering already does.
pub fn seek(seconds: f64) -> Result<(), String> {
    round_trip(|reply| Cmd::Seek(seconds, reply))
}

/// Set the playback gain, clamped to 0.0–1.0. Takes effect on the very next
/// output buffer — there is no separate "apply" step and nothing to fail on,
/// so this never blocks on the audio thread the way play/stop/pause/seek do.
pub fn set_volume(volume: f32) {
    VOLUME_BITS.store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}

/// Mute or unmute without touching the stored volume — see `MUTED`'s doc
/// comment for why this is a separate flag rather than zeroing the volume.
pub fn set_muted(muted: bool) {
    MUTED.store(muted, Ordering::Relaxed);
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
/// audio output callback (the consumer, reading from wherever the shared
/// playback position currently points — see `PlayingTrack::position`).
/// `done` marks that decoding has stopped — successfully finished, hit an
/// error, or was cancelled — and no more samples are coming.
struct StreamBuffer {
    samples: Mutex<Vec<f32>>,
    done: AtomicBool,
    cancel: Arc<AtomicBool>,
}

/// Decode `path` on a new background thread, appending each packet's samples
/// to `buf` as they're ready. Stops early if `buf.cancel` is set (the track
/// was stopped or superseded before reaching the end on its own).
fn spawn_decode_thread(
    mut decoder: flaccompagnon_core::decode::PcmStreamDecoder,
    buf: Arc<StreamBuffer>,
) {
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

/// Everything the `Cmd::Play` handler needs to keep from a successful
/// `build_stream` call: the open output stream itself, the flag that tells
/// its background decode thread (if any) to give up early, the shared
/// playback position (frames — read and advanced by the output callback,
/// overwritten by a `Cmd::Seek`), and the rate that position is measured
/// against.
struct BuiltStream {
    stream: cpal::Stream,
    cancel_decode: Arc<AtomicBool>,
    position: Arc<Mutex<usize>>,
    rate: u32,
}

fn build_stream(path: &Path, app: AppHandle, request_id: u64) -> Result<BuiltStream, String> {
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
    let native_rate = decoder.sample_rate;
    let native_config = cpal::StreamConfig {
        channels: native_channels as u16,
        sample_rate: native_rate,
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
    let position = Arc::new(Mutex::new(0usize));
    if let Ok(stream) = try_build_stream(
        &device,
        native_config,
        SampleSource::Streaming(stream_buf.clone()),
        position.clone(),
        app.clone(),
        request_id,
    ) {
        spawn_decode_thread(decoder, stream_buf);
        return Ok(BuiltStream {
            stream,
            cancel_decode: cancel,
            position,
            rate: native_rate,
        });
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
    let stream = try_build_stream(
        &device,
        config,
        SampleSource::Static(out_samples),
        position.clone(),
        app,
        request_id,
    )
    .map_err(|e| format!("Could not open the audio output device: {e}"))?;
    Ok(BuiltStream {
        stream,
        cancel_decode: fallback_cancel,
        position,
        rate: out_rate,
    })
}

fn try_build_stream(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    source: SampleSource,
    position: Arc<Mutex<usize>>,
    app: AppHandle,
    request_id: u64,
) -> Result<cpal::Stream, cpal::Error> {
    let channels = (config.channels as usize).max(1);
    let rate = config.sample_rate.max(1);
    let mut finished_emitted = false;
    // `Instant::now()` is a monotonic clock read (no syscall on the platforms
    // this targets), cheap enough to call every callback just to check the
    // throttle — the actual RMS pass only runs once it trips.
    let mut last_emit = std::time::Instant::now();

    device.build_output_stream(
        config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            // One critical section covers "read the current position, read
            // samples from it, advance it" — a `Cmd::Seek` arriving from the
            // audio thread takes the same lock, so it either lands fully
            // before this section (this callback plays from the new spot) or
            // fully after (the *next* callback does) — never in the middle,
            // which is what would let this callback's advance silently
            // overwrite a seek with a stale value.
            let (drained, frame_pos_after) = {
                let mut p = position.lock().unwrap();
                let frame_pos = *p;
                let flat_pos = frame_pos.saturating_mul(channels);
                let (n, drained) = source.read(flat_pos, data);
                *p = frame_pos + n / channels;
                (drained, *p)
            };

            // Volume/mute: applied after reading, before the level meter or
            // the device sees the samples, so muted/quiet audio also looks
            // quiet on the equalizer bars.
            let gain = if MUTED.load(Ordering::Relaxed) {
                0.0
            } else {
                f32::from_bits(VOLUME_BITS.load(Ordering::Relaxed))
            };
            if gain != 1.0 {
                for s in data.iter_mut() {
                    *s *= gain;
                }
            }

            if last_emit.elapsed() >= EMIT_INTERVAL {
                last_emit = std::time::Instant::now();
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
                let position_secs = frame_pos_after as f64 / rate as f64;
                let _ = app.emit(
                    "playback://position",
                    Position {
                        request_id,
                        position_secs,
                    },
                );
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
