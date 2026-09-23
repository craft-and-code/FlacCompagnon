//! Decoding audio into something the rest of the crate can analyze or play.
//!
//! One file per decode path, because they share the container-probing step and
//! almost nothing else:
//!
//! * [`stream`] — the generic path. Symphonia decodes any supported format and
//!   the samples are streamed into a [`StreamAnalyzer`], never held whole.
//! * [`flac`] — FLAC's fused path: one pass that feeds the analyzer *and*
//!   computes the STREAMINFO MD5 over the exact original integers.
//! * [`flac_md5`] — the verdict that pass produces ([`FlacMd5Status`]). It
//!   lives here rather than under `analysis/` because this is where it is
//!   decided: the MD5 comparison is part of the FLAC decode, by design, so
//!   the file is read once for both the hash and the measurements.
//! * [`dsd`] — DSD (DSF/DFF), which Symphonia cannot decode; ffmpeg converts
//!   the 1-bit stream to PCM and we analyze that.
//! * [`playback`] — decoding for the preview player rather than for analysis:
//!   whole-file and progressive variants, no analyzer involved.
//! * [`probe`] — the shared "open the file, find its default track" step, plus
//!   the header-only [`probe_info`].
//! * [`container`] — what the file *actually* is, from its magic bytes, versus
//!   what its extension claims.
//!
//! Every path exposes audio as normalized `f32` for spectral analysis.
//! Integer PCM sources also supply exact integers directly to the bit-depth
//! measurement, without a float round-trip that could discard low bits.
//!
//! [`StreamAnalyzer`]: crate::analysis::analyzer::StreamAnalyzer

pub mod container;
pub mod dsd;
pub mod flac;
pub mod flac_md5;
pub mod playback;
pub mod probe;
pub mod stream;

use crate::analysis::analyzer::StreamAnalyzer;

pub use container::{detect_container, ext_canonical};
pub use dsd::decode_and_analyze_dsd;
pub use flac::{decode_and_analyze_flac, decode_flac_to_pcm};
pub use flac_md5::FlacMd5Status;
pub use playback::{decode_to_pcm, PcmAudio, PcmStreamDecoder};
pub use probe::{probe_info, BasicInfo};
pub use stream::decode_and_analyze;

/// Result of decoding a file: metadata plus a fully-fed analyzer ready for
/// [`StreamAnalyzer::finish`].
pub struct DecodeOutcome {
    /// Human-readable format label (e.g. "FLAC", "DSF").
    pub format: String,
    /// The codec inside `format`, when Symphonia can name it and it isn't
    /// just repeating `format` — see [`crate::types::FileAnalysis::codec`].
    /// Only the generic Symphonia path ([`stream::decode_and_analyze`])
    /// populates this; FLAC's fused path and the DSD/ffmpeg path leave it
    /// `None`.
    pub codec: Option<String>,
    /// Sample rate actually decoded, in Hz.
    pub sample_rate: u32,
    /// Channel count actually decoded.
    pub channels: usize,
    /// Bit depth declared by the container, when it has one (DSD and some
    /// float formats don't).
    pub declared_bits: Option<u32>,
    /// Track length in seconds.
    pub duration_secs: f64,
    /// The analyzer, fed with every decoded sample and ready to be finished.
    pub analyzer: StreamAnalyzer,
}

/// A padding verdict requires actual samples and must not describe a prefix
/// as the whole file when the container declares that samples are missing.
pub(super) fn validate_decoded_frames(
    frames: u64,
    declared_frames: Option<u64>,
) -> Result<(), crate::AnalysisError> {
    if frames == 0 {
        return Err(crate::AnalysisError::Decode("no audio data decoded".into()));
    }
    if declared_frames.is_some_and(|declared| frames < declared) {
        return Err(crate::AnalysisError::Decode(
            "incomplete audio stream".into(),
        ));
    }
    Ok(())
}
