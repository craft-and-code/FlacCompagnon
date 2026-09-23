//! Renaming a file on disk.
//!
//! The results table's "click twice on the name to rename" — the UI only ever
//! lets the user edit the stem (see `ResultRow.tsx`), never the extension, and
//! this file is where that rule is enforced again server-side, so a
//! compromised or buggy frontend can never smuggle a different extension
//! through. Unlike [`super::tags`], this changes where a file *lives*, never
//! what's inside it — the audio and its tags are untouched.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// What a successful rename returns: the new path, plus its file name split
/// out separately (rather than making the frontend re-derive it from the
/// path with a `split(/[\\/]/)` regex that would have to reinvent the same
/// cross-platform rules `Path::file_name` already gets right).
#[derive(Serialize)]
pub struct RenameResult {
    path: String,
    file_name: String,
}

fn to_result(p: &Path) -> RenameResult {
    RenameResult {
        path: p.to_string_lossy().to_string(),
        file_name: p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
    }
}

/// Rejects a proposed file-name stem that would escape its folder, produce no
/// name at all, or embed a null byte — the same class of "untrusted string
/// that becomes part of a path" risk `cover_extension` guards against in
/// tags.rs, just for a name that came straight from a text field instead of a
/// MIME type sniffed from file bytes.
fn validate_stem(stem: &str) -> Result<&str, String> {
    let trimmed = stem.trim();
    if trimmed.is_empty() {
        return Err("File name cannot be empty.".to_string());
    }
    if trimmed.contains(['/', '\\', '\0']) {
        return Err("File name cannot contain \"/\", \"\\\" or a null character.".to_string());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("That is not a valid file name.".to_string());
    }
    Ok(trimmed)
}

/// The destination for renaming `path`'s stem to `new_stem`, keeping its
/// extension (or lack of one) exactly as it was.
fn renamed_path(path: &Path, new_stem: &str) -> PathBuf {
    let file_name = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{new_stem}.{ext}"),
        None => new_stem.to_string(),
    };
    path.with_file_name(file_name)
}

/// Renames `path` to `new_stem` plus its existing extension, in the same
/// folder. Returns the new path and file name. A `new_stem` that produces the
/// exact same path as `path` is a no-op (not an error) — the user
/// re-confirmed the name they already had, most likely by pressing Enter
/// without changing anything.
#[tauri::command]
pub async fn rename_file(path: String, new_stem: String) -> Result<RenameResult, String> {
    let stem = validate_stem(&new_stem)?.to_string();
    let src = PathBuf::from(path);
    let dest = renamed_path(&src, &stem);

    if dest == src {
        return Ok(to_result(&src));
    }

    tauri::async_runtime::spawn_blocking(move || {
        // Checked separately from the `rename` call itself so the error names
        // the actual conflicting file rather than surfacing whatever generic
        // OS error a same-name `rename` happens to produce (which varies by
        // platform, and on some doesn't fail at all — it silently replaces
        // the existing file, which is exactly what this guards against).
        if dest.exists() {
            let name = dest
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("that name");
            return Err(format!("\"{name}\" already exists in this folder."));
        }
        std::fs::rename(&src, &dest).map_err(|e| e.to_string())?;
        Ok(to_result(&dest))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "../../tests/unit/commands/rename.rs"]
mod tests;
