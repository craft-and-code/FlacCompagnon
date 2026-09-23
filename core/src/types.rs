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
    /// Container short name detected from the file's magic bytes (e.g.
    /// "FLAC", "WAV", "MP4"), independent of the file extension.
    pub format: String,
    /// The codec actually carried inside `format`, when that distinction
    /// means something — an MP4 container can hold ALAC or AAC, an OGG can
    /// hold Vorbis or Opus, and `format` alone can't tell those apart. `None`
    /// for single-codec containers (FLAC, DSD) where it would just repeat
    /// `format`, and for codecs the decode path doesn't identify by name
    /// (only the generic Symphonia path populates this; FLAC's fused path and
    /// the DSD/ffmpeg path leave it `None` since their container already says
    /// everything this field would).
    #[serde(default)]
    pub codec: Option<String>,
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
    /// Average bitrate in kbps, computed as `size_bytes * 8 / duration_secs`
    /// — the same "overall bit rate" a tool like MediaInfo reports, container
    /// overhead included, rather than the codec's own internal figure (which
    /// Symphonia doesn't expose uniformly across formats anyway). `None`
    /// when the duration isn't known (e.g. analysis failed before decoding).
    #[serde(default)]
    pub bitrate_kbps: Option<u32>,
    /// Filesystem modification time, as a Unix timestamp (seconds since the
    /// epoch) — read alongside `size_bytes`, from the same `fs::metadata`
    /// call. `None` if the file's metadata couldn't be read, or the platform
    /// doesn't report a modification time.
    #[serde(default)]
    pub modified_unix: Option<i64>,

    /// The three LAC-style detections (upscaling / upsampling / transcoding).
    pub detections: crate::analysis::detections::Detections,
    /// Detected spectral cutoff frequency in Hz.
    pub cutoff_hz: Option<f64>,
    /// Cutoff frequency as a ratio of Nyquist (cutoff / (sample_rate/2)).
    pub cutoff_ratio: Option<f64>,

    /// Estimated *effective* bit depth (integer sources only).
    pub real_bit_depth: Option<u32>,
    /// Supporting integer precision measurement; absent in older saved reports.
    #[serde(default)]
    pub bit_depth_evidence: Option<crate::analysis::bitdepth::BitDepthEvidence>,
    /// How strongly the audio sits on a lossy codec's quantization lattice,
    /// in `0..1` — the criterion `L` of [`crate::transcode`], maximised over
    /// every alignment, frame, window shape and stereo mode tried, and over
    /// both codecs.
    ///
    /// This is the *evidence*, where `Detections::transcoding` is only the
    /// verdict. Shown as its own column because the two answer different
    /// questions: "is this file transcoded" and "how sure are we". A file at
    /// 0.19 against a 0.031 threshold and one at 0.032 are both "Transcoded",
    /// and they do not deserve the same confidence.
    ///
    /// `None` means the search did not run — an untabulated sample rate, a
    /// file too short, or a decode that failed. **Not** the same as a low
    /// score, and the reason this is an `Option` rather than a `0.0`.
    pub lattice_score: Option<f32>,
    /// `true` when a >= 2 channel file is actually dual-mono.
    pub fake_stereo: Option<bool>,
    /// L/R correlation (-1..1), measured on two-channel audio. Absent for
    /// mono, multichannel or silent-channel files and older saved reports.
    #[serde(default)]
    pub phase_correlation: Option<f32>,
    /// Strongly opposed channels suggest one has inverted polarity.
    /// `None` means no two-channel phase analysis was available.
    #[serde(default)]
    pub phase_inverted: Option<bool>,
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
    /// ITU-R BS.1770-5 integrated loudness in LUFS. Absent for silence, short
    /// files, unsupported channel layouts and older saved reports.
    #[serde(default)]
    pub integrated_lufs: Option<f32>,
    /// EBU Tech 3342 loudness range in LU. Absent for silence, very short
    /// files, unsupported channel layouts and older saved reports.
    #[serde(default)]
    pub loudness_range_lu: Option<f32>,

    /// FLAC MD5 signature status. `None` for non-FLAC files (no column shown).
    pub flac_md5: Option<crate::decode::FlacMd5Status>,

    /// MD5 of the **file's bytes**, lowercase hex — tags and cover art
    /// included. Deliberately not the same thing as [`Self::flac_md5`], which
    /// is about the decoded audio; see [`crate::hash`] for the distinction
    /// and why both exist. `None` when the file could not be read.
    #[serde(default)]
    pub file_md5: Option<String>,

    /// CRC32 of the file's bytes, 8 lowercase hex characters — the `.sfv`
    /// variant. `None` when the file could not be read.
    #[serde(default)]
    pub file_crc32: Option<String>,

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
#[path = "../tests/unit/types.rs"]
mod tests;
