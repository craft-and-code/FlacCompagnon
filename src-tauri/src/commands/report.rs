//! Writing analysis results out (CSV, JSON, M3U) and reading a saved report
//! back in.
//!
//! Everything written here lives *outside* the audio files — these commands
//! never touch a track. The frontend supplies the rows in display order, so
//! what gets exported is exactly what is on screen.

use std::{
    io,
    path::{Path, PathBuf},
};

use flaccompagnon_core::{self as core, FolderReport};

/// Swap `dest`'s extension for `ext`, keeping its stem and parent folder.
///
/// Defense in depth, shared by every save-to-disk command below: the
/// extension actually written is always forced here from what the command
/// means to produce, never trusted from the frontend — so a compromised
/// frontend can't use one of these commands to write a file of any other
/// type than the one its name promises.
fn stem_with_ext(dest: &str, ext: &str) -> Result<PathBuf, String> {
    let path = Path::new(dest);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Invalid destination file name.".to_string())?;
    Ok(path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(format!("{stem}.{ext}")))
}

/// Explain an operating-system write refusal at the point where the user chose
/// the destination. macOS treats Desktop, Documents and Downloads as protected
/// folders for an installed app, while a development build often inherits the
/// terminal's existing permission and masks the difference.
fn report_write_error(path: &Path, format: &str, error: &io::Error) -> String {
    if error.kind() == io::ErrorKind::PermissionDenied {
        #[cfg(target_os = "macos")]
        return format!(
            "Could not write the {format} report to '{}': macOS denied access. Allow FlacCompagnon in System Settings → Privacy & Security → Files & Folders, then try again.",
            path.display()
        );
    }
    format!(
        "Could not write the {format} report to '{}': {error}",
        path.display()
    )
}

/// Write a report selected in the native save dialog, keeping the extension and
/// operating-system error handling identical for CSV and JSON exports.
fn save_report(
    dest: String,
    report: FolderReport,
    extension: &str,
    format: &str,
    write: fn(&Path, &FolderReport) -> io::Result<()>,
) -> Result<String, String> {
    let path = stem_with_ext(&dest, extension)?;
    write(&path, &report).map_err(|error| report_write_error(&path, format, &error))?;
    Ok(path.to_string_lossy().to_string())
}

/// Write only the CSV report for an already-analyzed result — the toolbar's
/// "Save…" (which calls this and [`save_report_json`] in sequence, see its
/// frontend doc comment for why) and the menu bar's standalone "Export CSV".
#[tauri::command]
pub async fn save_report_csv(dest: String, report: FolderReport) -> Result<String, String> {
    save_report(dest, report, "csv", "CSV", core::report::write_csv)
}

/// Write only the JSON report — same shape as [`save_report_csv`], for the
/// menu bar's standalone "Export JSON" (and the second half of "Save…").
#[tauri::command]
pub async fn save_report_json(dest: String, report: FolderReport) -> Result<String, String> {
    save_report(dest, report, "json", "JSON", core::report::write_json)
}

/// Write the table's current order out as a playlist (Simple or Extended
/// M3U — picked by the user in the export pop-in, or directly by the
/// corresponding menu bar item). The frontend builds each `entries` item
/// from data it already has (display order, per-file duration from the
/// analysis, cached tags) — this command just turns that into text and
/// writes it, no re-reading of the tracks themselves.
#[tauri::command]
pub async fn save_playlist(
    dest: String,
    entries: Vec<flaccompagnon_services::playlist::PlaylistEntry>,
    format: flaccompagnon_services::playlist::PlaylistFormat,
) -> Result<String, String> {
    let ext = match format {
        flaccompagnon_services::playlist::PlaylistFormat::Extended => "m3u8",
        flaccompagnon_services::playlist::PlaylistFormat::Simple => "m3u",
    };
    let out_path = stem_with_ext(&dest, ext)?;

    let content = flaccompagnon_services::playlist::build_playlist(&entries, format);
    let written = out_path.to_string_lossy().to_string();
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(&out_path, content).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(written)
}

/// Load a previously-saved JSON report (dropped onto the window) and return it
/// as a [`FolderReport`], ready to render without re-analyzing any audio.
#[tauri::command]
pub async fn load_report(path: String) -> Result<FolderReport, String> {
    // Reading and parsing both block; a large report on a slow disk would
    // otherwise stall the async runtime's thread.
    tauri::async_runtime::spawn_blocking(move || {
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        core::report::parse_json(&text)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "../../tests/unit/commands/report.rs"]
mod tests;
