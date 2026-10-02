//! Integrity-only CLI checks, using an independent encoder as ground truth.
use std::path::Path;
use std::process::{Command, Output};

use flacenc::component::BitRepr;
use flacenc::error::Verify;

fn flac_fixture() -> Vec<u8> {
    let mut config = flacenc::config::Encoder::default();
    config.multithread = false;
    let config = config.into_verified().expect("encoder settings");
    let source = flacenc::source::MemSource::from_samples(&[123, -456].repeat(256), 2, 16, 44_100);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .expect("reference encoder");
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream.write(&mut sink).expect("serialize FLAC");
    sink.as_slice().to_vec()
}

fn check(path: &Path, internal_only: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_flaccompagnon"));
    command.args(["--force", "-a", "flac-md5"]).arg(path);
    if internal_only {
        command.env("PATH", "");
    }
    command.output().expect("run integrity check")
}

#[test]
fn flac_md5_only_verifies_audio_with_or_without_the_optional_native_tool() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("track.flac");
    std::fs::write(&path, flac_fixture()).expect("FLAC fixture");
    for internal_only in [false, true] {
        let result = check(&path, internal_only);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&result.stdout).trim(),
            "FLAC MD5: OK"
        );
    }
    assert_eq!(
        std::fs::read_dir(dir.path())
            .expect("fixture directory")
            .count(),
        1,
        "integrity-only scans must not create partial JSON reports"
    );
}

#[test]
fn flac_md5_only_returns_failure_for_a_mismatching_signature() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("wrong-md5.flac");
    let mut bytes = flac_fixture();
    bytes[26] ^= 1;
    std::fs::write(&path, bytes).expect("altered signature");
    for internal_only in [false, true] {
        let result = check(&path, internal_only);
        assert!(!result.status.success());
        assert!(!String::from_utf8_lossy(&result.stdout).contains("MD5: OK"));
    }
}

#[test]
fn flac_md5_only_distinguishes_unsigned_audio_from_malformed_files() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("unsigned.flac");
    let mut bytes = flac_fixture();
    bytes[26..42].fill(0);
    std::fs::write(&path, bytes).expect("unsigned fixture");
    for internal_only in [false, true] {
        let result = check(&path, internal_only);
        assert!(result.status.success());
        assert!(String::from_utf8_lossy(&result.stdout).contains("No MD5 signature"));
    }
    std::fs::write(&path, b"not a FLAC").expect("malformed fixture");
    let result = check(&path, true);
    assert!(!result.status.success());
}

#[test]
fn requesting_json_produces_a_complete_analysis_report() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("track.flac");
    let destination = dir.path().join("report.json");
    std::fs::write(&path, flac_fixture()).expect("FLAC fixture");
    let result = Command::new(env!("CARGO_BIN_EXE_flaccompagnon"))
        .args(["--force", "--json"])
        .arg(&destination)
        .arg(&path)
        .output()
        .expect("run full analysis");
    assert!(result.status.success());
    let json = std::fs::read_to_string(destination).expect("JSON report");
    let report = flaccompagnon_core::report::parse_json(&json).expect("valid desktop report");
    let file = report.files.first().expect("analyzed file");
    assert!(
        file.file_md5.is_some(),
        "JSON must include the fingerprints"
    );
    assert!(file.file_crc32.is_some());
    assert_eq!(
        file.flac_md5,
        Some(flaccompagnon_core::FlacMd5Status::Match)
    );
}

#[test]
fn requested_combinations_only_print_the_selected_measurements() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("track.flac");
    std::fs::write(&path, flac_fixture()).expect("fixture");
    for (selection, lines) in [
        ("bit-depth", 1),
        ("flac-md5,bit-depth", 2),
        ("flac-md5,bit-depth,fingerprints", 3),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_flaccompagnon"))
            .args(["--force", "-a", selection])
            .arg(&path)
            .output()
            .expect("CLI");
        assert!(result.status.success());
        assert_eq!(
            String::from_utf8_lossy(&result.stdout).lines().count(),
            lines
        );
        assert!(!String::from_utf8_lossy(&result.stdout).contains("Authenticity:"));
    }
}

#[test]
fn partial_cache_is_reused_only_when_it_covers_the_request() {
    use flaccompagnon_core::{AnalysisSelection, ScanOptions};
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("track.flac");
    std::fs::write(&path, flac_fixture()).expect("fixture");
    let selection = AnalysisSelection::from_names(["bit-depth"]).expect("selection");
    let file = flaccompagnon_core::analyze_file_selected(&path, &ScanOptions::default(), selection);
    let report = flaccompagnon_core::folder_report(dir.path(), vec![file]);
    let destination = dir.path().join("partial.json");
    flaccompagnon_core::report::write_json(&destination, &report).expect("partial fixture");
    for (requested, reused) in [("bit-depth", true), ("bit-depth,flac-md5", false)] {
        let result = Command::new(env!("CARGO_BIN_EXE_flaccompagnon"))
            .args(["-a", requested])
            .arg(&path)
            .output()
            .expect("CLI");
        assert!(result.status.success());
        let progress = String::from_utf8_lossy(&result.stderr);
        assert_eq!(progress.contains("Reusing "), reused);
        assert_eq!(progress.contains("Analyzing "), !reused);
    }
    let result = Command::new(env!("CARGO_BIN_EXE_flaccompagnon"))
        .arg("--json")
        .arg(&destination)
        .arg(&path)
        .output()
        .expect("complete JSON scan");
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Analyzing "));
    let json = std::fs::read_to_string(&destination).expect("JSON");
    let complete = flaccompagnon_core::report::parse_json(&json).expect("report");
    assert!(complete.files[0].analyses_run.is_none());
    assert!(complete.files[0].file_md5.is_some());
}
