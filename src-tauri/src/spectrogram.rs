//! Spectrogram rendering through a system-installed `ffmpeg`.
//!
//! ffmpeg is resolved at runtime rather than bundled, so the build never
//! depends on a sidecar binary. Resolution order:
//!   1. the `FLACCOMPAGNON_FFMPEG` environment variable, if set;
//!   2. `ffmpeg` on the `PATH`;
//!   3. a list of common install locations (important on macOS, where an app
//!      launched from Finder does not inherit the shell `PATH` and therefore
//!      cannot see Homebrew's `/opt/homebrew/bin`).
//!
//! `showspectrumpic` with `legend=1` draws a labelled frequency axis (its top
//! equals Nyquist = sample_rate / 2); a caption drawn on top spells out the
//! sample rate / bit depth / format explicitly. If `drawtext` is unavailable we
//! transparently retry without the caption.

use std::path::Path;
use std::process::{Command, Stdio};

use flaccompagnon_core::BasicInfo;

const SPECTRUM: &str =
    "showspectrumpic=s=1800x940:mode=combined:legend=1:color=intensity:scale=log:gain=3";

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

/// Human-readable caption drawn on the spectrogram.
///
/// The result is embedded in an ffmpeg `drawtext` filter expression, and the
/// format label is derived from the *file extension* (untrusted input), so the
/// whole caption is restricted to a safe character set — no quotes, colons,
/// commas or backslashes can reach the filter graph.
fn caption(info: &BasicInfo) -> String {
    let bits = info
        .bits
        .map(|b| format!("{b}-bit"))
        .unwrap_or_else(|| "float".to_string());
    let nyquist = info.sample_rate / 2;
    let raw = format!(
        "{} Hz | {} | {} ch | {} | Nyquist {} Hz",
        info.sample_rate, bits, info.channels, info.format, nyquist
    );
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '|' | '-' | '/' | '.'))
        .collect()
}

/// Render a spectrogram PNG for `input` into `output` using `ffmpeg`.
pub fn render(
    ffmpeg: &str,
    input: &Path,
    output: &Path,
    info: Option<&BasicInfo>,
) -> Result<(), String> {
    let input_s = input.to_string_lossy().to_string();
    let output_s = output.to_string_lossy().to_string();

    let filter_with_text = match info {
        Some(i) => format!(
            "{SPECTRUM},drawtext=text='{}':fontcolor=white:fontsize=24:x=14:y=12:box=1:boxcolor=black@0.55",
            caption(i)
        ),
        None => SPECTRUM.to_string(),
    };

    // Preferred: spectrum + caption. Fall back to spectrum-only if drawtext
    // fails (e.g. no usable font); the legend still shows frequency to Nyquist.
    if run(ffmpeg, &input_s, &filter_with_text, &output_s).is_ok() {
        return Ok(());
    }
    run(ffmpeg, &input_s, SPECTRUM, &output_s)
}

fn run(ffmpeg: &str, input: &str, filter: &str, output: &str) -> Result<(), String> {
    let out = Command::new(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            input,
            "-lavfi",
            filter,
            "-frames:v",
            "1",
            output,
        ])
        .output()
        .map_err(|e| format!("failed to run ffmpeg: {e}"))?;

    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(format: &str) -> BasicInfo {
        BasicInfo {
            sample_rate: 44_100,
            channels: 2,
            bits: Some(16),
            format: format.to_string(),
        }
    }

    #[test]
    fn caption_reads_like_a_caption() {
        let c = caption(&info("FLAC"));
        assert_eq!(c, "44100 Hz | 16-bit | 2 ch | FLAC | Nyquist 22050 Hz");
    }

    #[test]
    fn float_sources_have_no_bit_depth() {
        let mut i = info("WAV");
        i.bits = None;
        assert!(caption(&i).contains("float"));
        assert!(!caption(&i).contains("-bit"));
    }

    /// Every character that could end a `drawtext` argument, start another
    /// filter, or escape out of the expression must be dropped. `format`
    /// comes from the file's own extension, so it is attacker-chosen: a file
    /// named `x.'a,b` would otherwise inject into the filter graph ffmpeg is
    /// handed.
    ///
    /// This is the test the whole `filter()` in `caption` exists for — if
    /// someone widens that character set, it fails here rather than in
    /// someone's music folder.
    #[test]
    fn caption_cannot_break_out_of_the_ffmpeg_filter_graph() {
        let hostile = "A'B\"C:D,E\\F;G=H[I]J{K}L`M$N\nO\rP\tQ%R*S?T<U>V&W(X)";
        let c = caption(&info(hostile));
        for bad in [
            '\'', '"', ':', ',', '\\', ';', '=', '[', ']', '{', '}', '`', '$', '\n', '\r', '\t',
            '%', '*', '?', '<', '>', '&', '(', ')',
        ] {
            assert!(!c.contains(bad), "{bad:?} survived in {c:?}");
        }
        // The harmless letters are still there, so the filter isn't just
        // emptying the string.
        assert!(c.contains("ABCDEF"), "{c:?}");
    }

    /// A format label is derived from an arbitrary file extension, which can
    /// be any Unicode at all. Non-ASCII is dropped rather than passed to
    /// ffmpeg's font renderer, and nothing panics on a multi-byte character.
    #[test]
    fn caption_drops_non_ascii_without_panicking() {
        for weird in ["é", "日本語", "🎵", "A\u{202E}B", "\u{0}"] {
            let c = caption(&info(weird));
            assert!(c.is_ascii(), "{c:?}");
            assert!(c.starts_with("44100 Hz"), "{c:?}");
        }
    }

    /// The caption is built from numbers the container declared, which may be
    /// absurd on a malformed file. Nothing here may divide by zero or panic.
    #[test]
    fn caption_survives_degenerate_stream_parameters() {
        let mut i = info("FLAC");
        i.sample_rate = 0;
        i.channels = 0;
        i.bits = Some(0);
        assert!(caption(&i).contains("Nyquist 0 Hz"));

        let mut i = info("FLAC");
        i.sample_rate = u32::MAX;
        assert!(!caption(&i).is_empty());
    }

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
}
