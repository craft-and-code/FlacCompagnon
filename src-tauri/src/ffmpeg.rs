//! Locate the optional decoder used only for DSD content.

use std::process::{Command, Stdio};

/// Locate a working `ffmpeg` executable, or `None` if none is found.
pub fn resolve_ffmpeg() -> Option<String> {
    resolve_with(std::env::var(FFMPEG_ENV).ok(), ffmpeg_works)
}

/// Environment variable that overrides the search entirely.
const FFMPEG_ENV: &str = "FLACCOMPAGNON_FFMPEG";

/// The resolution itself, with the environment and the "does it run?" probe
/// passed in so the precedence rule can be tested without a real ffmpeg or a
/// process-global `set_var` (which would make the test suite order-dependent).
///
/// An `explicit` path that is set but *doesn't* work falls through to the
/// normal search rather than failing: a stale variable in someone's shell
/// profile shouldn't disable the feature outright.
fn resolve_with(explicit: Option<String>, works: impl Fn(&str) -> bool) -> Option<String> {
    if let Some(p) = explicit.filter(|p| !p.is_empty()) {
        if works(&p) {
            return Some(p);
        }
    }
    candidates().into_iter().find(|c| works(c))
}

fn candidates() -> Vec<String> {
    let mut v = vec!["ffmpeg".to_string()];
    #[cfg(target_os = "macos")]
    v.extend(
        [
            "/opt/homebrew/bin/ffmpeg",
            "/usr/local/bin/ffmpeg",
            "/usr/bin/ffmpeg",
            "/opt/local/bin/ffmpeg",
        ]
        .map(String::from),
    );
    #[cfg(target_os = "linux")]
    v.extend(
        [
            "/usr/bin/ffmpeg",
            "/usr/local/bin/ffmpeg",
            "/snap/bin/ffmpeg",
            "/var/lib/flatpak/exports/bin/ffmpeg",
        ]
        .map(String::from),
    );
    #[cfg(target_os = "windows")]
    v.extend(
        [
            "ffmpeg.exe",
            "C:\\ffmpeg\\bin\\ffmpeg.exe",
            "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe",
        ]
        .map(String::from),
    );
    v
}

fn ffmpeg_works(path: &str) -> bool {
    Command::new(path)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "../tests/unit/ffmpeg.rs"]
mod tests;
