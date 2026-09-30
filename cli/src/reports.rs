//! Plan album reports and reuse successful measurements of unchanged files.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::args::Args;
use flaccompagnon_core::{self as core, FileAnalysis};

type Group = (PathBuf, Vec<PathBuf>, Option<PathBuf>);

fn album_folder(path: &Path) -> PathBuf {
    let folder = path.parent().unwrap_or(Path::new("."));
    let name = folder
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let disc = ["cd", "disc", "disk"].iter().any(|prefix| {
        name.strip_prefix(prefix).is_some_and(|rest| {
            let rest = rest.trim_start_matches([' ', '-', '_']);
            !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
        })
    });
    if disc {
        folder.parent().unwrap_or(folder).to_path_buf()
    } else {
        folder.to_path_buf()
    }
}

pub(crate) fn groups(paths: &[PathBuf], args: &Args) -> Vec<Group> {
    if args.json_layout.is_none() {
        return vec![(
            PathBuf::from(core::display_root(&args.targets)),
            paths.to_vec(),
            args.json.clone(),
        )];
    }
    let mut albums: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for path in paths {
        albums
            .entry(album_folder(path))
            .or_default()
            .push(path.clone());
    }
    albums
        .into_iter()
        .map(|(folder, paths)| {
            let stem = folder.file_name().unwrap_or_default().to_string_lossy();
            let parent = if args.json_layout.as_deref() == Some("artist") {
                folder.parent().unwrap_or(&folder)
            } else {
                &folder
            };
            let destination = parent.join(format!("{stem}.json"));
            (folder, paths, Some(destination))
        })
        .collect()
}

pub(crate) fn matches(file: &FileAnalysis, path: &Path) -> bool {
    if file.error.is_some() {
        return false;
    }
    let same = Path::new(&file.path) == path
        || std::fs::canonicalize(&file.path)
            .ok()
            .zip(std::fs::canonicalize(path).ok())
            .is_some_and(|(a, b)| a == b);
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|t| i64::try_from(t.as_secs()).ok());
    same && file.size_bytes == metadata.len()
        && file.modified_unix.is_some()
        && file.modified_unix == modified
}

pub(crate) fn cached(
    paths: &[PathBuf],
    destinations: &[&Path],
    args: &Args,
) -> Result<BTreeMap<PathBuf, FileAnalysis>, String> {
    if args.force {
        return Ok(BTreeMap::new());
    }
    let mut candidates = Vec::new();
    // A report may sit beside its album or one level above it in the artist folder.
    let mut visited = BTreeSet::new();
    for path in paths {
        let album = album_folder(path);
        for dir in [Some(album.as_path()), album.parent()]
            .into_iter()
            .flatten()
        {
            if !visited.insert(dir.to_path_buf()) {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                candidates.extend(
                    entries
                        .filter_map(Result::ok)
                        .map(|e| e.path())
                        .filter(|p| p.extension().is_some_and(|e| e == "json")),
                );
            }
        }
    }
    candidates.extend(
        destinations
            .iter()
            .filter(|p| p.exists())
            .map(|p| p.to_path_buf()),
    );
    candidates.sort();
    candidates.dedup();
    // Prefer the newest snapshot when several reports cover the same file.
    candidates.sort_by_key(|p| p.metadata().and_then(|m| m.modified()).ok());
    let mut files = BTreeMap::new();
    for candidate in candidates {
        let report = std::fs::read_to_string(&candidate)
            .map_err(|e| e.to_string())
            .and_then(|text| core::report::parse_json(&text));
        match report {
            Ok(report) => {
                for file in report.files {
                    let key = path_key(Path::new(&file.path));
                    files.insert(key, file);
                }
            }
            Err(error) if destinations.contains(&candidate.as_path()) => {
                return Err(format!(
                    "{}: {error}; use --force to replace it",
                    candidate.display()
                ))
            }
            Err(_) => {}
        }
    }
    Ok(files)
}

// Canonical keys let relative CLI targets reuse reports containing absolute paths.
pub(crate) fn path_key(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
