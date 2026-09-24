//! Known M/S signals through WAV decoding, analysis assembly and report export.

use flaccompagnon_core::{analyze_file, report, FolderReport, ScanOptions};

#[test]
fn hf_stereo_survives_decoding_and_report_export() {
    let dir = tempfile::tempdir().unwrap();
    for (name, reference_side, high_side, narrowed) in [
        ("narrow", 0.1, 0.001, true),
        ("wide", 0.1, 0.1, false),
        ("mono", 0.0, 0.0, false),
    ] {
        let path = dir.path().join(format!("{name}.wav"));
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
        for n in 0..rate * 2 {
            let phase = std::f64::consts::TAU * n as f64 / rate as f64;
            let mid = 0.1 * (3_000.0 * phase).sin() + 0.1 * (9_000.0 * phase).sin();
            let side =
                reference_side * (3_000.0 * phase).cos() + high_side * (9_000.0 * phase).cos();
            for sample in [mid + side, mid - side] {
                writer
                    .write_sample((sample * 8_388_607.0).round() as i32)
                    .unwrap();
            }
        }
        writer.finalize().unwrap();
        let result = analyze_file(&path, &ScanOptions::default());
        assert!(result.error.is_none(), "{:?}", result.error);
        let measurement = result
            .high_frequency_stereo
            .expect("two active stereo seconds");
        assert_eq!(measurement.narrowed, narrowed, "{name}: {measurement:?}");
        if name == "wide" {
            assert!(measurement.side_to_mid_db.abs() < 0.1);
        }
        let report = FolderReport {
            root: dir.path().to_string_lossy().into_owned(),
            files: vec![result],
            has_flac: false,
        };
        let json = report::build_json(&report).unwrap();
        assert_eq!(
            report::parse_json(&json).unwrap().files[0].high_frequency_stereo,
            Some(measurement)
        );
        let csv = report::build_csv(&report);
        let mut lines = csv.lines();
        let headers: Vec<_> = lines.next().unwrap().split(',').collect();
        let row: Vec<_> = lines.next().unwrap().split(',').collect();
        assert_eq!(headers.len(), row.len());
        let index = headers
            .iter()
            .position(|&h| h == "hf_stereo_narrowed")
            .unwrap();
        assert_eq!(row[index], narrowed.to_string());
    }
}
