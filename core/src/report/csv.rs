//! Spreadsheet-safe CSV serialization of analysis reports.

use super::phase;
use crate::{FlacMd5Status, FolderReport};

/// Default file name suggested when saving a report.
pub const CSV_FILE_NAME: &str = "FlacCompagnon.csv";

/// Build the CSV text for a folder report.
///
/// Column order mirrors the results table left-to-right (`ResultsTable.tsx`'s
/// `headers`), not the order fields were originally added to this struct —
/// so a column's position here means the same thing it means on screen.
/// Fields the table folds into a single cell (Detections: `status` +
/// `upscaling`/`upsampling`/`transcoding`/`lattice_score`; Clipping: `clipped` +
/// `clip_events`/`peak_dbfs`) are grouped together at that cell's position
/// rather than split across the row. No column is dropped — every field the
/// old order exported is still here, just reordered.
///
/// `file_md5` and `file_crc32` are the exception to that mirroring: they are
/// appended at the end because their columns are hidden by default in the
/// table, so there is no on-screen position to mirror.
///
/// `codec` and `bitrate_kbps` are deliberately slotted mid-row — right after
/// `format` and right before `sample_rate` respectively — rather than at the
/// end, on the maintainer's explicit request, even though the table's own
/// column order is otherwise just a display preference (`useColumnPrefs.ts`)
/// this file doesn't follow. This *is* a breaking change for any script or
/// spreadsheet already reading the CSV by position: those two columns moved.
/// `modified_unix` stays appended at the end — nobody asked to move that one,
/// and there's no natural mid-row spot for it the way there is for the other
/// two.
pub fn build_csv(report: &FolderReport) -> String {
    let mut out = String::new();
    out.push_str(
        "file,format,codec,badge,bitrate_kbps,sample_rate,declared_bits,real_bit_depth,\
         duration_s,size_bytes,status,upscaling,upsampling,transcoding,lattice_score,cutoff_hz,\
         cutoff_ratio,channels,fake_stereo,phase_correlation,phase_inverted,clipped,clip_events,peak_dbfs,true_peak_dbtp,integrated_lufs,max_momentary_lufs,momentary_max_start_s,max_short_term_lufs,short_term_max_start_s,loudness_range_lu,dr_db,\
         md5,bit_depth_method,stored_bits,balance_right_minus_left_db,balance_silent_channel,hf_side_to_mid_db,hf_reference_side_to_mid_db,hf_narrowed_block_fraction,hf_stereo_narrowed,suspected_clicks,suspected_dropouts,click_locations,dropout_locations,modified_unix,dc_offset_max_abs,dc_offset_channel_means,",
    );
    out.push_str(&phase::headers().join(","));
    out.push_str(",file_md5,file_crc32\n");
    for f in &report.files {
        let md5 = f
            .flac_md5
            .as_ref()
            .map(|m| match m {
                FlacMd5Status::NoSignature => "none",
                FlacMd5Status::Present => "present",
                FlacMd5Status::Match => "ok",
                FlacMd5Status::Mismatch => "mismatch",
                FlacMd5Status::Error(_) => "error",
            })
            .unwrap_or("");
        use crate::analysis::stereo::StereoBalance;
        let (balance_db, silent_channel) = match f.stereo_balance {
            Some(StereoBalance::Measured {
                right_minus_left_db,
            }) => (format!("{right_minus_left_db:.2}"), ""),
            Some(StereoBalance::LeftSilent) => (String::new(), "left"),
            Some(StereoBalance::RightSilent) => (String::new(), "right"),
            None => (String::new(), ""),
        };
        let (
            hf_side_to_mid_db,
            hf_reference_side_to_mid_db,
            hf_narrowed_block_fraction,
            hf_stereo_narrowed,
        ) = match f.high_frequency_stereo {
            Some(measurement) => (
                format!("{:.2}", measurement.side_to_mid_db),
                format!("{:.2}", measurement.reference_side_to_mid_db),
                format!("{:.3}", measurement.narrowed_block_fraction),
                measurement.narrowed.to_string(),
            ),
            None => (String::new(), String::new(), String::new(), String::new()),
        };
        let mut row = vec![
            csv_text(&f.file_name),
            csv_text(&f.format),
            csv_text(f.codec.as_deref().unwrap_or_default()),
            csv_text(f.badge.as_deref().unwrap_or_default()),
            opt(f.bitrate_kbps),
            f.sample_rate.to_string(),
            opt(f.declared_bits),
            opt(f.real_bit_depth),
            format!("{:.3}", f.duration_secs),
            f.size_bytes.to_string(),
            csv_text(&f.detections.summary),
            f.detections.upscaling.to_string(),
            f.detections.upsampling.to_string(),
            f.detections.transcoding.to_string(),
            f.lattice_score
                .map(|v| format!("{v:.4}"))
                .unwrap_or_default(),
            f.cutoff_hz.map(|v| format!("{v:.0}")).unwrap_or_default(),
            f.cutoff_ratio
                .map(|v| format!("{v:.3}"))
                .unwrap_or_default(),
            f.channels.to_string(),
            opt_bool(f.fake_stereo),
            f.phase_correlation
                .map(|v| format!("{v:.3}"))
                .unwrap_or_default(),
            opt_bool(f.phase_inverted),
            f.clipping.clipped.to_string(),
            f.clipping.clip_events.to_string(),
            format!("{:.2}", f.clipping.peak_dbfs),
            format!("{:.2}", f.clipping.true_peak_dbtp),
            f.integrated_lufs
                .map(|v| format!("{v:.1}"))
                .unwrap_or_default(),
            opt(f.loudness_peaks.and_then(|p| p.momentary).map(|p| p.lufs)),
            opt(f
                .loudness_peaks
                .and_then(|p| p.momentary)
                .map(|p| p.start_secs)),
            opt(f.loudness_peaks.and_then(|p| p.short_term).map(|p| p.lufs)),
            opt(f
                .loudness_peaks
                .and_then(|p| p.short_term)
                .map(|p| p.start_secs)),
            f.loudness_range_lu
                .map(|v| format!("{v:.1}"))
                .unwrap_or_default(),
            f.dr_db.map(|v| format!("{v:.1}")).unwrap_or_default(),
            md5.to_string(),
            f.bit_depth_evidence
                .map(|e| match e.method {
                    crate::analysis::bitdepth::BitDepthMethod::Stored => "Stored",
                    crate::analysis::bitdepth::BitDepthMethod::NarrowGrid => "NarrowGrid",
                })
                .unwrap_or("")
                .to_string(),
            opt(f.bit_depth_evidence.map(|e| e.stored_bits)),
            balance_db,
            silent_channel.to_string(),
            hf_side_to_mid_db,
            hf_reference_side_to_mid_db,
            hf_narrowed_block_fraction,
            hf_stereo_narrowed,
            opt(f.discontinuities.as_ref().map(|d| d.clicks.count)),
            opt(f.discontinuities.as_ref().map(|d| d.dropouts.count)),
            discontinuity_locations(f.discontinuities.as_ref().map(|d| &d.clicks)),
            discontinuity_locations(f.discontinuities.as_ref().map(|d| &d.dropouts)),
            opt(f.modified_unix),
            f.dc_offset
                .as_ref()
                .map(|dc| dc.max_abs.to_string())
                .unwrap_or_default(),
            f.dc_offset
                .as_ref()
                .map(|dc| {
                    dc.channel_means
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(";")
                })
                .unwrap_or_default(),
        ];
        row.extend(phase::values(f.local_phase.as_ref()));
        row.push(csv_text(f.file_md5.as_deref().unwrap_or_default()));
        row.push(csv_text(f.file_crc32.as_deref().unwrap_or_default()));
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

fn discontinuity_locations(
    summary: Option<&crate::analysis::discontinuities::EventSummary>,
) -> String {
    summary
        .map(|s| {
            s.events
                .iter()
                .map(|event| {
                    format!(
                        "ch{}@{:.6}s/{:.6}s",
                        event.channel, event.start_secs, event.duration_secs
                    )
                })
                .collect::<Vec<_>>()
                .join(";")
        })
        .unwrap_or_default()
}

// Imported reports and file names are untrusted text. Spreadsheet programs
// interpret leading operators even inside quoted CSV fields; an apostrophe
// keeps those cells textual while numeric measurement columns remain numeric.
fn csv_text(s: &str) -> String {
    let dangerous = matches!(s.chars().next(), Some('\t' | '\r' | '\n'))
        || matches!(s.trim_start().chars().next(), Some('=' | '+' | '-' | '@'));
    if dangerous {
        csv_escape(&format!("'{s}"))
    } else {
        csv_escape(s)
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

fn opt_bool(v: Option<bool>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

#[cfg(test)]
#[path = "../../tests/unit/report/csv.rs"]
mod tests;
