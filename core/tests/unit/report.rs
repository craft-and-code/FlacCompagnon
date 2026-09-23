use super::*;
use crate::analysis::detections::Detections;
use crate::{ClippingInfo, FileAnalysis};

fn sample_file() -> FileAnalysis {
    FileAnalysis {
        path: "/music/a.flac".into(),
        file_name: "a.flac".into(),
        format: "FLAC".into(),
        codec: None,
        ext_mismatch: false,
        sample_rate: 44_100,
        channels: 2,
        declared_bits: Some(16),
        duration_secs: 183.4,
        size_bytes: 32_345_678,
        bitrate_kbps: Some(1412),
        modified_unix: Some(1_700_000_000),
        detections: Detections {
            upscaling: false,
            upsampling: false,
            transcoding: false,
            detail: "Clean.".into(),
            summary: "Clean".into(),
        },
        cutoff_hz: Some(21000.0),
        cutoff_ratio: Some(0.95),
        real_bit_depth: Some(16),
        bit_depth_evidence: None,
        lattice_score: None,
        fake_stereo: Some(false),
        phase_correlation: Some(0.8),
        phase_inverted: Some(false),
        stereo_balance: None,
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
        integrated_lufs: Some(-23.0),
        loudness_range_lu: Some(10.0),
        flac_md5: Some(FlacMd5Status::Match),
        file_md5: Some("0123456789abcdef0123456789abcdef".into()),
        file_crc32: Some("0a1b2c3d".into()),
        error: None,
    }
}

#[test]
fn balance_exports_preserve_direction_and_silence_without_infinite_numbers() {
    use crate::analysis::stereo::StereoBalance;
    for (balance, db, silent) in [
        (Some(StereoBalance::Measured { right_minus_left_db: -6.0206 }), "-6.02", ""),
        (Some(StereoBalance::Measured { right_minus_left_db: 6.0206 }), "6.02", ""),
        (Some(StereoBalance::LeftSilent), "", "left"),
        (Some(StereoBalance::RightSilent), "", "right"),
        (None, "", ""),
    ] {
        let mut file = sample_file();
        file.stereo_balance = balance;
        let report = FolderReport { root: "/music".into(), files: vec![file], has_flac: true };
        let csv = build_csv(&report);
        let lines: Vec<_> = csv.lines().collect();
        let header: Vec<_> = lines[0].split(',').collect();
        let row: Vec<_> = lines[1].split(',').collect();
        assert_eq!(header.len(), row.len());
        for (column, expected) in [("balance_right_minus_left_db", db), ("balance_silent_channel", silent)] {
            let index = header.iter().position(|&name| name == column).unwrap();
            assert_eq!(row[index], expected);
        }
        let json = build_json(&report).unwrap();
        let read = parse_json(&json).unwrap();
        assert_eq!(read.files[0].stereo_balance, balance);
    }
}

#[test]
fn older_json_without_balance_still_loads() {
    let report = FolderReport { root: "/music".into(), files: vec![sample_file()], has_flac: true };
    let mut json: serde_json::Value = serde_json::from_str(&build_json(&report).unwrap()).unwrap();
    json["report"]["files"][0].as_object_mut().unwrap().remove("stereo_balance");
    let read = parse_json(&serde_json::to_string(&json).unwrap()).unwrap();
    assert_eq!(read.files[0].stereo_balance, None);
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
    assert!(lines[0].contains(",md5,"));
    // `modified_unix` used to be the last column and this asserted on the
    // end of the header; the fingerprints now follow it, and which columns
    // sit at the end has its own test. Here it only has to be present.
    assert!(lines[0].contains(",modified_unix,"));
    assert!(lines[1].contains("a.flac"));
    assert!(lines[1].contains("ok"));
    // The size is exported as a raw byte count, not a formatted string,
    // so a spreadsheet can sum and sort it.
    assert!(lines[0].contains(",size_bytes,"));
    assert!(lines[1].contains(",32345678,"));
    // Header and row must stay in lockstep — an easy thing to break when
    // adding a column to one and forgetting the other.
    assert_eq!(
        lines[0].split(',').count(),
        lines[1].split(',').count(),
        "CSV header and row column counts must match"
    );
}

