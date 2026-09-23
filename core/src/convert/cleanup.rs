//! Undoing a cancelled conversion batch.
//!
//! Cancellation here is not the same problem it is for analysis. Analysis
//! only *reads*, so abandoning it mid-run leaves nothing behind — dropping
//! the partial results is the whole cleanup. A conversion batch writes real
//! files, and the workers that were already mid-file when Cancel was pressed
//! go on to finish and write theirs (see `parallel_map_ordered`'s own
//! comment: in-flight items are allowed to complete rather than being killed
//! mid-write, which would leave a truncated file instead of no file). So
//! "cancelled" without this module means the user gets told the run was
//! cancelled while a folder of half a batch's output sits on disk — which is
//! exactly the behaviour this exists to fix.
//!
//! The one thing it must not do is delete something it didn't create. A user
//! converting into a folder that already holds a previous run's output would
//! otherwise have those earlier files removed by cancelling this one, which
//! is worse than the problem being solved. Hence `preexisting`: the caller
//! records which destination paths were already on disk *before* the batch
//! started, and only the rest are ever removed.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Delete the output files a cancelled batch created under `output_root`,
/// then remove any folders that creating them had left empty. `planned` is
/// every destination the batch was going to write (see
/// [`super::plan_batch`]); `preexisting` is the subset of those that already
/// existed before it started, which are left exactly as they were.
///
/// Returns how many files were actually removed. Errors are deliberately not
/// propagated: this runs while unwinding a cancelled batch, where a file that
/// can't be deleted (permissions, a lock, something removed it already) is
/// worth neither failing the cancellation over nor reporting on top of it.
pub fn undo_batch(planned: &[PathBuf], preexisting: &HashSet<PathBuf>, output_root: &Path) -> usize {
    let mut removed = 0usize;
    let mut parents: HashSet<PathBuf> = HashSet::new();

    for dest in planned {
        if preexisting.contains(dest) || !dest.starts_with(output_root) || !dest.is_file() {
            continue;
        }
        if std::fs::remove_file(dest).is_ok() {
            removed += 1;
            if let Some(parent) = dest.parent() {
                parents.insert(parent.to_path_buf());
            }
        }
    }

    // Deepest first, so a nested folder is gone before its parent is tried —
    // otherwise the parent still looks non-empty and survives a run that
    // emptied it.
    let mut parents: Vec<PathBuf> = parents.into_iter().collect();
    parents.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for parent in parents {
        prune_empty_dirs(&parent, output_root);
    }

    removed
}

/// Remove `dir` and each of its ancestors up to (but never including)
/// `output_root`, stopping at the first one that isn't empty.
/// `std::fs::remove_dir` refuses to remove a non-empty folder, so its failure
/// *is* the emptiness test — no separate `read_dir` race between checking and
/// removing.
fn prune_empty_dirs(mut dir: &Path, output_root: &Path) {
    while dir.starts_with(output_root) && dir != output_root {
        if std::fs::remove_dir(dir).is_err() {
            return;
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => return,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/convert/cleanup.rs"]
mod tests;
