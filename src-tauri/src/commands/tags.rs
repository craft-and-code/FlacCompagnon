//! Reading and writing tags and cover art.
//!
//! These are the only commands in the app that modify the user's audio files,
//! and only ever on an explicit Save from the tag panel. Both batch commands
//! report failures **per file** rather than aborting: one locked or read-only
//! track must not cost the user the other forty-nine.

use std::path::PathBuf;

use flaccompagnon_core as core;
use serde::Serialize;

use super::file_name;

/// One file's tags, or the reason they couldn't be read (unsupported format,
/// corrupt file, ...).
#[derive(Clone, Serialize)]
pub struct TagReadResult {
    path: String,
    tags: Option<core::tags::TagSet>,
    error: Option<String>,
}

/// Outcome of a batch tag write: how many of the targeted files were updated,
/// and the per-file errors for the rest.
#[derive(Clone, Serialize)]
pub struct TagWriteSummary {
    total: usize,
    written: usize,
    failed: usize,
    errors: Vec<String>,
}

/// Read the tags of an already-known set of files (the rows currently
/// selected in the results table — this does not expand folders, unlike
/// `analyze_paths`). Backs the tag panel's pre-fill.
#[tauri::command]
pub async fn read_tags_batch(paths: Vec<String>) -> Result<Vec<TagReadResult>, String> {
    if paths.is_empty() {
        return Err("Nothing to read.".to_string());
    }
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .iter()
            .map(|p| {
                let path = p.to_string_lossy().to_string();
                match core::tags::read_tags(p) {
                    Ok(tags) => TagReadResult {
                        path,
                        tags: Some(tags),
                        error: None,
                    },
                    Err(e) => TagReadResult {
                        path,
                        tags: None,
                        error: Some(e.to_string()),
                    },
                }
            })
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| e.to_string())
}

/// Apply the same [`core::tags::TagEdits`] to every file in `paths` — the tag
/// panel's Save button.
#[tauri::command]
pub async fn write_tags_batch(
    paths: Vec<String>,
    edits: core::tags::TagEdits,
) -> Result<TagWriteSummary, String> {
    if paths.is_empty() {
        return Err("Nothing to write.".to_string());
    }
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let total = paths.len();
    tauri::async_runtime::spawn_blocking(move || {
        let mut written = 0usize;
        let mut errors: Vec<String> = Vec::new();
        for p in &paths {
            match core::tags::write_tags(p, &edits) {
                Ok(()) => written += 1,
                Err(e) => errors.push(format!("{}: {e}", file_name(p))),
            }
        }
        TagWriteSummary {
            total,
            written,
            failed: errors.len(),
            errors,
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Read a plain image file dropped onto the tag panel's cover box (not one
/// of the audio files in the table) — backs "drop an image to replace every
/// selected file's cover".
#[tauri::command]
pub async fn read_cover_image(path: String) -> Result<core::tags::CoverArt, String> {
    let path = PathBuf::from(path);
    tauri::async_runtime::spawn_blocking(move || {
        core::tags::read_cover_file(&path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Write a cover already in hand (as base64 — nothing is re-read from any tag)
/// out as a plain "cover.<ext>" file in `dir`, backing the tag panel's
/// "extract this cover" button — the same convention Mp3tag and foobar2000
/// use. Overwrites anything already at that path. Returns the path written.
#[tauri::command]
pub async fn extract_cover_art(
    dir: String,
    mime: String,
    data_base64: String,
) -> Result<String, String> {
    let dest = PathBuf::from(dir).join(format!("cover.{}", cover_extension(&mime)));
    tauri::async_runtime::spawn_blocking(move || {
        core::tags::write_cover_file(&dest, &data_base64)
            .map(|_| dest.to_string_lossy().to_string())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// File extension for an image MIME type.
///
/// The MIME string is sniffed from the picture bytes embedded in an untrusted
/// audio file, and the result becomes part of a file name we then write — so
/// it is mapped through a fixed list rather than pasted in. Without that, a
/// picture declaring `image/../../../x` would decide where the file lands.
fn cover_extension(mime: &str) -> &'static str {
    match mime.trim().to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/webp" => "webp",
        "image/tiff" => "tiff",
        _ => "img",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_image_types() {
        assert_eq!(cover_extension("image/jpeg"), "jpg");
        assert_eq!(cover_extension("image/png"), "png");
        assert_eq!(cover_extension("IMAGE/PNG"), "png");
        assert_eq!(cover_extension(" image/webp "), "webp");
    }

    /// A MIME type from an untrusted file must never be able to steer where
    /// the extracted cover is written.
    #[test]
    fn a_hostile_mime_cannot_escape_the_folder() {
        for mime in [
            "image/../../../etc/passwd",
            "image/png/../../evil",
            "image/sh",
            "",
            "../..",
            "image/png\0.sh",
        ] {
            let ext = cover_extension(mime);
            assert!(
                !ext.contains(['/', '\\', '.', '\0']),
                "{mime} produced {ext}"
            );
        }
    }
}
