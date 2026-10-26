use super::*;

fn fixture(path: &Path) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 24,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for frame in 0..4096 {
        let sample =
            ((std::f64::consts::TAU * 3000.0 * frame as f64 / 48000.0).sin() * 2_097_152.0) as i32;
        writer.write_sample(sample).unwrap();
        writer.write_sample(-sample).unwrap();
    }
    writer.finalize().unwrap();
}

#[test]
fn native_spectrogram_needs_no_ffmpeg_and_preserves_opposed_channels() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("opposed.wav");
    fixture(&path);
    let before = std::fs::read(&path).unwrap();
    let options = AnalysisOptions {
        width: 80,
        height: 257,
        fft_size: Some(512),
        ..Default::default()
    };
    let (data, info) = analyze_file(&path, &options, None, &|| false).unwrap();
    assert_eq!(data.signal().frames, 4096);
    assert_eq!(info.bits, Some(24));
    assert!((data.level_db(40, 32).unwrap() + 12.041199826559248).abs() < 0.001);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn truncated_audio_and_malformed_files_are_rejected() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("truncated.wav");
    fixture(&path);
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..bytes.len() - 600]).unwrap();
    assert!(analyze_file(&path, &AnalysisOptions::default(), None, &|| false).is_err());
    std::fs::write(&path, b"not an audio file").unwrap();
    assert!(analyze_file(&path, &AnalysisOptions::default(), None, &|| false).is_err());
}

#[test]
fn cancellation_stops_before_opening_the_source() {
    assert!(analyze_file(
        Path::new("absent.wav"),
        &AnalysisOptions::default(),
        None,
        &|| true
    )
    .unwrap_err()
    .to_string()
    .contains("cancelled"));
}
