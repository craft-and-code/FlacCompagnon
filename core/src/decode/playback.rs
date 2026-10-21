//! Decoding for the preview player rather than for analysis: no analyzer, no
//! integer reconstruction, just interleaved `f32` out.
//!
//! The progressive reader delegates to aede-audio. The whole-file helper remains
//! for explicit conversion/analysis workflows; preview playback never uses it.
//!
//! DSD (`.dsf`/`.dff`) is not handled here — Symphonia cannot decode it, and
//! callers check for it before getting this far.

use std::path::Path;

use crate::AnalysisError;

/// Fully-decoded PCM audio: interleaved `f32` samples in `[-1.0, 1.0]`, at the
/// file's own sample rate and channel count.
pub struct PcmAudio {
    /// Interleaved `f32` samples, `[-1.0, 1.0]`.
    pub samples: Vec<f32>,
    /// Sample rate the file was decoded at, in Hz.
    pub sample_rate: u32,
    /// Channel count the samples are interleaved across.
    pub channels: usize,
}

/// Decode `path` into an in-memory interleaved PCM buffer.
///
/// Built on [`PcmStreamDecoder`] rather than repeating the decode loop: that
/// costs one `Vec` per packet over a hand-rolled version, which is invisible
/// next to decoding the file at all, and it means a fix to the packet loop
/// cannot land in one of the two paths and miss the other.
///
/// FLAC is the exception, and takes claxon first — see
/// [`super::decode_flac_to_pcm`] for why. Symphonia still runs if claxon
/// declines, so neither library being unhappy with a particular file is on
/// its own fatal.
pub fn decode_to_pcm(path: &Path) -> Result<PcmAudio, AnalysisError> {
    let is_flac = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("flac"));
    if is_flac {
        if let Ok(pcm) = super::decode_flac_to_pcm(path) {
            return Ok(pcm);
        }
    }
    let mut decoder = PcmStreamDecoder::open(path)?;
    let sample_rate = decoder.sample_rate;
    let channels = decoder.channels;

    let mut samples: Vec<f32> = Vec::new();
    while let Some(chunk) = decoder.next_chunk()? {
        samples.extend_from_slice(&chunk);
    }

    if samples.is_empty() {
        return Err(AnalysisError::Decode("no audio data decoded".into()));
    }

    Ok(PcmAudio {
        samples,
        sample_rate,
        channels,
    })
}

/// Compatibility adapter over the shared catalogue-independent progressive reader.
/// Analysis decoding retains its separate original-precision paths.
pub struct PcmStreamDecoder {
    inner: aede_audio::decode::PcmStreamDecoder,
    /// Source frames per second.
    pub sample_rate: u32,
    /// Interleaved channels per frame.
    pub channels: usize,
}
impl PcmStreamDecoder {
    /// Probe a local audio file without decoding the whole track.
    pub fn open(path: &Path) -> Result<Self, AnalysisError> {
        let inner = aede_audio::decode::PcmStreamDecoder::open(path)
            .map_err(|e| AnalysisError::Decode(e.to_string()))?;
        Ok(Self {
            sample_rate: inner.sample_rate,
            channels: inner.channels,
            inner,
        })
    }
    /// Yield finite complete PCM packets using the shared decoder.
    pub fn next_chunk(&mut self) -> Result<Option<Vec<f32>>, AnalysisError> {
        self.inner
            .next_chunk()
            .map_err(|e| AnalysisError::Decode(e.to_string()))
    }
}
