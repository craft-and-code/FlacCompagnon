//! Report generation: a spreadsheet-friendly CSV, and a re-importable JSON that
//! round-trips the full [`FolderReport`] — every field the app computed,
//! nested detections included — so a saved analysis can be dropped back onto
//! the window later and rendered without re-decoding a single audio file.

use serde::{Deserialize, Serialize};

use crate::FolderReport;

mod csv;
mod file;
mod phase;

pub use csv::{build_csv, CSV_FILE_NAME};
pub use file::{
    open_regular_file, read_json, validate_destination, write_atomic_bytes, write_csv, write_json,
    MAX_JSON_BYTES,
};
/// Same stem, JSON sibling — `save` always writes both from one dialog pick.
pub const JSON_FILE_NAME: &str = "FlacCompagnon.json";

/// Marker written into every JSON report so a dropped file can be recognized
/// (and rejected with a clear message) before attempting to parse it as one.
const JSON_FORMAT_MARKER: &str = "flaccompagnon-report";
/// Bumped if the JSON shape ever changes incompatibly.
const JSON_FORMAT_VERSION: u32 = 1;

// A report is exchanged between app builds, the CLI and other Rust software.
// Their optimized floating-point code may differ by one final IEEE-754 bit
// while yielding the same measurement, which used to make two analyses of the
// same file look different in a JSON diff. Nine decimal places are far beyond
// the display precision and preserve sub-microsecond event positions.
const JSON_FLOAT_SCALE: f64 = 1_000_000_000.0;

/// On-disk shape of the JSON report: the marker/version let a dropped file be
/// identified and versioned independently of [`FolderReport`]'s own shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonReport {
    format: String,
    version: u32,
    report: FolderReport,
}

/// Build the JSON text for a folder report (pretty-printed, wrapped with a
/// format marker and version — see `JsonReport`).
///
/// # Complete, always
///
/// Unlike the CSV, this is a full serialization of every [`crate::FileAnalysis`]
/// field, in the order the files are given (which is the table's display
/// order — the frontend hands them over already sorted). **What the user has
/// chosen to show or hide in the table has no effect here**: a hidden column's
/// value is still in the JSON, because this file is what a saved analysis is
/// reloaded from, and a reload that lost whatever happened to be hidden at
/// save time would be a data-loss bug rather than a formatting choice.
pub fn build_json(report: &FolderReport) -> serde_json::Result<String> {
    let mut report = report.clone();
    stabilize_report_floats(&mut report);
    let wrapped = JsonReport {
        format: JSON_FORMAT_MARKER.to_string(),
        version: JSON_FORMAT_VERSION,
        report,
    };
    serde_json::to_string_pretty(&wrapped)
}

/// Round the `f64` fields that cross the persisted report boundary.
///
/// The analysis keeps its full in-memory precision, while JSON becomes stable
/// across debug and release builds and across clients that use the same core
/// version. This works on the typed fields rather than generic JSON numbers:
/// serializing an `f32` through `serde_json::Value` would expand it to a longer
/// `f64` spelling despite its value already being stable.
fn stabilize_report_floats(report: &mut FolderReport) {
    for file in &mut report.files {
        file.duration_secs = stable_float(file.duration_secs);
        file.cutoff_hz = file.cutoff_hz.map(stable_float);
        file.cutoff_ratio = file.cutoff_ratio.map(stable_float);

        if let Some(offset) = &mut file.dc_offset {
            offset.channel_means.iter_mut().for_each(|mean| {
                *mean = stable_float(*mean);
            });
            offset.max_abs = stable_float(offset.max_abs);
        }

        if let Some(phase) = &mut file.local_phase {
            phase.window_secs = stable_float(phase.window_secs);
            phase.hop_secs = stable_float(phase.hop_secs);
            stabilize_phase_summary(phase.broadband.as_mut());
            for band in &mut phase.bands {
                band.low_hz = stable_float(band.low_hz);
                band.high_hz = stable_float(band.high_hz);
                stabilize_phase_summary(band.summary.as_mut());
            }
        }

        if let Some(peaks) = &mut file.loudness_peaks {
            if let Some(peak) = &mut peaks.momentary {
                peak.start_secs = stable_float(peak.start_secs);
            }
            if let Some(peak) = &mut peaks.short_term {
                peak.start_secs = stable_float(peak.start_secs);
            }
        }

        if let Some(discontinuities) = &mut file.discontinuities {
            for event in discontinuities
                .clicks
                .events
                .iter_mut()
                .chain(discontinuities.dropouts.events.iter_mut())
            {
                event.start_secs = stable_float(event.start_secs);
                event.duration_secs = stable_float(event.duration_secs);
            }
        }
    }
}

fn stabilize_phase_summary(summary: Option<&mut crate::analysis::local_phase::PhaseSummary>) {
    if let Some(summary) = summary {
        summary.minimum_start_secs = stable_float(summary.minimum_start_secs);
        summary.opposed_fraction = stable_float(summary.opposed_fraction);
    }
}

fn stable_float(value: f64) -> f64 {
    let rounded = (value * JSON_FLOAT_SCALE).round() / JSON_FLOAT_SCALE;
    if rounded.is_finite() {
        rounded
    } else {
        value
    }
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

#[cfg(test)]
#[path = "../../tests/unit/report.rs"]
mod tests;
