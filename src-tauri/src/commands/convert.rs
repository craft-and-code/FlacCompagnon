//! Converting the panel's imported tracks to another format.
//!
//! Mirrors [`super::analysis::analyze_paths`]'s shape (gather targets, run in
//! parallel over a thread pool, emit progress, honour cancellation) but
//! writes files instead of only reading them — the reason this lives in its
//! own module rather than folded into `analysis`, and the reason cancelling
//! is a different problem here: an abandoned analysis leaves no trace, an
//! abandoned conversion leaves however many files its workers had already
//! finished. See the `let Some(outcomes)` branch below and
//! [`convert::undo_batch`]. Pausing playback and freezing the rest of the UI
//! while this runs is the frontend's job (the player already exposes
//! `pause_playback`/`resume_playback`); this command only does the
//! conversion itself.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use flaccompagnon_core::convert::{self, ConvertSettings};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::batch::{cancelled, gather_targets, parallel_map_ordered, reset_cancel, Progress};
use super::file_name;

/// Summary returned once a conversion batch finishes (or is cancelled — see
/// below, a cancelled run returns an error instead, the same way
/// `analyze_paths` does).
#[derive(Clone, Serialize)]
pub struct ConvertSummary {
    total: usize,
    converted: usize,
    failed: usize,
    /// How many non-audio files (covers, playlists, spectrograms, ...) were
    /// copied verbatim — 0 unless `copy_others` was set.
    copied: usize,
    output_root: String,
    /// One message per failed file, `"name: reason"` — the batch keeps going
    /// past a single bad file rather than aborting the rest.
    errors: Vec<String>,
    /// Files that converted correctly but whose tags could not be carried
    /// over. Kept apart from `errors` on purpose: these files exist and play,
    /// so counting them as failures would send the user hunting for output
    /// that is sitting right there.
    tag_warnings: Vec<String>,
}

/// Expand what the conversion panel was handed — any mix of audio files and
/// folders — into the audio files it will actually convert.
///
/// The panel used to keep the dropped paths verbatim, which made its own
/// count wrong the moment a folder arrived: one row reading "1 track
/// imported" for what might hold a hundred. Resolving it here rather than
/// guessing in the frontend has two further benefits — the list then shows
/// exactly what will be written, and a single track inside a dropped folder
/// can be removed like any other, which was impossible while the folder was
/// one opaque row.
///
/// Recursive, and it applies the same filtering [`convert_files`] would
/// (extension-based, generated `spectres/` folders skipped), so what the panel
/// lists and what the batch writes cannot disagree.
///
/// Each file is returned with the `base` its output layout will be measured
/// against — the *parent* of the dropped target, so a dropped folder is
/// itself recreated at the destination. Recording it here is the whole point:
/// once the folders are expanded, nothing downstream can tell what was
/// dropped, and deriving a root from the files instead flattens the very case
/// this exists for (see `core::convert::layout`).
#[tauri::command]
pub async fn list_convert_sources(targets: Vec<String>) -> Result<Vec<SourceEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut out = Vec::new();
        for target in &targets {
            let target_path = PathBuf::from(target);
            // A dropped file mirrors nothing, so its base is its own parent
            // and it lands directly in the output folder. A dropped folder is
            // reproduced, so the base is one level up.
            let base = target_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| target_path.clone());
            for path in gather_targets(std::slice::from_ref(target), true) {
                out.push(SourceEntry {
                    path: path.to_string_lossy().to_string(),
                    base: base.to_string_lossy().to_string(),
                });
            }
        }
        out
    })
    .await
    .map_err(|e| e.to_string())
}

/// One importable audio file and the folder its output layout is measured
/// against. Mirrors `ConvertSource` in `src/types.ts`.
#[derive(Clone, Serialize, Deserialize)]
pub struct SourceEntry {
    /// Absolute path of the audio file.
    pub path: String,
    /// Folder `path`'s position is expressed relative to.
    pub base: String,
}

