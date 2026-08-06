//! The data types that cross this crate's boundary.
//!
//! Grouped together rather than each living next to the code that fills them
//! in, because they are one thing: the contract between `core`, the Tauri
//! commands and the frontend. `src/types.ts` mirrors this file field for
//! field, and the serialized shapes here are also what a saved JSON report
//! contains — so a change to any of them has to be made in three places at
//! once, and having them in one file is what makes that visible.
//!
//! Serialization notes live with the fields they constrain (see
//! [`FileAnalysis::size_bytes`] for why `serde(default)` matters to old
//! reports).

use serde::{Deserialize, Serialize};

/// Information about digital clipping found in a file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClippingInfo {
    /// Number of samples at or above the full-scale threshold.
    pub clipped_samples: u64,
    /// Number of clip *events* (runs of >= 3 consecutive full-scale samples).
    pub clip_events: u64,
    /// Peak absolute sample value observed, normalized to [0, 1].
    pub peak: f32,
    /// Peak in dBFS (0.0 == full scale).
    pub peak_dbfs: f32,
    /// True peak (inter-sample peak) magnitude from 4x oversampling
    /// (BS.1770-style). Can exceed 1.0 when the reconstructed waveform
    /// overshoots full scale between samples.
    pub true_peak: f32,
    /// True peak in dBTP. Positive values are inter-sample "overs": the DAC's
    /// reconstruction filter will clip even though no stored sample does.
    pub true_peak_dbtp: f32,
    /// `true` when at least one clip event was detected.
    pub clipped: bool,
}

impl ClippingInfo {
    /// The "nothing measured yet" value used to build a [`FileAnalysis`]
    /// skeleton before decoding, and left in place when decoding fails.
    ///
    /// Peaks start at −∞ dB rather than 0: a file that never decoded has no
    /// peak, and 0 dBFS would read as "clipped at full scale".
    pub fn unmeasured() -> Self {
        Self {
            clipped_samples: 0,
            clip_events: 0,
            peak: 0.0,
            peak_dbfs: f32::NEG_INFINITY,
            true_peak: 0.0,
            true_peak_dbtp: f32::NEG_INFINITY,
            clipped: false,
        }
    }
}

/// The complete analysis result for one file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    /// Absolute path to the analyzed file.
    pub path: String,
    /// File name alone, without its directory (kept alongside `path` since
    /// most UI surfaces show only this).
    pub file_name: String,
    /// Container/codec short name detected from the file's magic bytes
    /// (e.g. "FLAC", "WAV", "MP4"), independent of the file extension.
    pub format: String,
    /// `true` when the real container disagrees with the file extension
    /// (e.g. a WAV renamed to `.mp3`).
    pub ext_mismatch: bool,
    /// Sample rate the file was decoded at, in Hz.
    pub sample_rate: u32,
    /// Channel count the file was decoded with.
    pub channels: usize,
    /// Declared bit depth for integer PCM sources; `None` for float sources.
    pub declared_bits: Option<u32>,
    /// Track length in seconds.
    pub duration_secs: f64,
    /// On-disk size in bytes, straight from the filesystem metadata — the same
    /// number the OS itself reports, never derived from bitrate × duration
    /// (which would drift from what the file manager shows). `0` if the size
    /// could not be read.
    ///
    /// `serde(default)` keeps JSON reports exported before this field existed
    /// loadable: they simply come back with a size of 0.
    #[serde(default)]
    pub size_bytes: u64,

    /// The three LAC-style detections (upscaling / upsampling / transcoding).
    pub detections: crate::detections::Detections,
    /// Detected spectral cutoff frequency in Hz.
    pub cutoff_hz: Option<f64>,
    /// Cutoff frequency as a ratio of Nyquist (cutoff / (sample_rate/2)).
    pub cutoff_ratio: Option<f64>,

    /// Estimated *effective* bit depth (integer sources only).
    pub real_bit_depth: Option<u32>,
    /// AAC re-quantization hit-rate (0..1); high values prove an AAC source.
    pub requant_rate: Option<f32>,
    /// `true` when a >= 2 channel file is actually dual-mono.
    pub fake_stereo: Option<bool>,
    /// Verified quality badge: `Some("Hi-Res")` for > 48 kHz or > 16-bit PCM,
    /// `Some("DSD64")` etc. for DSD — granted only when no detection
    /// invalidates the claim (no upscaling/upsampling/transcoding).
    pub badge: Option<String>,

    /// Clipping / true-peak statistics for this file.
    pub clipping: ClippingInfo,

    /// Dynamic-range estimate in dB (peak vs loudest-20% RMS, DR-meter style).
    /// High (>= 12 dB) == dynamic master (Full Dynamic Range editions);
    /// low (< 8 dB) == loudness-war master. `None` when not measurable.
    pub dr_db: Option<f32>,

    /// FLAC MD5 signature status. `None` for non-FLAC files (no column shown).
    pub flac_md5: Option<crate::flac_md5::FlacMd5Status>,

    /// Populated when analysis failed; other fields hold best-effort defaults.
    pub error: Option<String>,
}

