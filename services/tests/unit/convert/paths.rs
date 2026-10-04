use super::*;

#[test]
fn a_missing_parent_does_not_hide_later_parent_traversal() {
    let dir = tempfile::tempdir().unwrap();
    let escaped = dir.path().join("missing/../../outside.flac");
    assert!(validate_output_directory(dir.path(), &escaped).is_err());
}

#[cfg(unix)]
#[test]
fn an_explicitly_selected_symlink_root_remains_supported() {
    let dir = tempfile::tempdir().unwrap();
    let actual = dir.path().join("actual");
    let selected = dir.path().join("selected");
    std::fs::create_dir_all(&actual).unwrap();
    std::os::unix::fs::symlink(&actual, &selected).unwrap();
    validate_output_directory(&selected, &selected.join("Album/track.flac")).unwrap();
    assert_eq!(
        resolve_output_root(&selected).unwrap(),
        actual.canonicalize().unwrap()
    );
}
