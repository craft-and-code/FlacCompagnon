//! Decoding audio into something the rest of the crate can analyze or play.
//!
//! One file per decode path, because they share the container-probing step and
//! almost nothing else:
//!
//! * [`stream`] — the generic path. Symphonia decodes any supported format and
//!   the samples are streamed into a [`StreamAnalyzer`], never held whole.
//! * [`flac`] — FLAC's fused path: one pass that feeds the analyzer *and*
//!   computes the STREAMINFO MD5 over the exact original integers.
//! * [`dsd`] — DSD (DSF/DFF), which Symphonia cannot decode; ffmpeg converts
//!   the 1-bit stream to PCM and we analyze that.
//! * [`playback`] — decoding for the preview player rather than for analysis:
//!   whole-file and progressive variants, no analyzer involved.
//! * [`probe`] — the shared "open the file, find its default track" step, plus
//!   the header-only [`probe_info`].
//! * [`container`] — what the file *actually* is, from its magic bytes, versus
//!   what its extension claims.
//!
//! Every path exposes audio as normalized `f32`. For integer PCM sources the
//! exact integer sample values are reconstructed from those floats (the
//! conversion is lossless for ≤ 24-bit content, since an `f32` mantissa
//! represents every integer up to 2^24 exactly) and fed to the bit-depth
//! estimator alongside.
//!
//! [`StreamAnalyzer`]: crate::analyzer::StreamAnalyzer

pub mod container;
pub mod dsd;
pub mod flac;
pub mod playback;
pub mod probe;
pub mod stream;

use crate::analyzer::StreamAnalyzer;

pub use container::{detect_container, ext_canonical};
pub use dsd::decode_and_analyze_dsd;
pub use flac::decode_and_analyze_flac;
pub use playback::{decode_to_pcm, PcmAudio, PcmStreamDecoder};
pub use probe::{probe_info, BasicInfo};
pub use stream::decode_and_analyze;

/// Result of decoding a file: metadata plus a fully-fed analyzer ready for
/// [`StreamAnalyzer::finish`].
pub struct DecodeOutcome {
    /// Human-readable format label (e.g. "FLAC", "DSF").
    pub format: String,
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
