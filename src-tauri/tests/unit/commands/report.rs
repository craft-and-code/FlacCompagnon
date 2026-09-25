use super::*;

#[test]
fn forces_the_extension_it_promises() {
    let out = stem_with_ext("/tmp/report.csv", "json").expect("valid");
    assert_eq!(out, PathBuf::from("/tmp/report.json"));
    // Already correct: unchanged.
    let out = stem_with_ext("/tmp/report.json", "json").expect("valid");
    assert_eq!(out, PathBuf::from("/tmp/report.json"));
    // No extension at all: one is added.
    let out = stem_with_ext("/tmp/report", "csv").expect("valid");
    assert_eq!(out, PathBuf::from("/tmp/report.csv"));
}

/// A frontend asking to write "evil.sh" must still get a .csv — this is
/// the whole point of forcing the extension backend-side.
#[test]
fn a_hostile_extension_cannot_survive() {
    let out = stem_with_ext("/tmp/evil.sh", "csv").expect("valid");
    assert_eq!(out, PathBuf::from("/tmp/evil.csv"));
    let out = stem_with_ext("/tmp/evil.command", "json").expect("valid");
    assert_eq!(out, PathBuf::from("/tmp/evil.json"));
}

#[test]
fn rejects_a_destination_with_no_file_name() {
    assert!(stem_with_ext("", "csv").is_err());
    assert!(stem_with_ext("/", "csv").is_err());
    assert!(stem_with_ext("..", "csv").is_err());
}

#[test]
fn report_write_error_names_the_report_and_destination() {
    let path = Path::new("/Users/example/Desktop/Album.json");
    let error = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    let message = report_write_error(path, "JSON", &error);

    assert!(message.contains("JSON report"));
    assert!(message.contains(&path.display().to_string()));
    #[cfg(target_os = "macos")]
    assert!(message.contains("Files & Folders"));
}
