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
