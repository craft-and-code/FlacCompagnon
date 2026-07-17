//! FlacCompagnon core analysis library.
//!
//! This crate is intentionally free of any Tauri / UI dependency so that every
//! analysis routine can be unit-tested in isolation with plain `cargo test`.
//!
//! Public entry points:
//! * [`analyze_file`]  — analyze a single audio file.
//! * [`analyze_folder`] — analyze every supported audio file inside a folder
//!   (non-recursive by default; see [`ScanOptions`]).

pub mod analyzer;
pub mod bitdepth;
pub mod clipping;
pub mod decode;
pub mod detections;
pub mod dsd;
pub mod flac_md5;
pub mod mdct;
pub mod report;
pub mod requant;
pub mod spectrum;
pub mod stereo;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use decode::{probe_info, BasicInfo};
pub use detections::{Detections, TranscodeState};
pub use flac_md5::FlacMd5Status;

/// Audio file extensions FlacCompagnon will attempt to analyze.
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "flac", "wav", "wave", "aif", "aiff", "aifc", "alac", "m4a", "mp4", "caf", "ogg", "oga",
    "mp3", "aac", "dsf", "dff",
];

/// Returns `true` if `path` has an extension FlacCompagnon knows how to decode.
pub fn is_supported_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

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
    /// `true` when at least one clip event was detected.
    pub clipped: bool,
}

/// The complete analysis result for one file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    pub path: String,
    pub file_name: String,
    /// Container/codec short name detected from the file's magic bytes
    /// (e.g. "FLAC", "WAV", "MP4"), independent of the file extension.
    pub format: String,
    /// `true` when the real container disagrees with the file extension
    /// (e.g. a WAV renamed to `.mp3`).
    pub ext_mismatch: bool,
    pub sample_rate: u32,
    pub channels: usize,
    /// Declared bit depth for integer PCM sources; `None` for float sources.
    pub declared_bits: Option<u32>,
    pub duration_secs: f64,

    /// The three LAC-style detections (upscaling / upsampling / transcoding).
    pub detections: Detections,
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

    pub clipping: ClippingInfo,

    /// Dynamic-range estimate in dB (peak vs loudest-20% RMS, DR-meter style).
    /// High (>= 12 dB) == dynamic master (Full Dynamic Range editions);
    /// low (< 8 dB) == loudness-war master. `None` when not measurable.
    pub dr_db: Option<f32>,

    /// FLAC MD5 signature status. `None` for non-FLAC files (no column shown).
    pub flac_md5: Option<FlacMd5Status>,

    /// Populated when analysis failed; other fields hold best-effort defaults.
    pub error: Option<String>,
}

/// Options controlling how a folder is scanned.
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
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("unsupported file: {0}")]
    Unsupported(String),
}

