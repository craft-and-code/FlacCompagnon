//! Report generation: a spreadsheet-friendly CSV, and a re-importable JSON that
//! round-trips the full [`FolderReport`] — every field the app computed,
//! nested detections included — so a saved analysis can be dropped back onto
//! the window later and rendered without re-decoding a single audio file.

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{FlacMd5Status, FolderReport};

/// Default file name suggested when saving a report.
pub const CSV_FILE_NAME: &str = "FlacCompagnon.csv";
/// Same stem, JSON sibling — `save` always writes both from one dialog pick.
pub const JSON_FILE_NAME: &str = "FlacCompagnon.json";

/// Marker written into every JSON report so a dropped file can be recognized
/// (and rejected with a clear message) before attempting to parse it as one.
const JSON_FORMAT_MARKER: &str = "flaccompagnon-report";
/// Bumped if the JSON shape ever changes incompatibly.
const JSON_FORMAT_VERSION: u32 = 1;

/// On-disk shape of the JSON report: the marker/version let a dropped file be
/// identified and versioned independently of [`FolderReport`]'s own shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonReport {
    format: String,
    version: u32,
    report: FolderReport,
}

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
         cutoff_ratio,channels,fake_stereo,phase_correlation,phase_inverted,clipped,clip_events,peak_dbfs,true_peak_dbtp,integrated_lufs,loudness_range_lu,dr_db,\
         md5,bit_depth_method,stored_bits,balance_right_minus_left_db,balance_silent_channel,hf_side_to_mid_db,hf_reference_side_to_mid_db,hf_narrowed_block_fraction,hf_stereo_narrowed,suspected_clicks,suspected_dropouts,click_locations,dropout_locations,modified_unix,file_md5,file_crc32\n",
    );
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
        let row = [
            csv_escape(&f.file_name),
            f.format.clone(),
            f.codec.clone().unwrap_or_default(),
            f.badge.clone().unwrap_or_default(),
            opt(f.bitrate_kbps),
            f.sample_rate.to_string(),
            opt(f.declared_bits),
            opt(f.real_bit_depth),
            format!("{:.3}", f.duration_secs),
            f.size_bytes.to_string(),
            f.detections.summary.clone(),
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
            f.file_md5.clone().unwrap_or_default(),
            f.file_crc32.clone().unwrap_or_default(),
        ];
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

/// Write the CSV report to `dest`.
pub fn write_csv(dest: &Path, report: &FolderReport) -> std::io::Result<()> {
    let mut file = std::fs::File::create(dest)?;
    file.write_all(build_csv(report).as_bytes())
}

/// Build the JSON text for a folder report (pretty-printed, wrapped with a
/// format marker and version — see `JsonReport`).
///
/// # Complete, always
///
/// Unlike the CSV, this is a full serialization of every [`FileAnalysis`]
/// field, in the order the files are given (which is the table's display
/// order — the frontend hands them over already sorted). **What the user has
/// chosen to show or hide in the table has no effect here**: a hidden column's
/// value is still in the JSON, because this file is what a saved analysis is
/// reloaded from, and a reload that lost whatever happened to be hidden at
/// save time would be a data-loss bug rather than a formatting choice.
pub fn build_json(report: &FolderReport) -> serde_json::Result<String> {
    let wrapped = JsonReport {
        format: JSON_FORMAT_MARKER.to_string(),
        version: JSON_FORMAT_VERSION,
        report: report.clone(),
    };
    serde_json::to_string_pretty(&wrapped)
}

/// Write the JSON report to `dest`.
pub fn write_json(dest: &Path, report: &FolderReport) -> std::io::Result<()> {
    let text =
        build_json(report).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut file = std::fs::File::create(dest)?;
    file.write_all(text.as_bytes())
}

/// Parse a previously-saved JSON report back into a [`FolderReport`], so it
/// can be rendered without re-analyzing any audio. Rejects JSON that doesn't
/// carry FlacCompagnon's format marker, with a message meant for end users
/// (someone dropped an unrelated `.json` file).
///
/// ```
/// use flaccompagnon_core::{report, FolderReport};
///
/// let empty = FolderReport { root: "/music".into(), files: vec![], has_flac: false };
///
/// // build_json / parse_json round-trip the whole report.
/// let json = report::build_json(&empty).unwrap();
/// let back = report::parse_json(&json).unwrap();
/// assert_eq!(back.root, "/music");
///
/// // Unrelated JSON is refused with a message meant for the user.
/// assert!(report::parse_json(r#"{"hello":"world"}"#).is_err());
/// ```
pub fn parse_json(text: &str) -> Result<FolderReport, String> {
    let wrapped: JsonReport = serde_json::from_str(text)
        .map_err(|e| format!("This doesn't look like a FlacCompagnon JSON report ({e})."))?;
    if wrapped.format != JSON_FORMAT_MARKER {
        return Err("This JSON file wasn't exported by FlacCompagnon.".to_string());
    }
    if wrapped.version > JSON_FORMAT_VERSION {
        return Err(
            "This report was saved by a newer version of FlacCompagnon — please update the app."
                .to_string(),
        );
    }
    if wrapped.report.files.iter().any(|file| {
        !matches!(
            file.detections.summary.as_str(),
            "Clean" | "Flagged" | "Not analyzed"
        )
    }) {
        return Err(
            "This report uses an unsupported detection status. Reanalyze the audio files.".into(),
        );
    }
    Ok(wrapped.report)
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
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
#[path = "../tests/unit/report.rs"]
mod tests;
