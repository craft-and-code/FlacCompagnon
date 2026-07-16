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
    if let Ok(p) = std::env::var("FLACCOMPAGNON_FFMPEG") {
        if !p.is_empty() && ffmpeg_works(&p) {
            return Some(p);
        }
    }
    candidates().into_iter().find(|c| ffmpeg_works(c))
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
fn caption(info: &BasicInfo) -> String {
    let bits = info
        .bits
        .map(|b| format!("{b}-bit"))
        .unwrap_or_else(|| "float".to_string());
    let nyquist = info.sample_rate / 2;
    format!(
        "{} Hz | {} | {} ch | {} | Nyquist {} Hz",
        info.sample_rate, bits, info.channels, info.format, nyquist
    )
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