/// Analyze a single audio file end-to-end.
pub fn analyze_file(path: &Path, opts: &ScanOptions) -> FileAnalysis {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let path_str = path.to_string_lossy().to_string();

    // Base skeleton so we can early-return a useful record on error.
    let mut result = FileAnalysis {
        path: path_str,
        file_name,
        format: String::new(),
        ext_mismatch: false,
        sample_rate: 0,
        channels: 0,
        declared_bits: None,
        duration_secs: 0.0,
        detections: Detections::unknown(),
        cutoff_hz: None,
        cutoff_ratio: None,
        real_bit_depth: None,
        requant_rate: None,
        fake_stereo: None,
        badge: None,
        clipping: ClippingInfo {
            clipped_samples: 0,
            clip_events: 0,
            peak: 0.0,
            peak_dbfs: f32::NEG_INFINITY,
            clipped: false,
        },
        dr_db: None,
        flac_md5: None,
        error: None,
    };

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    let is_flac = ext == "flac";

    // DSD files take a dedicated path: exact header verification, then (when
    // ffmpeg is available) content analysis on the decoded PCM.
    if ext == "dsf" || ext == "dff" {
        analyze_dsd(path, opts, &mut result);
        if let Some(detected) = decode::detect_container(path) {
            if let Some(expected) = decode::ext_canonical(path) {
                result.ext_mismatch = detected != expected;
            }
        }
        return result;
    }

    // Single decode pass. FLAC files go through the fused claxon path, which
    // feeds the analyzer AND hashes the MD5 from the same decoded samples —
    // nothing is decoded twice. If that fast path fails (corrupt or misnamed
    // file), fall back to symphonia so the analysis still happens; the MD5
    // column then reports the error.
    let mut flac_md5_status: Option<FlacMd5Status> = None;
    // Sigma-delta noise heritage of a DSD master, when found in hi-res PCM.
    let mut dsd_heritage: Option<f32> = None;
    let decoded = if is_flac {
        match decode::decode_and_analyze_flac(path, opts.verify_flac_md5) {
            Ok((outcome, md5)) => {
                flac_md5_status = Some(md5);
                Ok(outcome)
            }
            Err(e) => {
                flac_md5_status = Some(FlacMd5Status::Error(e.to_string()));
                decode::decode_and_analyze(path)
            }
        }
    } else {
        decode::decode_and_analyze(path)
    };

    match decoded {
        Ok(outcome) => {
            result.format = outcome.format;
            result.sample_rate = outcome.sample_rate;
            result.channels = outcome.channels;
            result.declared_bits = outcome.declared_bits;
            result.duration_secs = outcome.duration_secs;

            let summary = outcome
                .analyzer
                .finish(outcome.sample_rate, outcome.declared_bits);

            result.cutoff_hz = Some(summary.cutoff_hz);
            result.cutoff_ratio = Some(summary.cutoff_ratio);
            result.requant_rate = summary.requant_rate;
            result.clipping = summary.clipping.clone();
            result.dr_db = summary.dr_db;
            dsd_heritage = dsd::dsd_heritage_check(
                &summary.spectrum_db,
                outcome.sample_rate,
                summary.spectrum_db.len().saturating_sub(1) * 2,
            );

            if outcome.channels >= 2 {
                result.fake_stereo = Some(summary.fake_stereo);
            }
            let real_bits = match (outcome.declared_bits, summary.real_bit_depth) {
                (Some(_), Some(real)) => {
                    result.real_bit_depth = Some(real);
                    Some(real)
                }
                _ => None,
            };
            result.detections = detections::classify(
                &summary,
                outcome.sample_rate,
                outcome.declared_bits,
                real_bits,
            );
        }
        Err(e) => {
            result.error = Some(e.to_string());
        }
    }

    // Real container from magic bytes; flag a mismatch with the extension.
    if let Some(detected) = decode::detect_container(path) {
        result.format = detected.to_string();
        if let Some(expected) = decode::ext_canonical(path) {
            result.ext_mismatch = detected != expected;
        }
    }

    // FLAC MD5 signature, computed during the single decode pass above.
    if is_flac {
        result.flac_md5 = flac_md5_status;
    }

    // Verified Hi-Res badge: hi-res specs that no detection contradicts.
    let hires_specs = result.sample_rate > 48_000 || result.declared_bits.map_or(false, |b| b > 16);
    if hires_specs
        && result.error.is_none()
        && !result.detections.upscaling
        && !result.detections.upsampling
        && result.detections.transcoding != TranscodeState::Detected
    {
        result.badge = Some(if dsd_heritage.is_some() {
            // Hi-res PCM carrying the sigma-delta noise signature of a DSD master.
            "Hi-Res (DSD source)".to_string()
        } else {
            "Hi-Res".to_string()
        });
    }

    result
}