#[test]
fn csv_exports_phase_measurements_in_their_own_columns() {
    let mut file = sample_file();
    file.phase_correlation = Some(-1.0);
    file.phase_inverted = Some(true);
    let csv = build_csv(&FolderReport {
        root: "/music".into(),
        files: vec![file],
        has_flac: true,
    });
    let mut lines = csv.lines();
    let header: Vec<_> = lines.next().expect("header").split(',').collect();
    let row: Vec<_> = lines.next().expect("file row").split(',').collect();
    for (column, expected) in [("phase_correlation", "-1.000"), ("phase_inverted", "true")] {
        let at = header
            .iter()
            .position(|&name| name == column)
            .expect("phase column");
        assert_eq!(row[at], expected);
    }
}

#[test]
fn csv_exports_integrated_loudness_without_a_unit_suffix() {
    let csv = build_csv(&FolderReport {
        root: "/music".into(),
        files: vec![sample_file()],
        has_flac: true,
    });
    let mut lines = csv.lines();
    let header: Vec<_> = lines.next().expect("header").split(',').collect();
    let row: Vec<_> = lines.next().expect("file row").split(',').collect();
    let at = header.iter().position(|&name| name == "integrated_lufs").expect("LUFS column");
    assert_eq!(row[at], "-23.0");
    let at = header.iter().position(|&name| name == "loudness_range_lu").expect("LRA column");
    assert_eq!(row[at], "10.0");
}

#[test]
fn csv_keeps_estimated_depth_distinct_from_occupied_bits() {
    use crate::analysis::bitdepth::{BitDepthEvidence, BitDepthMethod};
    let mut file = sample_file();
    file.declared_bits = Some(24);
    file.real_bit_depth = Some(16);
    file.bit_depth_evidence = Some(BitDepthEvidence {
        stored_bits: 24,
        method: BitDepthMethod::NarrowGrid,
    });
    let csv = build_csv(&FolderReport {
        root: "/music".into(),
        files: vec![file],
        has_flac: true,
    });
    let mut lines = csv.lines();
    let header: Vec<_> = lines.next().expect("header").split(',').collect();
    let row: Vec<_> = lines.next().expect("file row").split(',').collect();
    for (column, expected) in [
        ("real_bit_depth", "16"),
        ("stored_bits", "24"),
        ("bit_depth_method", "NarrowGrid"),
    ] {
        let at = header
            .iter()
            .position(|&name| name == column)
            .expect("evidence column");
        assert_eq!(row[at], expected);
    }
}

/// The JSON is the archival format, so it must carry every field
/// regardless of what the table was showing. Written as an explicit list
/// rather than a count, so adding a field to `FileAnalysis` without
/// thinking about the report is a failing test rather than a silent gap.
#[test]
fn the_json_carries_every_analysis_field() {
    let report = FolderReport {
        root: "/music".into(),
        files: vec![sample_file()],
        has_flac: true,
    };
    let json = build_json(&report).expect("serialize");
    for key in [
        "path",
        "file_name",
        "format",
        "codec",
        "ext_mismatch",
        "sample_rate",
        "channels",
        "declared_bits",
        "duration_secs",
        "size_bytes",
        "bitrate_kbps",
        "modified_unix",
        "detections",
        "cutoff_hz",
        "cutoff_ratio",
        "real_bit_depth",
        "lattice_score",
        "fake_stereo",
        "phase_correlation",
        "phase_inverted",
        "stereo_balance",
        "badge",
        "clipping",
        "dr_db",
        "integrated_lufs",
        "loudness_range_lu",
        "flac_md5",
        "file_md5",
        "file_crc32",
        "error",
    ] {
        assert!(
            json.contains(&format!("\"{key}\"")),
            "the JSON report is missing `{key}`"
        );
    }
    // And the nested detection detail, which is the part a CSV cell folds
    // away and the JSON must not.
    assert!(json.contains("\"upscaling\""));
    assert!(json.contains("\"transcoding\""));
    assert!(json.contains("\"detail\""));
}

