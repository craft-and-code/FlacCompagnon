//! Known level steps through PCM decoding and archival loudness-maximum exports.

use flaccompagnon_core::{analyze_file, report, FolderReport, ScanOptions};

#[test]
fn loudness_peaks_keep_their_values_and_real_window_locations_through_reports() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("loudness-peaks.wav");
    let rate = 96_000;
    let frames = rate * 7 / 2;
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
    for n in 0..frames {
        let level = if n < rate * 3 { -35.0 } else { -20.0 };
        let sample = (10_f64.powf(level / 20.0)
            * (std::f64::consts::TAU * 1000.0 * n as f64 / rate as f64).sin()
            * 8_388_607.0)
            .round() as i32;
        writer.write_sample(sample).unwrap();
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();
    let file = analyze_file(&path, &ScanOptions::default());
    assert!(file.error.is_none(), "{:?}", file.error);
    let peaks = file.loudness_peaks.unwrap();
    let m = peaks.momentary.unwrap();
    let s = peaks.short_term.unwrap();
    assert!((m.lufs + 20.0).abs() <= 0.1);
    // A 3 s window at EOF contains 2.5 s at -35 and 0.5 s at -20 LUFS.
    let expected = 10.0 * ((2.5 * 10_f64.powf(-3.5) + 0.5 * 0.01) / 3.0).log10();
    assert!(
        (s.lufs as f64 - expected).abs() <= 0.1,
        "{} vs {expected}",
        s.lufs
    );
    assert!(m.start_secs >= 3.0 && m.start_secs + 0.4 <= 3.5 + 1e-9);
    assert!((s.start_secs - 0.5).abs() < 0.005);
    let folder = FolderReport {
        root: dir.path().to_string_lossy().into_owned(),
        files: vec![file],
        has_flac: false,
    };
    let json = report::build_json(&folder).unwrap();
    assert_eq!(
        report::parse_json(&json).unwrap().files[0].loudness_peaks,
        Some(peaks)
    );
    let csv = report::build_csv(&folder);
    let mut lines = csv.lines();
    let headers: Vec<_> = lines.next().unwrap().split(',').collect();
    let row: Vec<_> = lines.next().unwrap().split(',').collect();
    assert_eq!(headers.len(), row.len());
    let at = headers
        .iter()
        .position(|&h| h == "integrated_lufs")
        .unwrap();
    let expected_columns = [
        "max_momentary_lufs",
        "momentary_max_start_s",
        "max_short_term_lufs",
        "short_term_max_start_s",
    ];
    assert_eq!(&headers[at + 1..at + 5], &expected_columns);
    assert_eq!(
        &row[at + 1..at + 5],
        &[
            m.lufs.to_string(),
            m.start_secs.to_string(),
            s.lufs.to_string(),
            s.start_secs.to_string()
        ]
    );
    let mut old: serde_json::Value = serde_json::from_str(&json).unwrap();
    old["report"]["files"][0]
        .as_object_mut()
        .unwrap()
        .remove("loudness_peaks");
    let restored = report::parse_json(&old.to_string()).unwrap();
    assert!(restored.files[0].loudness_peaks.is_none());
    let csv = report::build_csv(&restored);
    let row: Vec<_> = csv.lines().nth(1).unwrap().split(',').collect();
    assert_eq!(&row[at + 1..at + 5], &["", "", "", ""]);
}
