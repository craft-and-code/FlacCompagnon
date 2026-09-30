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
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("Loudness:"),
        "explicit analysis selection enables terminal results"
    );
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("[1/1]"),
        "the terminal shows analysis progress"
    );
    let text = std::fs::read_to_string(output).expect("report was written");
    let parsed = report::parse_json(&text).expect("same re-importable JSON as desktop");
    assert_eq!(parsed.files.len(), 1);
    assert_eq!(parsed.files[0].path, audio.to_string_lossy());
}

fn silent_audio(path: &std::path::Path) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for _ in 0..4410 {
        writer.write_sample(0i16).unwrap();
    }
    writer.finalize().unwrap();
}

fn run(options: &[&str], target: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_flaccompagnon"))
        .args(options)
        .arg(target)
        .output()
        .unwrap()
}

#[test]
fn lowercase_v_shows_version() {
    let result = Command::new(env!("CARGO_BIN_EXE_flaccompagnon"))
        .arg("-v")
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).starts_with("flaccompagnon "));
}

#[test]
fn saved_results_are_reused_until_force_or_audio_changes() {
    let dir = tempfile::tempdir().unwrap();
    let album = dir.path().join("Album");
    std::fs::create_dir(&album).unwrap();
    let audio = album.join("track.wav");
    silent_audio(&audio);
    let first = run(&["--json-layout", "album"], &album);
    assert!(first.status.success());
    assert!(first.stdout.is_empty());
    let report_path = album.join("Album.json");
    let saved = std::fs::read(&report_path).unwrap();
    let second = run(&["--json-layout", "album"], &album);
    assert!(second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains("Reusing"));
    assert!(!String::from_utf8_lossy(&second.stderr).contains("Analyzing"));
    assert_eq!(saved, std::fs::read(&report_path).unwrap());
    let forced = run(
        &["--json-layout", "album", "--force", "--show-results"],
        &album,
    );
    assert!(forced.status.success());
    assert!(String::from_utf8_lossy(&forced.stderr).contains("Analyzing"));
    assert!(String::from_utf8_lossy(&forced.stdout).contains("Authenticity:"));
    use std::io::Write;
    std::fs::OpenOptions::new()
        .append(true)
        .open(&audio)
        .unwrap()
        .write_all(&[0, 0])
        .unwrap();
    let changed = run(&["--json-layout", "album"], &album);
    assert!(String::from_utf8_lossy(&changed.stderr).contains("Analyzing"));
}

#[test]
fn artist_layout_splits_albums_and_groups_disc_folders() {
    let dir = tempfile::tempdir().unwrap();
    let artist = dir.path().join("Artist");
    for folder in ["Album A/CD1", "Album A/CD2", "Album B"] {
        let path = artist.join(folder);
        std::fs::create_dir_all(&path).unwrap();
        silent_audio(&path.join("track.wav"));
    }
    let output = run(&["--json-layout", "artist"], &artist);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (name, count) in [("Album A", 2), ("Album B", 1)] {
        let report = report::parse_json(
            &std::fs::read_to_string(artist.join(format!("{name}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(report.files.len(), count);
    }
}

#[test]
fn malformed_destination_requires_force() {
    let dir = tempfile::tempdir().unwrap();
    silent_audio(&dir.path().join("track.wav"));
    let output = dir.path().join("report.json");
    std::fs::write(&output, "garbage").unwrap();
    let result = run(&["--json", output.to_str().unwrap()], dir.path());
    assert!(!result.status.success());
    assert_eq!(std::fs::read_to_string(&output).unwrap(), "garbage");
    let result = run(&["--json", output.to_str().unwrap(), "--force"], dir.path());
    assert!(result.status.success());
}