/// DSD analysis: exact DSF/DFF header verification plus, when ffmpeg is
/// available, a content check on the decoded PCM (PCM-source brick wall).
fn analyze_dsd(path: &Path, opts: &ScanOptions, result: &mut FileAnalysis) {
    let info = match dsd::parse(path) {
        Ok(i) => i,
        Err(e) => {
            result.error = Some(e.to_string());
            return;
        }
    };
    result.format = info.label();
    result.sample_rate = info.sample_rate;
    result.channels = info.channels;
    result.duration_secs = info.duration_secs();

    let mut flagged: Option<dsd::PcmSourceCheck> = None;
    let mut analyzed = false;

    if info.dst_compressed {
        result.detections = Detections {
            upscaling: false,
            upsampling: false,
            transcoding: TranscodeState::None,
            detail: "DST-compressed DFF: header verified; content analysis is not supported for DST streams.".to_string(),
            summary: "Unknown".to_string(),
        };
    } else if let Some(ffmpeg) = &opts.ffmpeg {
        let decoded_rate = (info.sample_rate / 8).max(1);
        match decode::decode_and_analyze_dsd(ffmpeg, path, info.channels, decoded_rate) {
            Ok(outcome) => {
                analyzed = true;
                if result.duration_secs == 0.0 {
                    result.duration_secs = outcome.duration_secs;
                }
                let summary = outcome.analyzer.finish(decoded_rate, None);
                result.cutoff_hz = Some(summary.cutoff_hz);
                result.cutoff_ratio = Some(summary.cutoff_ratio);
                result.clipping = summary.clipping.clone();
                result.dr_db = summary.dr_db;
                if info.channels >= 2 {
                    result.fake_stereo = Some(summary.fake_stereo);
                }
                let fft_size = summary.spectrum_db.len().saturating_sub(1) * 2;
                flagged = dsd::pcm_source_check(&summary.spectrum_db, decoded_rate, fft_size);
                result.detections = match flagged {
                    Some(hit) => Detections {
                        upscaling: false,
                        upsampling: true,
                        transcoding: TranscodeState::None,
                        detail: format!(
                            "Upsampling: PCM-sourced DSD — digital brick wall at ~{:.2} kHz ({:.0} dB drop). The 1-bit stream was converted from a {} PCM source.",
                            hit.boundary_hz / 1000.0,
                            hit.drop_db,
                            if hit.boundary_hz < 23_000.0 { "44.1 kHz" } else { "48 kHz" }
                        ),
                        summary: "Flagged".to_string(),
                    },
                    None => Detections {
                        upscaling: false,
                        upsampling: false,
                        transcoding: TranscodeState::None,
                        detail: "Content blends into the sigma-delta noise shaping with no PCM brick wall — consistent with native DSD.".to_string(),
                        summary: "Clean".to_string(),
                    },
                };
            }
            Err(e) => {
                result.error = Some(e.to_string());
            }
        }
    } else {
        result.detections = Detections {
            upscaling: false,
            upsampling: false,
            transcoding: TranscodeState::None,
            detail: "DSD header verified. Install ffmpeg to enable the content authenticity check.".to_string(),
            summary: "Unknown".to_string(),
        };
    }

    // DSD badge: header authentic and content not flagged as PCM-sourced.
    if result.error.is_none() && flagged.is_none() {
        result.badge = Some(if analyzed {
            info.label()
        } else {
            format!("{} (unverified)", info.label())
        });
    }
}

/// A single analyzed folder together with the files it contains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderReport {
    pub root: String,
    pub files: Vec<FileAnalysis>,
    /// `true` if any FLAC files were present (the UI shows the MD5 column then).
    pub has_flac: bool,
}

/// List every supported audio file under `root`, sorted, skipping any file that
/// lives inside a generated `spectres` folder.
pub fn list_audio_files(root: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let walker = walkdir::WalkDir::new(root).max_depth(if recursive { usize::MAX } else { 1 });
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_file() && is_supported_audio(p) {
            if p.components().any(|c| c.as_os_str() == "spectres") {
                continue;
            }
            paths.push(p.to_path_buf());
        }
    }
    paths.sort();
    paths
}

/// Analyze every supported audio file under `root`.
pub fn analyze_folder(root: &Path, opts: &ScanOptions) -> Result<FolderReport, AnalysisError> {
    let paths = list_audio_files(root, opts.recursive);
    let files: Vec<FileAnalysis> = paths.iter().map(|p| analyze_file(p, opts)).collect();
    let has_flac = files.iter().any(|f| f.flac_md5.is_some());

    Ok(FolderReport {
        root: root.to_string_lossy().to_string(),
        files,
        has_flac,
    })
}
