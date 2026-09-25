//! Machinery shared by the long-running batch commands (analysis and
//! spectrogram rendering): the cancellation flag, the progress event payload,
//! and turning what the user dropped into an actual list of audio files.
//!
//! It lives apart from both because it belongs to neither: putting it in
//! `analysis` would make `spectrograms` import from a sibling for no reason
//! other than history.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;

use serde::Serialize;

pub(crate) use flaccompagnon_core::{display_root, gather_targets};

/// Set when the user requests cancellation of the in-progress batch. Only one
/// long-running operation runs at a time (the UI enforces this), so a single
/// global flag is sufficient.
pub(crate) static CANCEL: AtomicBool = AtomicBool::new(false);

/// Clear the flag at the start of a batch. Without this, a batch started
/// right after a cancelled one would see the stale `true` and stop at once.
pub(crate) fn reset_cancel() {
    CANCEL.store(false, Ordering::SeqCst);
}

pub(crate) fn cancelled() -> bool {
    CANCEL.load(Ordering::SeqCst)
}

/// Progress event payload emitted during long-running operations.
#[derive(Clone, Serialize)]
pub(crate) struct Progress {
    pub current: usize,
    pub total: usize,
    pub file: String,
}

/// Request cancellation of the running analysis / spectrogram batch. The loops
/// check this between files and stop at the next boundary.
#[tauri::command]
pub fn cancel_task() {
    CANCEL.store(true, Ordering::SeqCst);
}

/// How many worker threads to run over `total` items: one per CPU core minus
/// one (so the UI thread still gets scheduled on a busy machine), at least
/// one, and never more than there are items to do.
fn worker_count(total: usize) -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .saturating_sub(1)
        .max(1)
        .min(total)
}

/// Apply `work` to every item on a pool of threads, returning the results **in
/// the input order** — not in the order the workers happened to finish.
///
/// That ordering is the point. Analysis is CPU-bound and wildly uneven (a
/// 3-minute MP3 next to a 20-minute hi-res FLAC), so results come back
/// scrambled; but the order here becomes the order of the results table, and
/// therefore of every CSV, JSON and M3U export. Each worker writes into its
/// own indexed slot instead of pushing onto a shared list, which is what keeps
/// the two apart.
///
/// Returns `None` when the run was cancelled — that is, when any slot was left
/// unfilled. `is_cancelled` is a parameter rather than a read of [`CANCEL`] so
/// this can be tested without touching process-global state.
pub(crate) fn parallel_map_ordered<T, R, C, F, P>(
    items: &[T],
    is_cancelled: C,
    work: F,
    on_progress: P,
) -> Option<Vec<R>>
where
    T: Sync,
    R: Send + Sync,
    C: Fn() -> bool + Sync,
    F: Fn(&T) -> R + Sync,
    // `done` is 1-based: how many items have finished, this one included.
    P: Fn(usize, &T) + Sync,
{
    let total = items.len();
    if total == 0 {
        return Some(Vec::new());
    }

    // Workers pull the next index off a shared counter rather than being handed
    // a fixed slice each: with per-file times varying by an order of magnitude,
    // a static split would leave most threads idle waiting for one straggler.
    let next = AtomicUsize::new(0);
    let completed = AtomicUsize::new(0);
    let slots: Vec<OnceLock<R>> = (0..total).map(|_| OnceLock::new()).collect();

    std::thread::scope(|s| {
        for _ in 0..worker_count(total) {
            s.spawn(|| loop {
                if is_cancelled() {
                    break; // stop pulling new work; in-flight items still finish
                }
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= total {
                    break;
                }
                let _ = slots[i].set(work(&items[i]));
                let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
                on_progress(done, &items[i]);
            });
        }
    });

    // Two checks, deliberately. The first honours the user's intent: pressing
    // Cancel means "throw this away", even in the race where the last item
    // finished a moment before the click landed. The second is structural — a
    // short result set means a worker stopped early, and a half-finished batch
    // must never be presented as a complete report.
    if is_cancelled() {
        return None;
    }
    let out: Vec<R> = slots.into_iter().filter_map(OnceLock::into_inner).collect();
    (out.len() == total).then_some(out)
}

#[cfg(test)]
#[path = "../../tests/unit/commands/batch.rs"]
mod tests;
