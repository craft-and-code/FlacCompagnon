//! Human-readable analysis measurements.

use crate::args::Args;
use flaccompagnon_core::{decode::FlacMd5Status, FileAnalysis};

fn flac_md5_label(status: Option<&FlacMd5Status>) -> String {
    let Some(status) = status else {
        return "N/A".to_string();
    };
    let label = status.label();
    // The column already identifies MD5; reuse the core wording without its prefix.
    label.strip_prefix("MD5 ").unwrap_or(&label).to_string()
}

pub(crate) fn display(file: &FileAnalysis, args: &Args) {
    println!("{}", file.path);
    if let Some(error) = &file.error {
        println!("  Error: {error}");
        return;
    }
    if args.selected("authenticity") {
        println!(
            "  Authenticity: {} — {}",
            file.detections.summary, file.detections.detail
        );
    }
    if args.selected("bit-depth") {
        println!(
            "  Bit depth: declared {:?}, effective {:?}",
            file.declared_bits, file.real_bit_depth
        );
    }
    if args.selected("spectrum") {
        println!("  Spectral cutoff: {:?} Hz", file.cutoff_hz);
    }
    if args.selected("stereo") {
        println!(
            "  Stereo: fake {:?}, balance {:?}",
            file.fake_stereo, file.stereo_balance
        );
    }
    if args.selected("phase") {
        println!(
            "  Phase: global {:?}, inverted {:?}, local {:?}",
            file.phase_correlation, file.phase_inverted, file.local_phase
        );
    }
    if args.selected("hf-stereo") {
        println!("  HF stereo: {:?}", file.high_frequency_stereo);
    }
    if args.selected("clipping") {
        println!(
            "  Clipping: {} events, {:.2} dBFS, {:.2} dBTP",
            file.clipping.clip_events, file.clipping.peak_dbfs, file.clipping.true_peak_dbtp
        );
    }
    if args.selected("loudness") {
        println!(
            "  Loudness: integrated {:?} LUFS, peaks {:?}, range {:?} LU",
            file.integrated_lufs, file.loudness_peaks, file.loudness_range_lu
        );
    }
    if args.selected("dynamics") {
        println!("  Dynamic range: {:?} dB", file.dr_db);
    }
    if args.selected("clicks") {
        println!(
            "  Suspected clicks: {:?}",
            file.discontinuities.as_ref().map(|d| d.clicks.count)
        );
    }
    if args.selected("dropouts") {
        println!(
            "  Suspected dropouts: {:?}",
            file.discontinuities.as_ref().map(|d| d.dropouts.count)
        );
    }
    if args.selected("dc-offset") {
        println!("  DC offset: {:?}", file.dc_offset);
    }
    if args.selected("flac-md5") {
        println!("  FLAC MD5: {}", flac_md5_label(file.flac_md5.as_ref()));
    }
    if args.selected("fingerprints") {
        println!(
            "  File MD5: {}; CRC32: {}",
            file.file_md5.as_deref().unwrap_or("N/A"),
            file.file_crc32.as_deref().unwrap_or("N/A")
        );
    }
}

#[cfg(test)]
#[path = "../tests/unit/display.rs"]
mod tests;
