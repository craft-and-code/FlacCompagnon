use super::*;

#[test]
fn an_existing_output_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("existing.wav");
    std::fs::write(&dest, b"original").unwrap();
    assert!(StagedOutput::new(&dest, ConvertFormat::Wav).is_err());
    assert_eq!(std::fs::read(&dest).unwrap(), b"original");
}

#[test]
fn a_destination_created_during_encoding_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("late.wav");
    let output = StagedOutput::new(&dest, ConvertFormat::Wav).unwrap();
    let scratch = output.path().to_path_buf();
    std::fs::write(&scratch, b"new audio").unwrap();
    std::fs::write(&dest, b"another writer").unwrap();
    assert!(output.publish(&dest).is_err());
    assert_eq!(std::fs::read(&dest).unwrap(), b"another writer");
    assert!(!scratch.exists());
}

#[test]
fn a_failed_encode_leaves_no_partial_output() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("failed.wav");
    let output = StagedOutput::new(&dest, ConvertFormat::Wav).unwrap();
    let scratch = output.path().to_path_buf();
    std::fs::write(&scratch, b"partial").unwrap();
    drop(output);
    assert!(!dest.exists());
    assert!(!scratch.exists());
}

#[cfg(unix)]
#[test]
fn a_dangling_output_symlink_is_never_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("linked.wav");
    std::os::unix::fs::symlink(dir.path().join("missing.wav"), &dest).unwrap();
    assert!(StagedOutput::new(&dest, ConvertFormat::Wav).is_err());
    assert!(std::fs::symlink_metadata(dest)
        .unwrap()
        .file_type()
        .is_symlink());
}
