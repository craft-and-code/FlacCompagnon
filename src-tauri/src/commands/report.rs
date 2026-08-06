//! Writing analysis results out (CSV, JSON, M3U) and reading a saved report
//! back in.
//!
//! Everything written here lives *outside* the audio files — these commands
//! never touch a track. The frontend supplies the rows in display order, so
//! what gets exported is exactly what is on screen.

use std::path::{Path, PathBuf};

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

/// Write only the CSV report for an already-analyzed result — the toolbar's
/// "Save…" (which calls this and [`save_report_json`] in sequence, see its
/// frontend doc comment for why) and the menu bar's standalone "Export CSV".
#[tauri::command]
pub async fn save_report_csv(dest: String, report: FolderReport) -> Result<String, String> {
    let path = stem_with_ext(&dest, "csv")?;
    core::report::write_csv(&path, &report).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

/// Write only the JSON report — same shape as [`save_report_csv`], for the
/// menu bar's standalone "Export JSON" (and the second half of "Save…").
#[tauri::command]
pub async fn save_report_json(dest: String, report: FolderReport) -> Result<String, String> {
    let path = stem_with_ext(&dest, "json")?;
    core::report::write_json(&path, &report).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
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
    entries: Vec<core::playlist::PlaylistEntry>,
    format: core::playlist::PlaylistFormat,
) -> Result<String, String> {
    let ext = match format {
        core::playlist::PlaylistFormat::Extended => "m3u8",
        core::playlist::PlaylistFormat::Simple => "m3u",
    };
    let out_path = stem_with_ext(&dest, ext)?;

    let content = core::playlist::build_playlist(&entries, format);
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
mod tests {
    use super::*;

    #[test]
    fn forces_the_extension_it_promises() {
        let out = stem_with_ext("/tmp/report.csv", "json").expect("valid");
        assert_eq!(out, PathBuf::from("/tmp/report.json"));
        // Already correct: unchanged.
        let out = stem_with_ext("/tmp/report.json", "json").expect("valid");
        assert_eq!(out, PathBuf::from("/tmp/report.json"));
        // No extension at all: one is added.
        let out = stem_with_ext("/tmp/report", "csv").expect("valid");
        assert_eq!(out, PathBuf::from("/tmp/report.csv"));
    }

    /// A frontend asking to write "evil.sh" must still get a .csv — this is
    /// the whole point of forcing the extension backend-side.
    #[test]
    fn a_hostile_extension_cannot_survive() {
        let out = stem_with_ext("/tmp/evil.sh", "csv").expect("valid");
        assert_eq!(out, PathBuf::from("/tmp/evil.csv"));
        let out = stem_with_ext("/tmp/evil.command", "json").expect("valid");
        assert_eq!(out, PathBuf::from("/tmp/evil.json"));
    }

    #[test]
    fn rejects_a_destination_with_no_file_name() {
        assert!(stem_with_ext("", "csv").is_err());
        assert!(stem_with_ext("/", "csv").is_err());
        assert!(stem_with_ext("..", "csv").is_err());
    }
}
