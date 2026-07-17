//! Report generation: a spreadsheet-friendly CSV, and a re-importable JSON that
//! round-trips the full [`FolderReport`] — every field the app computed,
//! nested detections included — so a saved analysis can be dropped back onto
//! the window later and rendered without re-decoding a single audio file.

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{FlacMd5Status, FolderReport, TranscodeState};

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
pub fn build_csv(report: &FolderReport) -> String {
    let mut out = String::new();
    out.push_str(
        "file,format,badge,sample_rate,channels,declared_bits,real_bit_depth,duration_s,\
         status,upscaling,upsampling,transcoding,aac_grid,cutoff_hz,cutoff_ratio,fake_stereo,\
         clipped,clip_events,peak_dbfs,true_peak_dbtp,dr_db,md5\n",
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
        let transcoding = match f.detections.transcoding {
            TranscodeState::None => "none",
            TranscodeState::Suspected => "suspected",
            TranscodeState::Detected => "detected",
        };
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{:.3},{},{},{},{},{},{},{},{},{},{},{:.2},{:.2},{},{}\n",
            csv_escape(&f.file_name),
            f.format,
            f.badge.clone().unwrap_or_default(),
            f.sample_rate,
            f.channels,
            opt(f.declared_bits),
            opt(f.real_bit_depth),
            f.duration_secs,
            f.detections.summary,
            f.detections.upscaling,
            f.detections.upsampling,
            transcoding,
            f.requant_rate.map(|v| format!("{v:.3}")).unwrap_or_default(),
            f.cutoff_hz.map(|v| format!("{v:.0}")).unwrap_or_default(),
            f.cutoff_ratio.map(|v| format!("{v:.3}")).unwrap_or_default(),
            opt_bool(f.fake_stereo),
            f.clipping.clipped,
            f.clipping.clip_events,
            f.clipping.peak_dbfs,
            f.clipping.true_peak_dbtp,
            f.dr_db.map(|v| format!("{v:.1}")).unwrap_or_default(),
            md5,
        ));
    }
    out
}

/// Write the CSV report to `dest`.
pub fn write_csv(dest: &Path, report: &FolderReport) -> std::io::Result<()> {
    let mut file = std::fs::File::create(dest)?;
    file.write_all(build_csv(report).as_bytes())
}

/// Build the JSON text for a folder report (pretty-printed, wrapped with a
/// format marker and version — see [`JsonReport`]).
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
    let text = build_json(report)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut file = std::fs::File::create(dest)?;
    file.write_all(text.as_bytes())
}

/// Parse a previously-saved JSON report back into a [`FolderReport`], so it
/// can be rendered without re-analyzing any audio. Rejects JSON that doesn't
/// carry FlacCompagnon's format marker, with a message meant for end users
/// (someone dropped an unrelated `.json` file).
pub fn parse_json(text: &str) -> Result<FolderReport, String> {
    let wrapped: JsonReport = serde_json::from_str(text).map_err(|e| {
        format!("This doesn't look like a FlacCompagnon JSON report ({e}).")
    })?;
    if wrapped.format != JSON_FORMAT_MARKER {
        return Err("This JSON file wasn't exported by FlacCompagnon.".to_string());
    }
    if wrapped.version > JSON_FORMAT_VERSION {
        return Err(
            "This report was saved by a newer version of FlacCompagnon — please update the app."
                .to_string(),
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
mod tests {
    use super::*;
    use crate::detections::{Detections, TranscodeState};
    use crate::{ClippingInfo, FileAnalysis};

    fn sample_file() -> FileAnalysis {
        FileAnalysis {
            path: "/music/a.flac".into(),
            file_name: "a.flac".into(),
            format: "FLAC".into(),
            ext_mismatch: false,
            sample_rate: 44_100,
            channels: 2,
            declared_bits: Some(16),
            duration_secs: 183.4,
            detections: Detections {
                upscaling: false,
                upsampling: false,
                transcoding: TranscodeState::None,
                detail: "Clean.".into(),
                summary: "Clean".into(),
            },
            cutoff_hz: Some(21000.0),
            cutoff_ratio: Some(0.95),
            real_bit_depth: Some(16),
            requant_rate: None,
            fake_stereo: Some(false),
            badge: None,
            clipping: ClippingInfo {
                clipped_samples: 0,
                clip_events: 0,
                peak: 0.9,
                peak_dbfs: -0.9,
                true_peak: 0.92,
                true_peak_dbtp: -0.7,
                clipped: false,
            },
            dr_db: Some(12.3),
            flac_md5: Some(FlacMd5Status::Match),
            error: None,
        }
    }

    #[test]
    fn csv_has_header_and_row() {
        let report = FolderReport {
            root: "/music".into(),
            files: vec![sample_file()],
            has_flac: true,
        };
        let csv = build_csv(&report);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("file,format"));
        assert!(lines[0].trim_end().ends_with(",md5"));
        assert!(lines[1].contains("a.flac"));
        assert!(lines[1].contains("ok"));
    }

    #[test]
    fn json_round_trips_the_full_report() {
        let report = FolderReport {
            root: "/music".into(),
            files: vec![sample_file()],
            has_flac: true,
        };
        let json = build_json(&report).expect("serializes");
        assert!(json.contains("flaccompagnon-report"));
        let parsed = parse_json(&json).expect("parses back");
        assert_eq!(parsed.root, report.root);
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].file_name, "a.flac");
        assert_eq!(parsed.files[0].dr_db, Some(12.3));
        assert_eq!(parsed.files[0].clipping.true_peak_dbtp, -0.7);
        assert_eq!(parsed.files[0].flac_md5, Some(FlacMd5Status::Match));
    }

    #[test]
    fn json_rejects_unrelated_files() {
        // Not FlacCompagnon's shape at all — fails to deserialize.
        let err = parse_json(r#"{"hello": "world"}"#).unwrap_err();
        assert!(err.contains("doesn't look like"));

        // Right shape, wrong marker — deserializes fine, rejected on the check.
        let err2 = parse_json(
            r#"{"format": "something-else", "version": 1, "report": {"root": "", "files": [], "has_flac": false}}"#,
        )
        .unwrap_err();
        assert!(err2.contains("wasn't exported by FlacCompagnon"));
    }
}
