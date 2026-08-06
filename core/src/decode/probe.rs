//! Opening a file and finding its default audio track.
//!
//! This exists because four call sites needed the same twenty lines — probe
//! the container, take the default track, pull the sample rate and channel
//! count off its codec parameters — and had four copies of them, which is how
//! they had already started to drift apart (only one of them disabled gapless
//! playback, for no stated reason). [`ProbedTrack`] is that step, once.

use std::fs::File;
use std::path::Path;

use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{CodecParameters, Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::container::format_label;
use crate::AnalysisError;

/// A probed file: its reader, positioned before the first packet, and the
/// codec parameters of the track worth decoding.
pub(super) struct ProbedTrack {
    pub format: Box<dyn FormatReader>,
    pub track_id: u32,
    pub params: CodecParameters,
}

/// Probe `path` and locate its default track.
///
/// `gapless` mirrors Symphonia's `enable_gapless`: it trims the encoder delay
/// and padding declared by formats that record them (MP3/AAC). Analysis and
/// playback want it on, so what we measure and what a player would output are
/// the same samples. [`probe_info`] leaves it off because it never reads a
/// packet, so the setting cannot affect its answer either way.
pub(super) fn probe(path: &Path, gapless: bool) -> Result<ProbedTrack, AnalysisError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions {
                enable_gapless: gapless,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .map_err(|e| AnalysisError::Decode(format!("probe failed: {e}")))?;

    let format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| AnalysisError::Decode("no default track".into()))?;
    let track_id = track.id;
    let params = track.codec_params.clone();

    Ok(ProbedTrack {
        format,
        track_id,
        params,
    })
}

impl ProbedTrack {
    pub(super) fn sample_rate(&self) -> Result<u32, AnalysisError> {
        self.params
            .sample_rate
            .ok_or_else(|| AnalysisError::Decode("unknown sample rate".into()))
    }

    pub(super) fn channels(&self) -> Result<usize, AnalysisError> {
        self.params
            .channels
            .map(|c| c.count())
            .ok_or_else(|| AnalysisError::Decode("unknown channel layout".into()))
    }

    pub(super) fn make_decoder(&self) -> Result<Box<dyn Decoder>, AnalysisError> {
        symphonia::default::get_codecs()
            .make(&self.params, &DecoderOptions::default())
            .map_err(|e| AnalysisError::Decode(format!("no decoder: {e}")))
    }
}

/// A reusable interleaved-`f32` scratch buffer for decoded packets.
///
/// Symphonia hands back a typed `AudioBufferRef` per packet; every caller
/// wants the same flat `&[f32]`. Allocating one buffer per packet would churn
/// the allocator once per ~4096 frames for the whole file, so the buffer is
/// kept and only reallocated when a later packet needs a bigger one (block
/// sizes can vary within a stream).
#[derive(Default)]
pub(super) struct InterleavedBuf {
    buf: Option<SampleBuffer<f32>>,
    frames: u64,
}

impl InterleavedBuf {
    pub(super) fn fill(&mut self, decoded: AudioBufferRef<'_>) -> &[f32] {
        let needed = decoded.capacity() as u64;
        if self.buf.is_none() || needed > self.frames {
            let spec = *decoded.spec();
            self.buf = Some(SampleBuffer::<f32>::new(needed, spec));
            self.frames = needed;
        }
        // Set on the line above if it was missing, so this cannot be None.
        let buf = self.buf.as_mut().expect("buffer just ensured present");
        buf.copy_interleaved_ref(decoded);
        buf.samples()
    }
}

/// Lightweight header information obtained without decoding the whole file.
#[derive(Debug, Clone)]
pub struct BasicInfo {
    /// Sample rate declared by the header, in Hz.
    pub sample_rate: u32,
    /// Channel count declared by the header.
    pub channels: usize,
    /// Bit depth declared by the header, when the format has one.
    pub bits: Option<u32>,
    /// Human-readable format label (e.g. "FLAC", "DSF").
    pub format: String,
}

/// Read basic stream parameters from a file's header only (fast — no full
/// decode). Used e.g. to caption spectrogram images.
pub fn probe_info(path: &Path) -> Result<BasicInfo, AnalysisError> {
    let probed = probe(path, false)?;
    Ok(BasicInfo {
        // Unlike the decode paths, a missing rate or channel count is not
        // fatal here: this only captions an image, and "0" is a better answer
        // than refusing to render the spectrogram at all.
        sample_rate: probed.params.sample_rate.unwrap_or(0),
        channels: probed.params.channels.map(|c| c.count()).unwrap_or(0),
        bits: probed.params.bits_per_sample,
        format: format_label(path),
    })
}
