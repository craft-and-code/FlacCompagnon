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
use serde::Deserialize;

/// Spectrum canvas dimensions passed to ffmpeg. Its legend extends the final
/// PNG beyond these dimensions, by an amount determined by ffmpeg.
///
/// `Half` is Aède's default: a 900×470 canvas, exactly half of the full canvas
/// in both directions. `Full` preserves FlacCompagnon's former 1800×940 canvas
/// for detailed inspection or direct comparison with older images.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpectrogramSize {
    #[default]
    Half,
    Full,
}

impl SpectrogramSize {
    fn dimensions(self) -> &'static str {
        match self {
            Self::Half => "900x470",
            Self::Full => "1800x940",
        }
    }
}

fn spectrum(size: SpectrogramSize) -> String {
    format!(
        "showspectrumpic=s={}:mode=combined:legend=1:color=intensity:scale=log:gain=3",
        size.dimensions()
    )
}

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
    size: SpectrogramSize,
) -> Result<(), String> {
    let input_s = input.to_string_lossy().to_string();
    let output_s = output.to_string_lossy().to_string();

    let base = spectrum(size);
    let filter_with_text = match info {
        Some(i) => format!(
            "{base},drawtext=text='{}':fontcolor=white:fontsize=24:x=14:y=12:box=1:boxcolor=black@0.55",
            caption(i)
        ),
        None => base.clone(),
    };

    // Preferred: spectrum + caption. Fall back to spectrum-only if drawtext
    // fails (e.g. no usable font); the legend still shows frequency to Nyquist.
    if run(ffmpeg, &input_s, &filter_with_text, &output_s).is_ok() {
        return Ok(());
    }
    run(ffmpeg, &input_s, &base, &output_s)
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
#[path = "../tests/unit/spectrogram.rs"]
mod tests;
