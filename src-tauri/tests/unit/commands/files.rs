use super::*;

#[test]
fn only_the_absent_paths_come_back() {
    let dir = tempfile::tempdir().expect("tempdir");
    let here = dir.path().join("here.flac");
    std::fs::write(&here, b"x").expect("write");
    let gone = dir.path().join("gone.flac");
    // A directory sitting where a track used to be is still "missing":
    // the row cannot be played, tagged or converted from it.
    let as_dir = dir.path().join("now-a-folder.flac");
    std::fs::create_dir(&as_dir).expect("mkdir");

    let out = missing_paths(vec![
        here.to_string_lossy().into(),
        gone.to_string_lossy().into(),
        as_dir.to_string_lossy().into(),
    ]);
    assert_eq!(out.len(), 2, "{out:?}");
    assert!(out.iter().any(|p| p.ends_with("gone.flac")));
    assert!(out.iter().any(|p| p.ends_with("now-a-folder.flac")));
}

#[test]
fn an_empty_request_is_an_empty_answer() {
    assert!(missing_paths(Vec::new()).is_empty());
}
#[test]
fn relative_file_manager_arguments_cannot_be_interpreted_as_options() {
    let current = std::env::current_dir().unwrap();
    let dir = tempfile::tempdir_in(&current).unwrap();
    let file = dir.path().join("-R");
    std::fs::write(&file, b"audio").unwrap();
    let relative = file.strip_prefix(&current).unwrap();
    let resolved = browser_path(relative.to_str().unwrap(), "File").unwrap();
    assert!(resolved.is_absolute());
    assert_eq!(resolved, file.canonicalize().unwrap());
}
