//! Assembling the `FileAnalysis` record itself — the parts that describe the
//! file rather than the audio inside it.
//!
//! Split out of `pipeline` because they answer a different question and change
//! for different reasons: `pipeline` decides *which decode path a file takes*
//! and what the measurements mean, while everything here is filesystem
//! metadata and arithmetic over it — size, modification time, fingerprints,
//! the bitrate that falls out of size and duration. None of it needs a decoder,
//! and all of it must work on a file that fails to decode at all.

use std::path::Path;

use crate::analysis::detections::Detections;
use crate::types::{ClippingInfo, FileAnalysis};

/// The record we can already fill in before decoding anything, so an
/// unreadable file still produces a useful row.
pub(super) fn skeleton(path: &Path) -> FileAnalysis {
    // Read once, up front, from the filesystem: available even for a file
    // whose audio fails to decode, so an unreadable track still reports an
    // honest size (and modification time) in the table. One `metadata` call
    // rather than two separate ones for size and mtime.
    let meta = std::fs::metadata(path).ok();
    let digest = crate::hash::file_digest(path).ok();
    let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified_unix = meta.as_ref().and_then(|m| m.modified().ok()).and_then(|t| {
        t.duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_secs()).ok())
    });

    FileAnalysis {
        path: path.to_string_lossy().to_string(),
        file_name: path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
        format: String::new(),
        codec: None,
        ext_mismatch: false,
        sample_rate: 0,
        channels: 0,
        declared_bits: None,
        duration_secs: 0.0,
        size_bytes,
        bitrate_kbps: None, // filled in once `duration_secs` is known — see `analyze_file`
        modified_unix,
        detections: Detections::not_analyzed(),
        cutoff_hz: None,
        cutoff_ratio: None,
        real_bit_depth: None,
        bit_depth_evidence: None,
        lattice_score: None,
        fake_stereo: None,
        phase_correlation: None,
        phase_inverted: None,
        badge: None,
        clipping: ClippingInfo::unmeasured(),
        dr_db: None,
        integrated_lufs: None,
        loudness_range_lu: None,
        flac_md5: None,
        // Computed here, beside `size_bytes` and `modified_unix`, and for the
        // same reason: it describes the file as an object rather than the
        // audio inside it, so a track that fails to decode still gets an
        // honest fingerprint. One extra read of the file — negligible next to
        // the spectral analysis and the lattice sweep that follow.
        file_md5: digest.as_ref().map(|d| d.md5.clone()),
        file_crc32: digest.as_ref().map(|d| d.crc32.clone()),
        error: None,
    }
}

/// Average bitrate in kbps from the file's own size and duration — see
/// [`FileAnalysis::bitrate_kbps`] for why this (not a codec-reported figure)
/// is what's shown.
pub(super) fn bitrate_kbps(size_bytes: u64, duration_secs: f64) -> Option<u32> {
    if duration_secs <= 0.0 {
        return None;
    }
    let kbps = (size_bytes as f64 * 8.0) / duration_secs / 1000.0;
    if kbps.is_finite() && kbps >= 0.0 {
        Some(kbps.round() as u32)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "../../tests/unit/pipeline/record.rs"]
mod tests;
