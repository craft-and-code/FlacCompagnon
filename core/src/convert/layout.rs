//! Where the output goes: one destination per converted file, and the
//! non-audio files that travelled with them.
//!
//! Split out of [`super`] because it answers a different question from the
//! rest of that module. The encoders decide *how* a file is written; this
//! decides *where*, and it is the part users notice when it is wrong — a
//! flattened album is immediately visible, a slightly different LPC order is
//! not.
//!
//! ## Why the base folder is remembered, not computed
//!
//! An earlier version derived one shared root from the file list, as the
//! deepest common ancestor of every source's parent. That is subtly not what
//! the user asked for, and the gap showed up in the most ordinary case there
//! is: drop a band folder holding a single album, and every file shares the
//! parent `Band/Album`, so the "common root" *is* `Band/Album` and stripping
//! it drops both folders — the tracks land flat in the destination, the band
//! and album folders gone. The user dropped a folder; the folder is what they
//! expect to find.
//!
//! So each source carries the base its layout is expressed against, set from
//! what was actually dropped rather than recovered afterwards. Per file, not
//! per batch, which also fixes a second-order oddity: two unrelated folders
//! no longer get forced under whatever distant ancestor they happen to share.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::ConvertFormat;

/// One source file and the folder its layout should be mirrored from.
#[derive(Debug, Clone)]
pub struct ConvertSource {
    /// The audio file to convert.
    pub path: PathBuf,
    /// The folder `path`'s position is expressed relative to — the *parent*
    /// of what was dropped, so a dropped folder is itself recreated at the
    /// destination. For a loose file this is simply its own parent, so it
    /// lands directly in the output folder.
    pub base: PathBuf,
}

/// Work out one destination path per source, reproducing each file's position
/// relative to its own [`ConvertSource::base`] under `output_root` — same
/// folders, same nesting, only the root and the extension change.
///
/// A source that (unexpectedly) doesn't live under its base falls back to
/// just its own file name directly under `output_root`, rather than failing
/// the whole batch over one path oddity: a destination that is merely flatter
/// than intended is a far smaller problem than a batch that refuses to run.
pub fn plan_batch(
    sources: &[ConvertSource],
    output_root: &Path,
    format: ConvertFormat,
) -> Vec<PathBuf> {
    sources
        .iter()
        .map(|s| {
            let rel: &Path = s.path.strip_prefix(&s.base).unwrap_or_else(|_| {
                s.path.file_name().map(Path::new).unwrap_or(s.path.as_path())
            });
            output_root.join(rel).with_extension(format.extension())
        })
        .collect()
}

/// The folders to sweep for non-audio files, paired with the base each one's
/// contents are mirrored against.
///
/// Recovered from the sources rather than passed in: a source's first path
/// component under its base *is* what was dropped. `Band/Album/01.flac`
/// relative to `/music` means `/music/Band` was dropped, so that is the
/// folder to sweep — sweeping `/music` itself would drag in every other band
/// sitting beside it.
fn sweep_roots(sources: &[ConvertSource]) -> HashSet<(PathBuf, PathBuf)> {
    sources
        .iter()
        .filter_map(|s| {
            let rel = s.path.strip_prefix(&s.base).ok()?;
            let mut comps = rel.components();
            let first = comps.next()?;
            // A single component means the file sits directly in `base` — it
            // was dropped as a loose file, so the folder to sweep is `base`.
            let root = if comps.next().is_none() {
                s.base.clone()
            } else {
                s.base.join(first)
            };
            Some((root, s.base.clone()))
        })
        .collect()
}

/// Copy every file that isn't one of the tracks being converted — covers,
/// `.m3u` playlists, generated spectrograms, anything else sharing the
/// folder — to the same relative position under `output_root`.
///
/// "Tout ou rien": there is no per-file choice, by design. The audio files in
/// `sources` are the exclusion list, so a caller cannot get it out of step
/// with what was actually converted.
///
/// ## Overlapping drops
///
/// Dropping a folder *and* something inside it means one neighbouring file is
/// reachable from two sweeps, under two different bases — and those bases give
/// it two *different* destinations (`out/Band/Album/cover.jpg` from `/music`,
/// `out/Album/cover.jpg` from `/music/Band`). So the thing that has to be
/// unique is the **source**, not the destination: deduplicating on the
/// destination lets the same file through twice, which is exactly what it did.
///
/// Which of the two destinations wins then has to be decided rather than left
/// to chance. The sweeps run outermost base first, so the deepest surviving
/// structure wins — consistent with the rest of this module: the user dropped
/// `Band`, so `Band/` is what they expect to find. Sorting also makes the
/// returned list stable, which a `HashSet` iteration order alone was not.
pub fn passthrough_files(
    sources: &[ConvertSource],
    output_root: &Path,
) -> std::io::Result<Vec<PathBuf>> {
    let audio: HashSet<&Path> = sources.iter().map(|s| s.path.as_path()).collect();
    let mut written = Vec::new();
    let mut copied: HashSet<PathBuf> = HashSet::new();

    // A path sorts after every prefix of itself, so ordering by base puts the
    // outermost (shortest) one first; the root breaks ties between two sweeps
    // sharing a base.
    let mut roots: Vec<(PathBuf, PathBuf)> = sweep_roots(sources).into_iter().collect();
    roots.sort_by(|(a_root, a_base), (b_root, b_base)| {
        a_base.cmp(b_base).then_with(|| a_root.cmp(b_root))
    });

    for (root, base) in roots {
        let walk = walkdir::WalkDir::new(&root).sort_by_file_name();
        for entry in walk.into_iter().filter_map(Result::ok) {
            let path = entry.into_path();
            if !path.is_file() || audio.contains(path.as_path()) {
                continue;
            }
            let Ok(rel) = path.strip_prefix(&base) else {
                continue;
            };
            // Only after `strip_prefix` succeeded: a file this sweep can't
            // place must stay eligible for a later one that can.
            if !copied.insert(path.clone()) {
                continue;
            }
            let dest = output_root.join(rel);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&path, &dest)?;
            written.push(dest);
        }
    }
    Ok(written)
}

#[cfg(test)]
#[path = "../../tests/unit/convert/layout.rs"]
mod tests;
