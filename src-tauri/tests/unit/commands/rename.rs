use super::*;

#[test]
fn keeps_the_original_extension() {
    assert_eq!(
        renamed_path(Path::new("/music/01 old.flac"), "01 new"),
        PathBuf::from("/music/01 new.flac")
    );
}

#[test]
fn keeps_no_extension_when_there_was_none() {
    assert_eq!(
        renamed_path(Path::new("/music/README"), "NOTES"),
        PathBuf::from("/music/NOTES")
    );
}

#[test]
fn a_dotfile_style_name_has_no_extension_to_keep() {
    // ".hidden"'s "extension" per `Path::extension()` is actually `None`
    // (a name that's *only* a leading dot has no further dot to split
    // on) — this just confirms the same rule Rust's stdlib already uses
    // is what protects `renamed_path` here, not a special case of ours.
    assert_eq!(
        renamed_path(Path::new("/music/.hidden"), "renamed"),
        PathBuf::from("/music/renamed")
    );
}

#[test]
fn rejects_empty_or_whitespace_only_names() {
    assert!(validate_stem("").is_err());
    assert!(validate_stem("   ").is_err());
}

#[test]
fn trims_surrounding_whitespace() {
    assert_eq!(validate_stem("  My Title  ").unwrap(), "My Title");
}

#[test]
fn rejects_path_separators_and_nul() {
    for bad in ["a/b", "a\\b", "a\0b", "../escape", "/etc/passwd"] {
        assert!(validate_stem(bad).is_err(), "{bad:?} should be rejected");
    }
}

#[test]
fn rejects_dot_and_dotdot() {
    assert!(validate_stem(".").is_err());
    assert!(validate_stem("..").is_err());
}
