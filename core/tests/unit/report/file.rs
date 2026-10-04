use super::*;

fn empty_report() -> FolderReport {
    FolderReport {
        root: "/music".into(),
        files: vec![],
        has_flac: false,
    }
}

#[test]
fn oversized_reports_are_rejected_before_reading_their_contents() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.json");
    File::create(&path)
        .unwrap()
        .set_len(MAX_JSON_BYTES + 1)
        .unwrap();
    assert!(read_json(&path).unwrap_err().contains("64 MiB"));
}

#[test]
fn malformed_report_is_rejected_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broken.json");
    std::fs::write(&path, b"{broken").unwrap();
    assert!(read_json(&path).is_err());
}

#[test]
fn replacing_a_hardlinked_report_does_not_truncate_its_other_name() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.json");
    let destination = dir.path().join("export.json");
    std::fs::write(&original, b"original bytes").unwrap();
    std::fs::hard_link(&original, &destination).unwrap();
    write_json(&destination, &empty_report()).unwrap();
    assert_eq!(std::fs::read(&original).unwrap(), b"original bytes");
    assert_eq!(read_json(&destination).unwrap().root, "/music");
}

#[test]
fn json_and_csv_replace_reports_with_complete_output() {
    let dir = tempfile::tempdir().unwrap();
    let report = empty_report();
    let json = dir.path().join("report.json");
    let csv = dir.path().join("report.csv");
    for _ in 0..2 {
        write_json(&json, &report).unwrap();
        write_csv(&csv, &report).unwrap();
    }
    assert_eq!(read_json(&json).unwrap().root, report.root);
    assert_eq!(std::fs::read_to_string(csv).unwrap(), build_csv(&report));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[cfg(unix)]
#[test]
fn report_exports_never_follow_a_symlink_into_audio() {
    let dir = tempfile::tempdir().unwrap();
    let audio = dir.path().join("track.flac");
    let destination = dir.path().join("export.json");
    std::fs::write(&audio, b"audio bytes").unwrap();
    std::os::unix::fs::symlink(&audio, &destination).unwrap();
    assert!(write_json(&destination, &empty_report()).is_err());
    assert!(write_csv(&destination, &empty_report()).is_err());
    assert_eq!(std::fs::read(&audio).unwrap(), b"audio bytes");
}

#[cfg(unix)]
#[test]
fn named_pipe_reports_are_rejected_without_opening_them() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pipe.json");
    assert!(std::process::Command::new("mkfifo")
        .arg(&path)
        .status()
        .unwrap()
        .success());
    assert!(read_json(&path).is_err());
    assert!(open_regular_file(&path).is_err());
    assert!(write_json(&path, &empty_report()).is_err());
}
