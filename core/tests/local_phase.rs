//! Analytically defined stereo sections through decoding, assembly and exports.

use flaccompagnon_core::{analyze_file, report, FolderReport, ScanOptions};

#[test]
fn local_phase_survives_decoding_and_exports_time_and_band_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("local-phase.wav");
    let rate = 96_000;
    let mut writer = hound::WavWriter::create(
        &path,
        hound::WavSpec {
            channels: 2,
            sample_rate: rate,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for n in 0..rate * 3 {
        let phase = std::f64::consts::TAU * n as f64 / rate as f64;
        let bass = 0.2 * (100.0 * phase).sin();
        let treble = 0.1 * (9000.0 * phase).sin();
        let left = bass + treble;
        let right = if (rate..rate * 2).contains(&n) {
            -left
        } else {
            bass - treble
        };
        for sample in [left, right] {
            writer
                .write_sample((sample * 8_388_607.0).round() as i32)
                .unwrap();
        }
    }
    writer.finalize().unwrap();
    let file = analyze_file(&path, &ScanOptions::default());
    assert!(file.error.is_none(), "{:?}", file.error);
    assert!(file.phase_correlation.unwrap() > 0.0);
    assert_eq!(file.phase_inverted, Some(false));
    let local = file.local_phase.unwrap();
    let broadband = local.broadband.unwrap();
    assert!((broadband.minimum_correlation + 1.0).abs() < 1e-5);
    assert!(broadband.minimum_start_secs >= 1.0);
    assert!(broadband.minimum_start_secs + local.window_secs <= 2.0);
    assert!(broadband.opposed_fraction > 0.0 && broadband.opposed_fraction < 1.0);
    assert!((local.bands[3].summary.unwrap().correlation + 1.0).abs() < 1e-5);
    let report = FolderReport {
        root: dir.path().to_string_lossy().into_owned(),
        files: vec![file],
        has_flac: false,
    };
    let json = report::build_json(&report).unwrap();
    assert_eq!(
        report::parse_json(&json).unwrap().files[0].local_phase,
        Some(local)
    );
    let csv = report::build_csv(&report);
    let mut lines = csv.lines();
    let headers: Vec<_> = lines.next().unwrap().split(',').collect();
    let row: Vec<_> = lines.next().unwrap().split(',').collect();
    assert_eq!(headers.len(), row.len());
    for (column, expected) in [
        ("phase_window_s", local.window_secs.to_string()),
        ("phase_hop_s", local.hop_secs.to_string()),
        ("phase_analyzed_windows", local.analyzed_windows.to_string()),
        (
            "local_phase_minimum",
            broadband.minimum_correlation.to_string(),
        ),
        (
            "local_phase_minimum_start_s",
            broadband.minimum_start_secs.to_string(),
        ),
        (
            "local_phase_opposed_fraction",
            broadband.opposed_fraction.to_string(),
        ),
        (
            "phase_6000_20000_correlation",
            local.bands[3].summary.unwrap().correlation.to_string(),
        ),
        ("phase_200_2000_minimum", String::new()),
    ] {
        let index = headers.iter().position(|&h| h == column).unwrap();
        assert_eq!(row[index], expected, "{column}");
    }
    let mut old: serde_json::Value = serde_json::from_str(&json).unwrap();
    old["report"]["files"][0]
        .as_object_mut()
        .unwrap()
        .remove("local_phase");
    let restored = report::parse_json(&old.to_string()).unwrap();
    assert!(restored.files[0].local_phase.is_none());
    let absent_csv = report::build_csv(&restored);
    let absent: Vec<_> = absent_csv.lines().nth(1).unwrap().split(',').collect();
    assert_eq!(headers.len(), absent.len());
    for (header, value) in headers.iter().zip(absent) {
        if header.starts_with("local_phase_")
            || header.starts_with("phase_2")
            || header.starts_with("phase_6")
            || *header == "phase_window_s"
        {
            assert_eq!(value, "", "{header}");
        }
    }
}