/// A single analyzed folder together with the files it contains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderReport {
    /// Display path of the scanned root — the dropped/selected folder, or the
    /// first target's parent when individual files were dropped.
    pub root: String,
    /// One entry per analyzed file, in the order the report/exports use.
    pub files: Vec<FileAnalysis>,
    /// `true` if any FLAC files were present (the UI shows the MD5 column then).
    pub has_flac: bool,
}

/// Options controlling how a folder is scanned.
///
/// ```
/// use flaccompagnon_core::ScanOptions;
///
/// // Defaults: recurse, verify FLAC MD5, no ffmpeg (DSD headers only).
/// let opts = ScanOptions::default();
/// assert!(opts.recursive);
/// assert!(opts.verify_flac_md5);
/// assert!(opts.ffmpeg.is_none());
///
/// // Skim a single folder, and enable DSD content analysis.
/// let quick = ScanOptions {
///     recursive: false,
///     ffmpeg: Some("/opt/homebrew/bin/ffmpeg".into()),
///     ..ScanOptions::default()
/// };
/// assert!(!quick.recursive);
/// ```
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// Recurse into sub-folders.
    pub recursive: bool,
    /// Compare the FLAC MD5 signature against a hash computed during the single
    /// decode pass (near-free, since analysis decodes every sample anyway).
    /// When false, only the signature's presence is reported.
    pub verify_flac_md5: bool,
    /// Path to an `ffmpeg` binary, used to decode DSD (DSF/DFF) content.
    /// `None` limits DSD files to exact header verification.
    pub ffmpeg: Option<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            verify_flac_md5: true,
            ffmpeg: None,
        }
    }
}

/// Errors that can occur while analyzing.
///
/// Note that [`analyze_file`](crate::analyze_file) never surfaces one of these
/// to its caller: a per-file failure belongs in [`FileAnalysis::error`] so a
/// batch can keep going. These travel inside the crate, between the decoders
/// and the pipeline.
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    /// The file could not be read from disk (permissions, missing file, …).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// The file's audio could not be decoded, or its container/header was
    /// malformed.
    #[error("decode error: {0}")]
    Decode(String),
    /// The file's format is recognized but not supported for analysis.
    #[error("unsupported file: {0}")]
    Unsupported(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file that never decoded must not look like it peaked at full scale.
    #[test]
    fn unmeasured_clipping_has_no_peak() {
        let c = ClippingInfo::unmeasured();
        assert!(!c.clipped);
        assert_eq!(c.clipped_samples, 0);
        assert_eq!(c.clip_events, 0);
        assert!(c.peak_dbfs.is_infinite() && c.peak_dbfs.is_sign_negative());
        assert!(c.true_peak_dbtp.is_infinite() && c.true_peak_dbtp.is_sign_negative());
    }
}
