use super::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Create an empty file (and any missing parent). Empty is fine: nothing
/// here decodes anything — `is_supported_audio` only looks at the
/// extension, and that is exactly the behaviour under test.
fn touch(root: &Path, rel: &str) -> PathBuf {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(&p, b"").expect("write");
    p
}

fn s(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

#[test]
fn a_dropped_file_is_kept_only_if_it_is_audio() {
    let dir = tempfile::tempdir().expect("tempdir");
    let song = touch(dir.path(), "song.flac");
    let cover = touch(dir.path(), "cover.jpg");

    assert_eq!(gather_targets(&[s(&song)], true), vec![song.clone()]);
    assert!(gather_targets(&[s(&cover)], true).is_empty());
}

#[test]
fn a_dropped_folder_expands_to_its_audio_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    touch(dir.path(), "a.flac");
    touch(dir.path(), "b.wav");
    touch(dir.path(), "notes.txt");
    touch(dir.path(), "disc2/c.flac");

    assert_eq!(gather_targets(&[s(dir.path())], true).len(), 3);
    assert_eq!(gather_targets(&[s(dir.path())], false).len(), 2);
}

/// Dropping a folder *and* one of the files inside it is an ordinary
/// gesture (rubber-band selection in Finder). The track must be analyzed
/// once, not twice — a duplicate row would also be exported twice.
#[test]
fn a_file_dropped_alongside_its_own_folder_is_not_analyzed_twice() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = touch(dir.path(), "a.flac");
    touch(dir.path(), "b.flac");

    let got = gather_targets(&[s(dir.path()), s(&a)], true);
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(got.iter().filter(|p| **p == a).count(), 1);
    // The same file listed twice collapses just as well.
    assert_eq!(gather_targets(&[s(&a), s(&a)], true), vec![a]);
}

/// The order here is what the results table shows before the user drags
/// anything, and therefore what a CSV/JSON export contains.
#[test]
fn output_is_sorted_regardless_of_drop_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    let c = touch(dir.path(), "c.flac");
    let a = touch(dir.path(), "a.flac");
    let b = touch(dir.path(), "b.flac");

    let got = gather_targets(&[s(&c), s(&a), s(&b)], true);
    let mut expected = vec![a, b, c];
    expected.sort();
    assert_eq!(got, expected);
}

/// Paths can arrive stale — from a saved `.json` report whose files have
/// since moved. A missing one is skipped, and must not take the rest of
/// the batch down with it.
#[test]
fn missing_paths_are_skipped_not_fatal() {
    let dir = tempfile::tempdir().expect("tempdir");
    let real = touch(dir.path(), "real.flac");
    let gone = dir.path().join("gone.flac");

    assert_eq!(gather_targets(&[s(&gone), s(&real)], true), vec![real]);
    assert!(gather_targets(&[s(&gone)], true).is_empty());
    assert!(gather_targets(&[], true).is_empty());
}

/// The whole reason for the indexed slots: results must come back in input
/// order even when the workers finish in the opposite one. The sleep is
/// inverted against the index so the *first* item is the *last* to finish —
/// a version that pushed onto a shared list would return this reversed.
#[test]
fn results_keep_input_order_even_when_workers_finish_backwards() {
    let items: Vec<usize> = (0..8).collect();
    let got = parallel_map_ordered(
        &items,
        || false,
        |i| {
            std::thread::sleep(std::time::Duration::from_millis((8 - *i as u64) * 5));
            i * 10
        },
        |_, _| {},
    )
    .expect("not cancelled");
    assert_eq!(got, vec![0, 10, 20, 30, 40, 50, 60, 70]);
}

/// Every item is processed exactly once. Two workers grabbing the same
/// index would analyze a file twice and — worse — leave another one never
/// analyzed, which the slot check would then report as a cancellation.
#[test]
fn every_item_is_processed_exactly_once() {
    let items: Vec<usize> = (0..500).collect();
    let calls = AtomicUsize::new(0);
    let got = parallel_map_ordered(
        &items,
        || false,
        |i| {
            calls.fetch_add(1, Ordering::SeqCst);
            *i
        },
        |_, _| {},
    )
    .expect("not cancelled");
    assert_eq!(calls.load(Ordering::SeqCst), 500);
    assert_eq!(got, items);
}

/// Progress fires once per item, and its `done` count runs 1..=total with
/// no gaps and no repeats — that count drives the progress bar, so a
/// duplicate would make it stall and a gap would make it jump.
#[test]
fn progress_is_reported_once_per_item_with_a_complete_count() {
    let items: Vec<usize> = (0..64).collect();
    let seen = std::sync::Mutex::new(Vec::new());
    parallel_map_ordered(
        &items,
        || false,
        |i| *i,
        |done, _| seen.lock().expect("not poisoned").push(done),
    )
    .expect("not cancelled");

    let mut seen = seen.into_inner().expect("not poisoned");
    seen.sort_unstable();
    assert_eq!(seen, (1..=64).collect::<Vec<_>>());
}

/// A cancelled run must not come back as a short-but-successful report:
/// exporting it would silently drop every file that never got analyzed.
#[test]
fn a_cancelled_run_yields_none_rather_than_partial_results() {
    let items: Vec<usize> = (0..16).collect();
    let got = parallel_map_ordered(&items, || true, |i| *i, |_, _| {});
    assert_eq!(got, None);
}

#[test]
fn an_empty_batch_succeeds_with_nothing() {
    let items: Vec<usize> = Vec::new();
    assert_eq!(
        parallel_map_ordered(&items, || false, |i| *i, |_, _| {}),
        Some(Vec::new())
    );
    // Even "cancelled", since there was nothing to leave unfinished.
    assert_eq!(
        parallel_map_ordered(&items, || true, |i| *i, |_, _| {}),
        Some(Vec::new())
    );
}

/// Never zero workers (nothing would run), never more than there is work
/// (idle threads spawned for nothing).
#[test]
fn worker_count_stays_between_one_and_the_item_count() {
    for total in 1..=64usize {
        let n = worker_count(total);
        assert!(n >= 1, "total={total} gave {n}");
        assert!(n <= total, "total={total} gave {n}");
    }
}

#[test]
fn display_root_is_the_folder_itself_for_a_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(display_root(&[s(dir.path())]), s(dir.path()));
}

#[test]
fn display_root_is_the_parent_for_a_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let song = touch(dir.path(), "song.flac");
    assert_eq!(display_root(&[s(&song)]), s(dir.path()));
}

/// Only the first target decides, by design: a mixed drop has no single
/// root, and the first item is the one the user aimed at.
#[test]
fn display_root_follows_the_first_target_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    let other = tempfile::tempdir().expect("tempdir");
    let song = touch(other.path(), "song.flac");

    assert_eq!(
        display_root(&[s(dir.path()), s(&song)]),
        s(dir.path()),
        "the folder came first"
    );
    assert_eq!(
        display_root(&[s(&song), s(dir.path())]),
        s(other.path()),
        "the file came first, so its parent wins"
    );
    assert_eq!(display_root(&[]), "");
}
