//! Finding the files worth analyzing, and analyzing a whole folder of them.

use std::path::{Path, PathBuf};

use crate::pipeline::analyze_file;
use crate::types::{AnalysisError, FileAnalysis, FolderReport, ScanOptions};

/// Audio file extensions FlacCompagnon will attempt to analyze.
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "flac", "wav", "wave", "aif", "aiff", "aifc", "alac", "m4a", "mp4", "caf", "ogg", "oga",
    "mp3", "aac", "dsf", "dff",
];

/// Folder name used for generated spectrograms. Files inside one are skipped
/// so a second scan doesn't try to analyze the app's own PNG output — and, more
/// to the point, so re-scanning a folder never grows the file list.
const GENERATED_DIR: &str = "spectres";

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
/// lives inside a generated `spectres` folder.
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
        .filter(|p| !p.components().any(|c| c.as_os_str() == GENERATED_DIR))
        .collect();
    paths.sort();
    paths
}

/// Analyze every supported audio file under `root`.
pub fn analyze_folder(root: &Path, opts: &ScanOptions) -> Result<FolderReport, AnalysisError> {
    let paths = list_audio_files(root, opts.recursive);
    let files: Vec<FileAnalysis> = paths.iter().map(|p| analyze_file(p, opts)).collect();
    let has_flac = files.iter().any(|f| f.flac_md5.is_some());

    Ok(FolderReport {
        root: root.to_string_lossy().to_string(),
        files,
        has_flac,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn touch(dir: &Path, rel: &str) {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(&p, b"").expect("write");
    }

    #[test]
    fn every_supported_extension_is_recognized() {
        for ext in SUPPORTED_EXTENSIONS {
            let name = format!("track.{ext}");
            assert!(is_supported_audio(Path::new(&name)), "{ext}");
            // Uppercase too: file systems are case-insensitive in practice.
            let upper = format!("track.{}", ext.to_uppercase());
            assert!(is_supported_audio(Path::new(&upper)), "{upper}");
        }
        assert!(!is_supported_audio(Path::new("cover.jpg")));
        assert!(!is_supported_audio(Path::new("notes.txt")));
        assert!(!is_supported_audio(Path::new("README")));
        // An extension that merely *contains* a supported one is not one.
        assert!(!is_supported_audio(Path::new("track.flacx")));
    }

    /// Generated spectrograms live next to the audio; a rescan must not pick
    /// up anything from there, at any depth.
    #[test]
    fn generated_spectres_folders_are_skipped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        touch(root, "a.flac");
        touch(root, "spectres/a.flac");
        touch(root, "album/b.flac");
        touch(root, "album/spectres/b.flac");

        let found = list_audio_files(root, true);
        assert_eq!(found.len(), 2, "found: {found:?}");
        assert!(found.iter().all(|p| !p
            .components()
            .any(|c| c.as_os_str() == GENERATED_DIR)));
    }

    #[test]
    fn non_recursive_stays_in_the_top_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        touch(root, "a.flac");
        touch(root, "album/b.flac");

        assert_eq!(list_audio_files(root, false).len(), 1);
        assert_eq!(list_audio_files(root, true).len(), 2);
    }

    /// The list is sorted, because it is what the report's row order comes
    /// from before the user reorders anything.
    #[test]
    fn results_are_sorted() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        for name in ["c.flac", "a.flac", "b.flac"] {
            touch(root, name);
        }
        let found = list_audio_files(root, false);
        let mut sorted = found.clone();
        sorted.sort();
        assert_eq!(found, sorted);
    }

    /// A folder that doesn't exist is an empty list, not a panic — the path
    /// can come from a stale saved report.
    #[test]
    fn a_missing_root_yields_nothing() {
        let missing = Path::new("/definitely/not/a/real/folder/anywhere");
        assert!(list_audio_files(missing, true).is_empty());
    }
}
