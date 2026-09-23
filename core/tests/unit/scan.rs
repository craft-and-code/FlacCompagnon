use super::*;
use std::fs;

fn touch(dir: &Path, rel: &str) {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(&p, b"").expect("write");
}

#[test]
fn every_supported_extension_is_recognized() {
    for ext in SUPPORTED_EXTENSIONS {
        let name = format!("track.{ext}");
        assert!(is_supported_audio(Path::new(&name)), "{ext}");
        // Uppercase too: file systems are case-insensitive in practice.
        let upper = format!("track.{}", ext.to_uppercase());
        assert!(is_supported_audio(Path::new(&upper)), "{upper}");
    }
    assert!(!is_supported_audio(Path::new("cover.jpg")));
    assert!(!is_supported_audio(Path::new("notes.txt")));
    assert!(!is_supported_audio(Path::new("README")));
    // An extension that merely *contains* a supported one is not one.
    assert!(!is_supported_audio(Path::new("track.flacx")));
}

/// The symptom: a `.opus` file dropped on the app vanished without a
/// word, while the *same* Opus audio named `.ogg` was accepted (and then
/// failed at decode with a real message). Whether a file is worth trying
/// must not depend on which of two names its container was given.
#[test]
fn opus_is_accepted_under_both_its_container_names() {
    for name in ["track.opus", "track.ogg", "track.oga", "TRACK.OPUS"] {
        assert!(is_supported_audio(Path::new(name)), "{name}");
    }
}

/// Generated spectrograms live next to the audio; a rescan must not pick
/// up anything from there, at any depth — under either folder name, since
/// libraries scanned before the rename still hold `spectres/`.
#[test]
fn generated_spectrogram_folders_are_skipped_under_both_names() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    touch(root, "a.flac");
    touch(root, "spectrograms/a.flac");
    touch(root, "spectres/a.flac");
    touch(root, "album/b.flac");
    touch(root, "album/spectrograms/b.flac");
    touch(root, "album/spectres/b.flac");

    let found = list_audio_files(root, true);
    assert_eq!(found.len(), 2, "found: {found:?}");
    assert!(found.iter().all(|p| !p
        .components()
        .any(|c| GENERATED_DIRS.iter().any(|d| c.as_os_str() == *d))));
}

#[test]
fn non_recursive_stays_in_the_top_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    touch(root, "a.flac");
    touch(root, "album/b.flac");

    assert_eq!(list_audio_files(root, false).len(), 1);
    assert_eq!(list_audio_files(root, true).len(), 2);
}

/// The list is sorted, because it is what the report's row order comes
/// from before the user reorders anything.
#[test]
fn results_are_sorted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    for name in ["c.flac", "a.flac", "b.flac"] {
        touch(root, name);
    }
    let found = list_audio_files(root, false);
    let mut sorted = found.clone();
    sorted.sort();
    assert_eq!(found, sorted);
}

/// A folder that doesn't exist is an empty list, not a panic — the path
/// can come from a stale saved report.
#[test]
fn a_missing_root_yields_nothing() {
    let missing = Path::new("/definitely/not/a/real/folder/anywhere");
    assert!(list_audio_files(missing, true).is_empty());
}
