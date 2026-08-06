//! Batch spectrogram rendering: one PNG per track, in a `spectres/` folder
//! next to the source file.
//!
//! The spectrogram is the final arbiter when a detection is ambiguous, so this
//! is deliberately a manual action — it writes files, and analysis on its own
//! never does.

use std::path::Path;

use flaccompagnon_core as core;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::batch::{cancelled, gather_targets, reset_cancel, Progress};
use super::file_name;
use crate::spectrogram;

/// Summary returned after a spectrogram batch.
#[derive(Clone, Serialize)]
pub struct SpectroSummary {
    total: usize,
    rendered: usize,
    failed: usize,
    spectres_dirs: Vec<String>,
    errors: Vec<String>,
}

/// Shown when ffmpeg is missing. Long on purpose: this is the one dependency
/// the app cannot bundle, and "not found" alone leaves the user stuck.
const NO_FFMPEG: &str = "ffmpeg was not found on your system. Install it and try again \
     (macOS: `brew install ffmpeg`, Debian/Ubuntu: `sudo apt install ffmpeg`, \
     Windows: `choco install ffmpeg`). You can also set the FLACCOMPAGNON_FFMPEG \
     environment variable to its full path.";

/// Render a spectrogram PNG for every audio file implied by `targets`.
#[tauri::command]
pub async fn generate_spectrograms(
    app: AppHandle,
    targets: Vec<String>,
) -> Result<SpectroSummary, String> {
    if targets.is_empty() {
        return Err("Nothing to render.".to_string());
    }

    // Resolve ffmpeg before doing anything else, so a missing install fails
    // immediately rather than once per file.
    let ffmpeg = spectrogram::resolve_ffmpeg().ok_or_else(|| NO_FFMPEG.to_string())?;

    let paths = gather_targets(&targets, true);
    if paths.is_empty() {
        return Err("No supported audio files found.".to_string());
    }
    let total = paths.len();

    reset_cancel();
    let app_bg = app.clone();
    let summary = tauri::async_runtime::spawn_blocking(move || {
        let mut rendered = 0usize;
        let mut errors: Vec<String> = Vec::new();
        let mut spectres_dirs: Vec<String> = Vec::new();

        for (i, p) in paths.iter().enumerate() {
            if cancelled() {
                break;
            }
            let _ = app_bg.emit(
                "spectro://progress",
                Progress {
                    current: i,
                    total,
                    file: file_name(p),
                },
            );

            let parent = p.parent().unwrap_or_else(|| Path::new("."));
            let spectres_dir = parent.join("spectres");
            if let Err(e) = std::fs::create_dir_all(&spectres_dir) {
                errors.push(format!("{}: {e}", file_name(p)));
                continue;
            }
            let dir_str = spectres_dir.to_string_lossy().to_string();
            if !spectres_dirs.contains(&dir_str) {
                spectres_dirs.push(dir_str);
            }

            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("track");
            let out = spectres_dir.join(format!("{stem}.png"));
            let info = core::probe_info(p).ok();

            match spectrogram::render(&ffmpeg, p, &out, info.as_ref()) {
                Ok(()) => rendered += 1,
                Err(e) => errors.push(format!("{}: {e}", file_name(p))),
            }
        }

        SpectroSummary {
            total,
            rendered,
            // Derived rather than counted in parallel with `errors`: the two
            // were incremented separately before, which is one edit away from
            // reporting a failure count that doesn't match the error list.
            failed: errors.len(),
            spectres_dirs,
            errors,
        }
    })
    .await
    .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "spectro://progress",
        Progress {
            current: total,
            total,
            file: String::new(),
        },
    );

    Ok(summary)
}
