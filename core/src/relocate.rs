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
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One file's move: where the listing thought it was, and where it is now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relocation {
    /// The stale path, exactly as the listing holds it.
    pub from: String,
    /// The path found under the folder the user chose.
    pub to: String,
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
/// tests below work on paths that never existed.
pub fn match_moved_files(missing: &[String], candidates: &[PathBuf]) -> Vec<Relocation> {
    // Indexed by file name: the one part of a path a move cannot change.
    let mut by_name: HashMap<String, Vec<&PathBuf>> = HashMap::new();
    for path in candidates {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            by_name.entry(name.to_string()).or_default().push(path);
        }
    }

    let mut out = Vec::new();
    for from in missing {
        let old = Path::new(from);
        let Some(name) = old.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(options) = by_name.get(name) else {
            continue;
        };
        // `max_by_key` returns the *last* maximum, so ties resolve to whichever
        // candidate the walk found last — arbitrary but stable, and a tie here
        // means two files that are equally plausible by every signal a path
        // carries.
        let Some(best) = options.iter().max_by_key(|c| shared_suffix(old, c)) else {
            continue;
        };
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

/// How many trailing path components `a` and `b` have in common, file name
/// included.
///
/// Comparison is case-insensitive: the same library read from a case-sensitive
/// filesystem and a case-preserving one differs in exactly this way, and a
/// score that flipped between them would make the ranking depend on which
/// machine the report was saved on.
fn shared_suffix(a: &Path, b: &Path) -> usize {
    let parts = |p: &Path| -> Vec<String> {
        p.components()
            .filter_map(|c| match c {
                Component::Normal(s) => s.to_str().map(|s| s.to_lowercase()),
                _ => None,
            })
            .collect()
    };
    let (a, b) = (parts(a), parts(b));
    a.iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn a_moved_library_is_found_by_file_name() {
        let missing = vec!["/old/Album/01 Intro.flac".to_string()];
        let found = paths(&["/new/Album/01 Intro.flac", "/new/Album/02 Next.flac"]);
        assert_eq!(
            match_moved_files(&missing, &found),
            vec![Relocation {
                from: "/old/Album/01 Intro.flac".into(),
                to: "/new/Album/01 Intro.flac".into(),
            }]
        );
    }

    /// The case the whole suffix score exists for: a discography where the
    /// file name alone is ambiguous. Picking by name only would be a coin
    /// toss, and a wrong pick here silently repoints a row at another album's
    /// track.
    #[test]
    fn the_album_folder_breaks_a_tie_between_identical_names() {
        let missing = vec!["/old/Reise Reise/01 Intro.flac".to_string()];
        let found = paths(&[
            "/new/Mutter/01 Intro.flac",
            "/new/Reise Reise/01 Intro.flac",
            "/new/Sehnsucht/01 Intro.flac",
        ]);
        let got = match_moved_files(&missing, &found);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].to, "/new/Reise Reise/01 Intro.flac");
    }

    /// A listing assembled from several folders: pointing at one of them must
    /// fix that folder's files and leave the others untouched, rather than
    /// refusing because it cannot fix everything.
    #[test]
    fn a_partial_match_relocates_what_it_can() {
        let missing = vec![
            "/old/a/one.flac".to_string(),
            "/elsewhere/two.flac".to_string(),
        ];
        let found = paths(&["/new/a/one.flac"]);
        let got = match_moved_files(&missing, &found);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].from, "/old/a/one.flac");
    }

    #[test]
    fn case_differences_do_not_change_the_ranking() {
        let missing = vec!["/old/Reise Reise/01 Intro.flac".to_string()];
        let found = paths(&["/new/MUTTER/01 Intro.flac", "/new/reise reise/01 Intro.flac"]);
        let got = match_moved_files(&missing, &found);
        assert_eq!(got[0].to, "/new/reise reise/01 Intro.flac");
    }

    #[test]
    fn a_file_already_at_its_recorded_path_is_not_a_move() {
        let missing = vec!["/same/one.flac".to_string()];
        assert!(match_moved_files(&missing, &paths(&["/same/one.flac"])).is_empty());
    }

    #[test]
    fn nothing_to_match_is_not_an_error() {
        assert!(match_moved_files(&[], &paths(&["/new/one.flac"])).is_empty());
        assert!(match_moved_files(&["/old/one.flac".to_string()], &[]).is_empty());
    }

    #[test]
    fn shared_suffix_counts_from_the_end() {
        assert_eq!(shared_suffix(Path::new("/a/b/c.flac"), Path::new("/x/b/c.flac")), 2);
        assert_eq!(shared_suffix(Path::new("/a/b/c.flac"), Path::new("/x/y/c.flac")), 1);
        assert_eq!(shared_suffix(Path::new("/a/b/c.flac"), Path::new("/x/y/z.flac")), 0);
    }
}
