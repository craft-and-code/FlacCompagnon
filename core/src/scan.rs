//! Finding the files worth analyzing, and analyzing a whole folder of them.

use std::path::{Path, PathBuf};

use crate::pipeline::analyze_file;
use crate::types::{AnalysisError, FileAnalysis, FolderReport, ScanOptions};

/// Audio file extensions FlacCompagnon will attempt to *analyze* — which is
/// not the same as the ones it can currently decode.
///
/// `.opus` is listed even though there is no Opus decoder yet (see
/// [`crate::decode`]). Leaving it out was worse, not safer: `.ogg` and `.oga`
/// are here, an Ogg stream can just as well carry Opus, so the same audio was
/// accepted under one name and silently refused at the drop under another. A
/// listed file that fails with "Opus decoding is not supported yet" tells the
/// user what is going on; a file that vanishes on drop tells them the app is
/// broken. The day a decoder is wired in, this entry needs no change.
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "flac", "wav", "wave", "aif", "aiff", "aifc", "alac", "m4a", "mp4", "caf", "ogg", "oga",
    "opus", "mp3", "aac", "dsf", "dff",
];

/// Folder names used for generated spectrograms. Files inside one are skipped
/// so a second scan doesn't try to analyze the app's own PNG output — and, more
/// to the point, so re-scanning a folder never grows the file list.
///
/// Two names, because only the first is written any more: `spectres` was the
/// original (French) name, and libraries scanned before the rename still hold
/// those folders. Dropping it from this list would not corrupt anything, but
/// it would silently change the behaviour of every folder a long-time user
/// already has — the kind of regression nobody thinks to test for.
const GENERATED_DIRS: [&str; 2] = ["spectrograms", "spectres"];

/// Returns `true` if `path` has an extension FlacCompagnon knows how to decode.
///
/// The check is on the extension only and is case-insensitive; the *real*
/// container is identified later from the file's magic bytes (which is how a
/// WAV renamed to `.flac` gets flagged).
///
/// ```
/// use std::path::Path;
/// use flaccompagnon_core::is_supported_audio;
///
/// assert!(is_supported_audio(Path::new("song.flac")));
/// assert!(is_supported_audio(Path::new("song.FLAC")));   // case-insensitive
/// assert!(is_supported_audio(Path::new("album.dsf")));   // DSD
/// assert!(!is_supported_audio(Path::new("cover.jpg")));
/// assert!(!is_supported_audio(Path::new("README")));     // no extension
/// ```
pub fn is_supported_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// List every supported audio file under `root`, sorted, skipping any file that
/// lives inside a generated spectrogram folder (see `GENERATED_DIRS`).
pub fn list_audio_files(root: &Path, recursive: bool) -> Vec<PathBuf> {
    let depth = if recursive { usize::MAX } else { 1 };
    let mut paths: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .max_depth(depth)
        .into_iter()
        // A directory we cannot read (permissions, a broken symlink) is
        // skipped rather than failing the scan: one unreadable folder must
        // not cost the user the rest of their library.
        .filter_map(Result::ok)
        .map(|e| e.into_path())
        .filter(|p| p.is_file() && is_supported_audio(p))
        .filter(|p| {
            !p.components()
                .any(|c| GENERATED_DIRS.iter().any(|d| c.as_os_str() == *d))
        })
        .collect();
    paths.sort();
    paths
}

/// Expand files and folders into one sorted, deduplicated list of supported audio.
pub fn gather_targets(targets: &[String], recursive: bool) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for target in targets {
        let path = Path::new(target);
        if path.is_file() {
            if is_supported_audio(path) {
                paths.push(path.to_path_buf());
            }
        } else if path.is_dir() {
            paths.extend(list_audio_files(path, recursive));
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// Report root for a folder or the first individual file's parent.
pub fn display_root(targets: &[String]) -> String {
    let Some(first) = targets.first() else {
        return String::new();
    };
    let path = Path::new(first);
    let root = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    root.to_string_lossy().to_string()
}

/// Assemble files already analyzed by any client into a standard report.
pub fn folder_report(root: &Path, files: Vec<FileAnalysis>) -> FolderReport {
    let has_flac = files.iter().any(|file| file.flac_md5.is_some());
    FolderReport {
        root: root.to_string_lossy().to_string(),
        files,
        has_flac,
    }
}

/// Analyze every supported audio file under `root`.
pub fn analyze_folder(root: &Path, opts: &ScanOptions) -> Result<FolderReport, AnalysisError> {
    let paths = list_audio_files(root, opts.recursive);
    let files: Vec<FileAnalysis> = paths.iter().map(|p| analyze_file(p, opts)).collect();
    Ok(folder_report(root, files))
}

#[cfg(test)]
#[path = "../tests/unit/scan.rs"]
mod tests;