#[test]
fn older_json_without_phase_fields_still_loads() {
    let report = FolderReport {
        root: "/music".into(),
        files: vec![sample_file()],
        has_flac: true,
    };
    let mut json: serde_json::Value = serde_json::from_str(&build_json(&report).unwrap()).unwrap();
    let file = json["report"]["files"][0].as_object_mut().unwrap();
    file.remove("phase_correlation");
    file.remove("phase_inverted");
    let old = serde_json::to_string(&json).unwrap();
    let read = parse_json(&old).unwrap();
    assert!(read.files[0].phase_correlation.is_none());
    assert!(read.files[0].phase_inverted.is_none());
}

#[test]
fn older_json_without_integrated_lufs_still_loads() {
    let report = FolderReport {
        root: "/music".into(),
        files: vec![sample_file()],
        has_flac: true,
    };
    let mut json: serde_json::Value = serde_json::from_str(&build_json(&report).unwrap()).unwrap();
    json["report"]["files"][0].as_object_mut().unwrap().remove("integrated_lufs");
    let read = parse_json(&serde_json::to_string(&json).unwrap()).unwrap();
    assert_eq!(read.files[0].integrated_lufs, None);
}

#[test]
fn older_json_without_loudness_range_still_loads() {
    let report = FolderReport {
        root: "/music".into(),
        files: vec![sample_file()],
        has_flac: true,
    };
    let mut json: serde_json::Value = serde_json::from_str(&build_json(&report).unwrap()).unwrap();
    json["report"]["files"][0].as_object_mut().unwrap().remove("loudness_range_lu");
    let read = parse_json(&serde_json::to_string(&json).unwrap()).unwrap();
    assert_eq!(read.files[0].loudness_range_lu, None);
}

/// Files come out in the order they went in — the table's display order,
/// not any sort the backend might have applied while scanning.
#[test]
fn the_json_keeps_the_given_file_order() {
    let mut a = sample_file();
    a.file_name = "zzz.flac".into();
    a.path = "/music/zzz.flac".into();
    let mut b = sample_file();
    b.file_name = "aaa.flac".into();
    b.path = "/music/aaa.flac".into();
    let report = FolderReport {
        root: "/music".into(),
        files: vec![a, b],
        has_flac: true,
    };
    let json = build_json(&report).expect("serialize");
    let first = json.find("zzz.flac").expect("first file");
    let second = json.find("aaa.flac").expect("second file");
    assert!(first < second, "the report reordered the files");
}

/// The two fingerprint columns are appended at the end, and both hold
/// the file's own hash — not FLAC's `md5` column, which sits earlier and
/// means something else entirely.
#[test]
fn the_file_fingerprints_are_the_last_two_columns() {
    let report = FolderReport {
        root: "/music".into(),
        files: vec![sample_file()],
        has_flac: true,
    };
    let csv = build_csv(&report);
    let lines: Vec<&str> = csv.lines().collect();
    let cols: Vec<&str> = lines[0].trim_end().split(',').collect();
    assert_eq!(&cols[cols.len() - 2..], &["file_md5", "file_crc32"]);
    let row: Vec<&str> = lines[1].trim_end().split(',').collect();
    assert_eq!(row[cols.len() - 1], "0a1b2c3d");
    // The FLAC signature column is still its own thing, further left.
    let md5_at = cols.iter().position(|c| *c == "md5").expect("md5 column");
    assert!(md5_at < cols.len() - 2);
    assert_eq!(row[md5_at], "ok");
}

