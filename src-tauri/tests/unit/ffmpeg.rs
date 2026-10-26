use super::*;

/// PATH is searched first, so a user who installed ffmpeg anywhere and put
/// it on their PATH wins over the hardcoded list.
#[test]
fn the_bare_name_is_tried_first() {
    assert_eq!(candidates().first().map(String::as_str), Some("ffmpeg"));
}

/// The absolute paths exist precisely because an app launched from Finder
/// (or a .desktop file) does not inherit the shell's PATH. Dropping the
/// package-manager location would silently break the feature for most
/// users while still passing every test run from a terminal.
#[test]
fn the_usual_install_locations_are_covered() {
    let c = candidates();
    #[cfg(target_os = "macos")]
    assert!(c.iter().any(|p| p == "/opt/homebrew/bin/ffmpeg"), "{c:?}");
    #[cfg(target_os = "linux")]
    assert!(c.iter().any(|p| p == "/usr/bin/ffmpeg"), "{c:?}");
    #[cfg(target_os = "windows")]
    assert!(c.iter().any(|p| p.ends_with("ffmpeg.exe")), "{c:?}");
    // No duplicates: each candidate costs a process spawn to probe.
    let mut sorted = c.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), c.len(), "duplicate candidate in {c:?}");
}

#[test]
fn an_explicit_override_wins_over_the_search() {
    let got = resolve_with(Some("/custom/ffmpeg".to_string()), |_| true);
    assert_eq!(got.as_deref(), Some("/custom/ffmpeg"));
}

/// A stale `FLACCOMPAGNON_FFMPEG` left in a shell profile must not disable
/// the feature — the normal search still runs.
#[test]
fn a_broken_override_falls_back_to_the_search() {
    let got = resolve_with(Some("/gone/ffmpeg".to_string()), |p| p != "/gone/ffmpeg");
    assert_eq!(got.as_deref(), Some("ffmpeg"));
    // An empty variable is treated as unset, not as an empty path.
    let got = resolve_with(Some(String::new()), |p| !p.is_empty());
    assert_eq!(got.as_deref(), Some("ffmpeg"));
}

#[test]
fn nothing_found_is_none_not_a_panic() {
    assert_eq!(resolve_with(None, |_| false), None);
    assert_eq!(resolve_with(Some("/x".to_string()), |_| false), None);
}

/// Probing a path that isn't an executable must answer "no", not fail.
#[test]
fn probing_a_nonexistent_binary_is_false() {
    assert!(!ffmpeg_works("/definitely/not/a/binary/anywhere"));
    assert!(!ffmpeg_works(""));
}
