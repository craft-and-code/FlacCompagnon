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
    let found = paths(&[
        "/new/MUTTER/01 Intro.flac",
        "/new/reise reise/01 Intro.flac",
    ]);
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
    assert_eq!(
        shared_suffix(Path::new("/a/b/c.flac"), Path::new("/x/b/c.flac")),
        2
    );
    assert_eq!(
        shared_suffix(Path::new("/a/b/c.flac"), Path::new("/x/y/c.flac")),
        1
    );
    assert_eq!(
        shared_suffix(Path::new("/a/b/c.flac"), Path::new("/x/y/z.flac")),
        0
    );
}
