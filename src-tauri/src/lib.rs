//! FlacCompagnon Tauri backend: exposes analysis, CSV export and spectrogram
//! commands to the web frontend and streams progress events.
//!
//! Nothing here ever modifies the audio files being analyzed — they are only
//! ever opened read-only. The only files written are the (optional, on-demand)
//! CSV export and the spectrogram PNGs, both of which live outside the tracks.

mod spectrogram;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use flaccompagnon_core::{self as core, FolderReport, ScanOptions};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Set when the user requests cancellation of the in-progress batch. Only one
/// long-running operation runs at a time (the UI enforces this), so a single
/// global flag is sufficient.
static CANCEL: AtomicBool = AtomicBool::new(false);

/// Progress event payload emitted during long-running operations.
#[derive(Clone, Serialize)]
struct Progress {
    current: usize,
    total: usize,
    file: String,
}

/// Summary returned after a spectrogram batch.
#[derive(Clone, Serialize)]
struct SpectroSummary {
    total: usize,
    rendered: usize,
    failed: usize,
    spectres_dirs: Vec<String>,
    errors: Vec<String>,
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
}

/// Collect the audio files implied by a dropped/selected `target`, which may be
/// either a single audio file or a folder.
fn collect_paths(target: &Path, recursive: bool) -> Vec<PathBuf> {
    if target.is_file() {
        if core::is_supported_audio(target) {
            vec![target.to_path_buf()]
        } else {
            Vec::new()
        }
    } else {
        core::list_audio_files(target, recursive)
    }
}

/// Gather, de-duplicate and sort the audio files implied by a set of dropped or
/// selected `targets` (any mix of files and folders).
fn gather_targets(targets: &[String], recursive: bool) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for t in targets {
        let tp = PathBuf::from(t);
        if tp.exists() {
            paths.extend(collect_paths(&tp, recursive));
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// The folder shown as the report "root": the folder itself for a single dropped
/// folder, otherwise the parent folder of the first item.
fn display_root(targets: &[String]) -> String {
    if let Some(first) = targets.first() {
        let p = PathBuf::from(first);
        let root = if p.is_dir() {
            p
        } else {
            p.parent().map(|x| x.to_path_buf()).unwrap_or(p)
        };
        return root.to_string_lossy().to_string();
    }
    String::new()
}

/// Analyze the dropped/selected `targets` — any mix of audio files and folders —
/// and return the structured result. No files are written; use `save_csv`.
#[tauri::command]
async fn analyze_paths(app: AppHandle, targets: Vec<String>) -> Result<FolderReport, String> {
    if targets.is_empty() {
        return Err("Nothing to analyze.".to_string());
    }
    let opts = ScanOptions::default();
    let paths = gather_targets(&targets, opts.recursive);
    if paths.is_empty() {
        return Err("No supported audio files found.".to_string());
    }
    let total = paths.len();
    let root_str = display_root(&targets);

    CANCEL.store(false, Ordering::SeqCst);
    let app_bg = app.clone();
    let report_opt = tauri::async_runtime::spawn_blocking(move || {
        let mut files = Vec::with_capacity(total);
        for (i, p) in paths.iter().enumerate() {
            if CANCEL.load(Ordering::SeqCst) {
                return None;
            }
            let _ = app_bg.emit(
                "analyze://progress",
                Progress {
                    current: i,
                    total,
                    file: file_name(p),
                },
            );
            files.push(core::analyze_file(p, &opts));
        }
        let has_flac = files.iter().any(|f| f.flac_md5.is_some());
        Some(FolderReport {
            root: root_str,
            files,
            has_flac,
        })
    })
    .await
    .map_err(|e| e.to_string())?;

    let report = match report_opt {
        Some(r) => r,
        None => return Err("cancelled".to_string()),
    };

    let _ = app.emit(
        "analyze://progress",
        Progress { current: total, total, file: String::new() },
    );
    Ok(report)
}

/// Whether a usable `ffmpeg` is present on the system (gates the spectrogram UI).
#[tauri::command]
async fn ffmpeg_available() -> bool {
    spectrogram::resolve_ffmpeg().is_some()
}

/// Request cancellation of the running analysis / spectrogram batch. The loops
/// check this between files and stop at the next boundary.
#[tauri::command]
fn cancel_task() {
    CANCEL.store(true, Ordering::SeqCst);
}

/// Reveal a file in the OS file browser (Finder / Explorer / default manager),
/// selecting it when the platform supports it. Lets the user quickly locate and
/// play a track.
#[tauri::command]
fn reveal_in_folder(path: String) -> Result<(), String> {
    // Only reveal paths that actually exist (no shell is involved anywhere —
    // all arguments go through Command::arg — but this avoids handing garbage
    // to the OS file manager).
    if !std::path::Path::new(&path).exists() {
        return Err("File not found.".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{path}"))
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        // No portable "select the file" across Linux file managers; open the
        // containing directory instead.
        let p = std::path::Path::new(&path);
        let dir = p.parent().unwrap_or(p);
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Write the CSV report for an already-analyzed result to `dest`.
///
/// Defense in depth: only ever writes files with a `.csv` extension, so a
/// compromised frontend cannot use this command to overwrite arbitrary files.
#[tauri::command]
async fn save_csv(dest: String, report: FolderReport) -> Result<String, String> {
    let path = Path::new(&dest);
    let is_csv = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("csv"))
        .unwrap_or(false);
    if !is_csv {
        return Err("The destination must be a .csv file.".to_string());
    }
    core::report::write_csv(path, &report).map_err(|e| e.to_string())?;
    Ok(dest)
}

/// Render a spectrogram PNG for every audio file implied by `targets`, placing
/// each image in a `spectres/` folder next to the source file.
#[tauri::command]
async fn generate_spectrograms(app: AppHandle, targets: Vec<String>) -> Result<SpectroSummary, String> {
    if targets.is_empty() {
        return Err("Nothing to render.".to_string());
    }

    // Resolve ffmpeg from the system before doing anything else.
    let ffmpeg = spectrogram::resolve_ffmpeg().ok_or_else(|| {
        "ffmpeg was not found on your system. Install it and try again \
         (macOS: `brew install ffmpeg`, Debian/Ubuntu: `sudo apt install ffmpeg`, \
         Windows: `choco install ffmpeg`). You can also set the FLACCOMPAGNON_FFMPEG \
         environment variable to its full path."
            .to_string()
    })?;

    let paths = gather_targets(&targets, true);
    if paths.is_empty() {
        return Err("No supported audio files found.".to_string());
    }
    let total = paths.len();

    CANCEL.store(false, Ordering::SeqCst);
    let app_bg = app.clone();
    let summary = tauri::async_runtime::spawn_blocking(move || {
        let mut rendered = 0usize;
        let mut failed = 0usize;
        let mut errors: Vec<String> = Vec::new();
        let mut spectres_dirs: Vec<String> = Vec::new();

        for (i, p) in paths.iter().enumerate() {
            if CANCEL.load(Ordering::SeqCst) {
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
                failed += 1;
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
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{}: {e}", file_name(p)));
                }
            }
        }

        SpectroSummary {
            total,
            rendered,
            failed,
            spectres_dirs,
            errors,
        }
    })
    .await
    .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "spectro://progress",
        Progress { current: total, total, file: String::new() },
    );

    Ok(summary)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            analyze_paths,
            save_csv,
            ffmpeg_available,
            cancel_task,
            reveal_in_folder,
            generate_spectrograms
        ])
        .run(tauri::generate_context!())
        .expect("error while running FlacCompagnon");
}
