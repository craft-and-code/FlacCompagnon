use std::process::Command;

use flaccompagnon_core::report;

#[test]
fn cli_json_round_trips_as_a_desktop_report() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let audio = dir.path().join("tone.wav");
    let output = dir.path().join("report.json");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&audio, spec).expect("fixture WAV");
    for _ in 0..4_410 {
        writer.write_sample(0i16).expect("sample");
    }
    writer.finalize().expect("complete WAV");

    let result = Command::new(env!("CARGO_BIN_EXE_flaccompagnon"))
        .arg("--analysis")
        .arg("loudness")
        .arg("--json")
        .arg(&output)
        .arg(&audio)
        .output()
        .expect("run CLI");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(output).expect("report was written");
    let parsed = report::parse_json(&text).expect("same re-importable JSON as desktop");
    assert_eq!(parsed.files.len(), 1);
    assert_eq!(parsed.files[0].path, audio.to_string_lossy());
}
