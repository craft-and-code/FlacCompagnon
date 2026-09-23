use super::*;

fn touch(path: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(path, b"data").expect("write");
}

#[test]
fn removes_the_files_the_batch_wrote() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let a = root.join("a.flac");
    let b = root.join("Disc 2/b.flac");
    touch(&a);
    touch(&b);

    let planned = vec![a.clone(), b.clone()];
    assert_eq!(undo_batch(&planned, &HashSet::new(), root), 2);
    assert!(!a.exists());
    assert!(!b.exists());
}

/// The case this module's `preexisting` argument exists for: converting
/// into a folder that already holds an earlier run's output, then
/// cancelling, must not take that earlier output down with it.
#[test]
fn leaves_files_that_were_already_there_before_the_batch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let old = root.join("old.flac");
    let fresh = root.join("fresh.flac");
    touch(&old);
    touch(&fresh);

    let planned = vec![old.clone(), fresh.clone()];
    let preexisting: HashSet<PathBuf> = [old.clone()].into_iter().collect();
    assert_eq!(undo_batch(&planned, &preexisting, root), 1);
    assert!(
        old.exists(),
        "a file that predates the batch must survive it"
    );
    assert!(!fresh.exists());
}

/// A planned destination that never got written (the batch was cancelled
/// before reaching it) is simply not there — that's the normal case for
/// most of the list, not an error.
#[test]
fn a_destination_that_was_never_written_is_not_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let never = root.join("never.flac");

    assert_eq!(undo_batch(&[never], &HashSet::new(), root), 0);
}

/// Subfolders the batch created to mirror the source layout go too, so a
/// cancelled run doesn't leave an empty skeleton of the album's folder
/// structure behind.
#[test]
fn prunes_folders_the_batch_created_and_then_emptied() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let nested = root.join("Album/Disc 2/b.flac");
    touch(&nested);

    undo_batch(&[nested], &HashSet::new(), root);
    assert!(!root.join("Album/Disc 2").exists());
    assert!(!root.join("Album").exists());
    assert!(root.exists(), "the output root itself is never removed");
}

/// A folder that still holds something else — a cover the user had put
/// there, a file from an earlier run — must survive, along with every
/// ancestor above it.
#[test]
fn keeps_a_folder_that_still_has_something_else_in_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let converted = root.join("Album/b.flac");
    let cover = root.join("Album/cover.jpg");
    touch(&converted);
    touch(&cover);

    undo_batch(&[converted], &HashSet::new(), root);
    assert!(cover.exists());
    assert!(root.join("Album").exists());
}

/// Defence in depth against a bad `output_root`/`planned` pairing: this
/// deletes files, so a path outside the folder the user picked is never
/// touched, whatever the caller passed.
#[test]
fn never_touches_anything_outside_the_output_root() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("out");
    std::fs::create_dir_all(&root).expect("mkdir");
    let outside = dir.path().join("elsewhere/keep.flac");
    touch(&outside);

    assert_eq!(
        undo_batch(std::slice::from_ref(&outside), &HashSet::new(), &root),
        0
    );
    assert!(outside.exists());
}
