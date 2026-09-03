//! Handing a path to the OS file browser.
//!
//! Two commands rather than one, because the platforms distinguish them and
//! so does the UI: revealing *a file* selects it inside its parent folder
//! (the results table's magnifier), while opening *a folder* shows its
//! contents (the folder icon next to the analyzed root path).
//!
//! No shell is ever involved — every path goes through `Command::arg`, so a
//! file name containing quotes, spaces or `;` is just a file name.

use std::path::Path;
use std::process::Command;

/// Reveal a file in the OS file browser (Finder / Explorer / default manager),
/// selecting it when the platform supports it.
#[tauri::command]
pub fn reveal_in_folder(path: String) -> Result<(), String> {
    // Only reveal paths that actually exist — this avoids handing garbage to
    // the OS file manager, which reports it far less clearly than we can.
    if !Path::new(&path).exists() {
        return Err("File not found.".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        spawn(Command::new("open").arg("-R").arg(&path))?;
    }
    #[cfg(target_os = "windows")]
    {
        spawn(Command::new("explorer").arg(format!("/select,{path}")))?;
    }
    #[cfg(target_os = "linux")]
    {
        // No portable "select the file" across Linux file managers; open the
        // containing directory instead.
        let p = Path::new(&path);
        spawn(Command::new("xdg-open").arg(p.parent().unwrap_or(p)))?;
    }
    Ok(())
}

/// Open a folder in the OS file browser, showing *its* contents — unlike
/// [`reveal_in_folder`], which selects a file within its *parent*.
#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    if !Path::new(&path).is_dir() {
        return Err("Folder not found.".to_string());
    }
    #[cfg(target_os = "macos")]
    let mut cmd = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut cmd = Command::new("explorer");
    #[cfg(target_os = "linux")]
    let mut cmd = Command::new("xdg-open");

    spawn(cmd.arg(&path))
}

/// Launch `cmd` and forget about it: these open a GUI application, so waiting
/// for it to exit would block until the user closes their file manager.
fn spawn(cmd: &mut Command) -> Result<(), String> {
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Which of `paths` no longer exist on disk.
///
/// Returns only the missing ones rather than a parallel array of booleans:
/// the answer is almost always empty, and an empty vector says "nothing to
/// worry about" without the caller having to zip anything back together.
///
/// A `metadata` call per path and nothing else — no decoding, no analysis.
/// This is what backs the results table's refresh button and the automatic
/// check after a saved `.json` report is reloaded, where the paths in the
/// report may point at files that have since been moved, renamed or deleted.
///
/// A path that exists but is now a *directory* counts as missing: whatever is
/// there, it is no longer the track the report describes.
#[tauri::command]
pub fn missing_paths(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .filter(|p| !Path::new(p).is_file())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_absent_paths_come_back() {
        let dir = tempfile::tempdir().expect("tempdir");
        let here = dir.path().join("here.flac");
        std::fs::write(&here, b"x").expect("write");
        let gone = dir.path().join("gone.flac");
        // A directory sitting where a track used to be is still "missing":
        // the row cannot be played, tagged or converted from it.
        let as_dir = dir.path().join("now-a-folder.flac");
        std::fs::create_dir(&as_dir).expect("mkdir");

        let out = missing_paths(vec![
            here.to_string_lossy().into(),
            gone.to_string_lossy().into(),
            as_dir.to_string_lossy().into(),
        ]);
        assert_eq!(out.len(), 2, "{out:?}");
        assert!(out.iter().any(|p| p.ends_with("gone.flac")));
        assert!(out.iter().any(|p| p.ends_with("now-a-folder.flac")));
    }

    #[test]
    fn an_empty_request_is_an_empty_answer() {
        assert!(missing_paths(Vec::new()).is_empty());
    }
}
