//! Running the detector over a set of dropped or selected targets.
//!
//! Analysis never modifies the files it scans — they are opened read-only, in
//! `core`, and nothing here writes anything.

use flaccompagnon_core::{self as core, FolderReport, ScanOptions};
use tauri::{AppHandle, Emitter};

use super::batch::{
    cancelled, display_root, gather_targets, parallel_map_ordered, reset_cancel, Progress,
};
use super::file_name;
use crate::spectrogram;

/// Analyze the dropped/selected `targets` — any mix of audio files and folders —
/// and return the structured result. No files are written; use the commands in
/// [`super::report`] for that.
#[tauri::command]
pub async fn analyze_paths(app: AppHandle, targets: Vec<String>) -> Result<FolderReport, String> {
    if targets.is_empty() {
        return Err("Nothing to analyze.".to_string());
    }
    let opts = ScanOptions {
        // ffmpeg (when present) enables DSD content analysis.
        ffmpeg: spectrogram::resolve_ffmpeg(),
        ..ScanOptions::default()
    };
    let paths = gather_targets(&targets, opts.recursive);
    if paths.is_empty() {
        return Err("No supported audio files found.".to_string());
    }
    let total = paths.len();
    let root_str = display_root(&targets);

    reset_cancel();
    let app_bg = app.clone();
    // Files are independent and CPU-bound, so they are analyzed in parallel;
    // `parallel_map_ordered` is what guarantees the results still come back in
    // the sorted order the report depends on. See its docs for why that isn't
    // free.
    let report_opt = tauri::async_runtime::spawn_blocking(move || {
        let files = parallel_map_ordered(
            &paths,
            cancelled,
            |path| core::analyze_file(path, &opts),
            |done, path| {
                let _ = app_bg.emit(
                    "analyze://progress",
                    Progress {
                        // The bar shows how many are *finished behind* this
                        // one, so it reads 0/N while the first is still going.
                        current: done.saturating_sub(1),
                        total,
                        file: file_name(path),
                    },
                );
            },
        )?;

        let has_flac = files.iter().any(|f| f.flac_md5.is_some());
        Some(FolderReport {
            root: root_str,
            files,
            has_flac,
        })
    })
    .await
    .map_err(|e| e.to_string())?;

    let report = report_opt.ok_or_else(|| "cancelled".to_string())?;

    let _ = app.emit(
        "analyze://progress",
        Progress {
            current: total,
            total,
            file: String::new(),
        },
    );
    Ok(report)
}

/// Whether a usable `ffmpeg` is present on the system (gates the spectrogram UI
/// and DSD content analysis).
#[tauri::command]
pub async fn ffmpeg_available() -> bool {
    spectrogram::resolve_ffmpeg().is_some()
}