/// The three detections are one family and must read as one family.
///
/// `transcoding` briefly shipped as `yes`/`no` while its two siblings
/// were `true`/`false` — defensible on its own terms (a spreadsheet
/// coerces and localises `TRUE`), indefensible next to them, and worse
/// still for anything consuming the CSV alongside the JSON report, where
/// all three are JSON booleans. Whichever spelling wins, it wins for all
/// three at once.
#[test]
fn the_three_detection_columns_share_one_spelling() {
    let mut f = sample_file();
    f.detections.upscaling = true;
    f.detections.upsampling = false;
    f.detections.transcoding = true;
    let report = FolderReport {
        root: "/music".into(),
        files: vec![f],
        has_flac: true,
    };
    let csv = build_csv(&report);
    let lines: Vec<&str> = csv.lines().collect();
    let cols: Vec<&str> = lines[0].split(',').collect();
    let row: Vec<&str> = lines[1].split(',').collect();
    let at = |name: &str| {
        let i = cols.iter().position(|c| *c == name).expect(name);
        row[i]
    };
    assert_eq!(at("upscaling"), "true");
    assert_eq!(at("upsampling"), "false");
    assert_eq!(at("transcoding"), "true");
}

/// Locks in the maintainer's explicit request to move `codec` and
/// `bitrate_kbps` out of their original "appended at the end" spot —
/// see `build_csv`'s doc comment for why that's a deliberate exception.
#[test]
fn csv_places_codec_after_format_and_bitrate_before_sample_rate() {
    let report = FolderReport {
        root: "/music".into(),
        files: vec![sample_file()],
        has_flac: true,
    };
    let csv = build_csv(&report);
    let header: Vec<&str> = csv
        .lines()
        .next()
        .expect("header line")
        .split(',')
        .collect();
    assert_eq!(header.get(1), Some(&"format"));
    assert_eq!(header.get(2), Some(&"codec"));
    let bitrate_idx = header
        .iter()
        .position(|&h| h == "bitrate_kbps")
        .expect("bitrate_kbps");
    let rate_idx = header
        .iter()
        .position(|&h| h == "sample_rate")
        .expect("sample_rate");
    assert_eq!(
        bitrate_idx + 1,
        rate_idx,
        "bitrate_kbps must sit right before sample_rate"
    );
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
    assert_eq!(parsed.files[0].integrated_lufs, Some(-23.0));
    assert_eq!(parsed.files[0].loudness_range_lu, Some(10.0));
    assert_eq!(parsed.files[0].clipping.true_peak_dbtp, -0.7);
    assert_eq!(parsed.files[0].flac_md5, Some(FlacMd5Status::Match));
    assert_eq!(parsed.files[0].size_bytes, 32_345_678);
}

#[test]
fn unsupported_report_status_requires_reanalysis() {
    let mut file = sample_file();
    file.detections.summary = "Retired status".into();
    let report = FolderReport {
        root: "/music".into(),
        files: vec![file],
        has_flac: true,
    };
    let error = parse_json(&build_json(&report).expect("save fixture"))
        .expect_err("obsolete results must not be reintroduced");
    assert!(error.contains("Reanalyze"));
}

/// A report exported before `size_bytes` existed must still load — the
/// field is `serde(default)` precisely so an older `.json` dropped onto
/// the window doesn't fail to parse.
#[test]
fn json_without_size_bytes_still_loads() {
    let report = FolderReport {
        root: "/music".into(),
        files: vec![sample_file()],
        has_flac: true,
    };
    let json = build_json(&report).expect("serializes");
    // Strip the field the way an older export simply wouldn't have it.
    let mut doc: serde_json::Value = serde_json::from_str(&json).unwrap();
    doc["report"]["files"][0]
        .as_object_mut()
        .unwrap()
        .remove("size_bytes");
    let older = serde_json::to_string(&doc).unwrap();

    let parsed = parse_json(&older).expect("older reports must still parse");
    assert_eq!(parsed.files[0].size_bytes, 0);
    assert_eq!(parsed.files[0].file_name, "a.flac");
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
