//! Finding files that moved: matching a listing's stale paths against a folder
//! the user points at.
//!
//! # Why by file name, and why a suffix score
//!
//! A saved report records absolute paths. Move the library, restore it from a
//! backup, plug the drive in on another machine, and every path in the report
//! is wrong while every *file* is fine. The information that survives all of
//! those is the file name and the tail of its folder structure.
//!
//! Matching on the name alone is not enough: a discography holds a dozen
//! `01 Intro.flac`. So candidates are ranked by how many trailing path
//! components they share with the old path — `Rammstein/Reise/01 Intro.flac`
//! beats `Various/Hits/01 Intro.flac` when relocating the former, and no
//! shared parent at all still matches when there is only one candidate.
//!
//! This is deliberately per-file rather than a single "old prefix → new
//! prefix" rewrite. A listing assembled from several folders — the mixtape
//! case — has no common prefix to rewrite, but each of its files can still be
//! found on its own.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One file's move: where the listing thought it was, and where it is now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relocation {
    /// The stale path, exactly as the listing holds it.
    pub from: String,
    /// The path found under the folder the user chose.
    pub to: String,
}

struct Candidate<'a> {
    path: &'a Path,
    parts: Vec<String>,
}

/// Match each of `missing` against the files under `root`.
///
/// Only unambiguous, same-name matches are returned; a file with no candidate
/// is simply absent from the result, and the caller reports how many were
/// found rather than failing the whole operation. That matters for a listing
/// spanning several folders: pointing at one of them should fix that folder's
/// files and leave the rest alone.
///
/// `candidates` is every path under the chosen folder — taking it as an
/// argument rather than walking the disk here keeps this crate free of the
/// directory traversal (and the traversal's error handling), and lets the
/// unit tests work on paths that never existed.
pub fn match_moved_files(missing: &[String], candidates: &[PathBuf]) -> Vec<Relocation> {
    // Indexed by file name: the one part of a path a move cannot change.
    let mut by_name: HashMap<String, Vec<Candidate<'_>>> = HashMap::new();
    for path in candidates {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            by_name
                .entry(name.to_lowercase())
                .or_default()
                .push(Candidate {
                    path,
                    parts: path_parts(path),
                });
        }
    }

    let mut out = Vec::new();
    for from in missing {
        let old = Path::new(from);
        let Some(name) = from.rsplit(['/', '\\']).next().filter(|s| !s.is_empty()) else {
            continue;
        };
        let Some(options) = by_name.get(&name.to_lowercase()) else {
            continue;
        };
        let old_parts = path_parts(old);
        let mut best: Option<(&Path, usize)> = None;
        let mut tied = false;
        for candidate in options {
            let score = shared_suffix(&old_parts, &candidate.parts);
            match best {
                Some((_, current)) if score < current => {}
                Some((_, current)) if score == current => tied = true,
                _ => {
                    best = Some((candidate.path, score));
                    tied = false;
                }
            }
        }
        let Some((best, _)) = best else {
            continue;
        };
        // A tied folder suffix carries no evidence for choosing one track
        // over another. Leave the row missing rather than silently changing
        // its playback and tag-editing target to an unrelated album.
        if tied {
            continue;
        }
        if let Some(to) = best.to_str() {
            // A candidate identical to the stale path is not a move. This can
            // only happen if the file came back between the presence check and
            // the folder being chosen, and reporting it as relocated would be
            // a lie in the toast.
            if to != from {
                out.push(Relocation {
                    from: from.clone(),
                    to: to.to_string(),
                });
            }
        }
    }
    out
}

// Reports can come from another OS, so both separator styles are accepted;
// case folding is cached once per candidate rather than per missing track.
fn path_parts(path: &Path) -> Vec<String> {
    path.to_string_lossy()
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Count matching trailing components, including the file name.
fn shared_suffix(a: &[String], b: &[String]) -> usize {
    a.iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count()
}

#[cfg(test)]
#[path = "../tests/unit/relocate.rs"]
mod tests;