/// Convert every audio file implied by `targets` (the panel's own imported
/// files/folders, independent of the main results table) to `settings`'s
/// format, writing the result under `output_root` in the same relative
/// layout the sources have today. When `copy_others` is set, every other
/// file sharing a source folder (covers, `.m3u` playlists, generated
/// spectrograms, ...) is copied there too, unconverted.
#[tauri::command]
pub async fn convert_files(
    app: AppHandle,
    targets: Vec<SourceEntry>,
    output_root: String,
    settings: ConvertSettings,
    copy_others: bool,
) -> Result<ConvertSummary, String> {
    if targets.is_empty() {
        return Err("Nothing to convert.".to_string());
    }
    // Already expanded and filtered by `list_convert_sources`; re-walking here
    // would also throw away the `base` each file was imported with.
    let sources: Vec<convert::ConvertSource> = targets
        .into_iter()
        .map(|t| convert::ConvertSource {
            path: PathBuf::from(t.path),
            base: PathBuf::from(t.base),
        })
        .collect();
    let total = sources.len();
    let output_root_path = PathBuf::from(&output_root);
    let destinations = convert::plan_batch(&sources, &output_root_path, settings.format);
    // Taken before a single file is written, so a cancellation can tell the
    // outputs *this* batch created from ones that were already sitting in the
    // chosen folder (an earlier run, most likely) and must survive it — see
    // `convert::undo_batch`.
    let preexisting: HashSet<PathBuf> = destinations
        .iter()
        .filter(|dest| dest.exists())
        .cloned()
        .collect();
    let pairs: Vec<(PathBuf, PathBuf)> = sources
        .iter()
        .map(|s| s.path.clone())
        .zip(destinations.iter().cloned())
        .collect();

    reset_cancel();
    let app_bg = app.clone();
    let outcomes = tauri::async_runtime::spawn_blocking(move || {
        parallel_map_ordered(
            &pairs,
            cancelled,
            // Handed down into the per-file work too, not just checked
            // between files: a hi-res track takes seconds to decode and
            // re-encode, and without this Cancel wouldn't be felt until every
            // worker's current file had run to completion.
            |(src, dest)| convert::convert_file(src, dest, &settings, &cancelled),
            |done, (src, _)| {
                let _ = app_bg.emit(
                    "convert://progress",
                    Progress {
                        current: done.saturating_sub(1),
                        total,
                        file: file_name(src),
                    },
                );
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?;

    // Unlike a cancelled analysis, which leaves nothing behind because it only
    // ever read, a cancelled conversion has already written every file that
    // finished before the click landed. Reporting "cancelled" while those sit
    // in the output folder is the worst of both — so they go before the error
    // does.
    let Some(outcomes) = outcomes else {
        convert::undo_batch(&destinations, &preexisting, &output_root_path);
        return Err("cancelled".to_string());
    };

    let mut errors: Vec<String> = outcomes
        .iter()
        .filter_map(|r| r.as_ref().err().map(ToString::to_string))
        .collect();
    // A tag warning belongs to a file that *did* convert, so it is collected
    // from the `Ok` side and never counted in `failed`.
    let tag_warnings: Vec<String> = outcomes
        .iter()
        .filter_map(|r| r.as_ref().ok().and_then(|o| o.tag_warning.clone()))
        .collect();
    let failed = errors.len();
    let converted = outcomes.len() - failed;

    let mut copied = 0usize;
    if copy_others {
        // No exclusion list to assemble here anymore: `passthrough_files`
        // takes the sources themselves and derives both what to sweep and
        // what to skip, so the two cannot drift apart.
        match convert::passthrough_files(&sources, &output_root_path) {
            Ok(written) => copied = written.len(),
            Err(e) => errors.push(format!("copy: {e}")),
        }
    }

    let _ = app.emit(
        "convert://progress",
        Progress {
            current: total,
            total,
            file: String::new(),
        },
    );

    Ok(ConvertSummary {
        total,
        converted,
        failed,
        copied,
        output_root,
        errors,
        tag_warnings,
    })
}
