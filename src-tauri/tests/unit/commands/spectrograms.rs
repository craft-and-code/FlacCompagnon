use super::*;

#[test]
fn generated_folder_is_created_and_can_be_reused() {
    let root = tempfile::tempdir().unwrap();
    let directory = output_directory(root.path()).unwrap();
    assert!(directory.is_dir());
    assert_eq!(output_directory(root.path()).unwrap(), directory);
}

#[test]
fn an_existing_file_cannot_become_the_generated_folder() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("spectrograms");
    std::fs::write(&file, b"keep").unwrap();
    assert!(output_directory(root.path()).is_err());
    assert_eq!(std::fs::read(file).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn a_linked_spectrogram_folder_cannot_redirect_output() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let image = outside.path().join("track.png");
    std::fs::write(&image, b"keep").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("spectrograms")).unwrap();
    assert!(output_directory(root.path()).is_err());
    assert_eq!(std::fs::read(image).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn a_dangling_spectrogram_folder_link_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("missing");
    std::os::unix::fs::symlink(&target, root.path().join("spectrograms")).unwrap();
    assert!(output_directory(root.path()).is_err());
    assert!(!target.exists());
}
