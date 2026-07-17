//! CSV report generation. (The earlier plain-text `.log` was dropped — the CSV
//! is self-sufficient and re-importable.)

use std::io::Write;
use std::path::Path;

use crate::{FlacMd5Status, FolderReport, TranscodeState};

/// Default file name suggested when saving a CSV report.
pub const CSV_FILE_NAME: &str = "FlacCompagnon.csv";

/// Build the CSV text for a folder report.
pub fn build_csv(report: &FolderReport) -> String {
    let mut out = String::new();
    out.push_str(
        "file,format,badge,sample_rate,channels,declared_bits,real_bit_depth,duration_s,\
         status,upscaling,upsampling,transcoding,aac_grid,cutoff_hz,cutoff_ratio,fake_stereo,\
         clipped,clip_events,peak_dbfs,dr_db,md5\n",
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
            "{},{},{},{},{},{},{},{:.3},{},{},{},{},{},{},{},{},{},{},{:.2},{},{}\n",
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
}
