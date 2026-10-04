use super::*;

fn src(path: &str, base: &str) -> ConvertSource {
    ConvertSource {
        path: PathBuf::from(path),
        base: PathBuf::from(base),
    }
}

#[test]
fn mirrors_the_source_structure_under_the_new_root() {
    let sources = vec![
        src("/music/Album/01 track.mp3", "/music"),
        src("/music/Album/Disc 2/02 track.mp3", "/music"),
    ];
    let got =
        plan_batch(&sources, Path::new("/export"), ConvertFormat::Flac).expect("valid output plan");
    assert_eq!(
        got,
        vec![
            PathBuf::from("/export/Album/01 track.flac"),
            PathBuf::from("/export/Album/Disc 2/02 track.flac"),
        ]
    );
}

/// The regression this whole shape exists for. A band folder holding
/// exactly one album gives every file the same parent, so a base derived
/// from the file list would be `/music/Band/Album` and both folders would
/// vanish — which is precisely what users saw: tracks dumped flat into
/// the destination.
#[test]
fn a_dropped_folder_survives_even_when_it_holds_a_single_album() {
    let sources = vec![
        src("/music/Band/Album/01.flac", "/music"),
        src("/music/Band/Album/02.flac", "/music"),
    ];
    let got =
        plan_batch(&sources, Path::new("/export"), ConvertFormat::Opus).expect("valid output plan");
    assert_eq!(
        got,
        vec![
            PathBuf::from("/export/Band/Album/01.opus"),
            PathBuf::from("/export/Band/Album/02.opus"),
        ]
    );
}

/// Two unrelated folders keep their own names instead of being forced
/// under whatever distant ancestor they share — with one shared base
/// these would have come out as `music/A/...` and `elsewhere/B/...`.
#[test]
fn unrelated_drops_do_not_grow_a_shared_prefix() {
    let sources = vec![
        src("/music/A/01.flac", "/music"),
        src("/elsewhere/B/01.flac", "/elsewhere"),
    ];
    let got =
        plan_batch(&sources, Path::new("/export"), ConvertFormat::Mp3).expect("valid output plan");
    assert_eq!(
        got,
        vec![
            PathBuf::from("/export/A/01.mp3"),
            PathBuf::from("/export/B/01.mp3"),
        ]
    );
}

/// Loose files mirror nothing: their base is their own parent, so they
/// land directly in the destination.
#[test]
fn dropped_files_land_flat_in_the_destination() {
    let sources = vec![src("/music/Album/01 track.flac", "/music/Album")];
    let got =
        plan_batch(&sources, Path::new("/export"), ConvertFormat::Wav).expect("valid output plan");
    assert_eq!(got, vec![PathBuf::from("/export/01 track.wav")]);
}

#[test]
fn falls_back_to_the_file_name_for_a_path_outside_its_base() {
    let sources = vec![src("/elsewhere/loose.wav", "/music/Album")];
    let got =
        plan_batch(&sources, Path::new("/export"), ConvertFormat::Wav).expect("valid output plan");
    assert_eq!(got, vec![PathBuf::from("/export/loose.wav")]);
}

#[test]
fn sweep_root_is_the_dropped_folder_not_its_parent() {
    let sources = vec![src("/music/Band/Album/01.flac", "/music")];
    let roots = sweep_roots(&sources);
    assert_eq!(
        roots.into_iter().collect::<Vec<_>>(),
        vec![(PathBuf::from("/music/Band"), PathBuf::from("/music"))]
    );
}

#[test]
fn sweep_root_of_a_loose_file_is_its_own_folder() {
    let sources = vec![src("/music/Album/01.flac", "/music/Album")];
    let roots = sweep_roots(&sources);
    assert_eq!(
        roots.into_iter().collect::<Vec<_>>(),
        vec![(PathBuf::from("/music/Album"), PathBuf::from("/music/Album"))]
    );
}

#[test]
fn passthrough_copies_everything_except_the_converted_tracks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let music = dir.path().join("music");
    let out = dir.path().join("out");
    std::fs::create_dir_all(music.join("Band/Album/Disc 1")).expect("mkdir");
    for rel in [
        "Band/Album/track.mp3",
        "Band/Album/cover.jpg",
        "Band/Album/Disc 1/playlist.m3u",
    ] {
        std::fs::write(music.join(rel), b"x").expect("write");
    }

    let sources = vec![ConvertSource {
        path: music.join("Band/Album/track.mp3"),
        base: music.clone(),
    }];
    let written = passthrough_files(&sources, &out).expect("copy");

    assert_eq!(written.len(), 2, "{written:?}");
    assert!(out.join("Band/Album/cover.jpg").exists());
    assert!(out.join("Band/Album/Disc 1/playlist.m3u").exists());
    assert!(
        !out.join("Band/Album/track.mp3").exists(),
        "the audio is written by the encoder, not copied"
    );
}

/// A folder dropped alongside something inside it must not copy the same
/// neighbour twice.
///
/// The symptom that caught the first attempt: deduplicating on the
/// *destination* let the cover through twice, because the two bases give
/// it two different destinations (`out/Band/Album/cover.jpg` and
/// `out/Album/cover.jpg`) — so the set never saw a collision. Hence the
/// assertion on where the single copy lands, not just on how many there
/// are: a count alone would also pass if the arbitrary winner changed
/// between runs.
#[test]
fn overlapping_drops_copy_each_file_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let music = dir.path().join("music");
    let out = dir.path().join("out");
    std::fs::create_dir_all(music.join("Band/Album")).expect("mkdir");
    std::fs::write(music.join("Band/Album/a.flac"), b"x").expect("write");
    std::fs::write(music.join("Band/Album/cover.jpg"), b"x").expect("write");

    // Same file reached through two different drops of the same base.
    let sources = vec![
        ConvertSource {
            path: music.join("Band/Album/a.flac"),
            base: music.clone(),
        },
        ConvertSource {
            path: music.join("Band/Album/a.flac"),
            base: music.join("Band"),
        },
    ];
    let written = passthrough_files(&sources, &out).expect("copy");
    let covers: Vec<_> = written
        .iter()
        .filter(|p| p.file_name().is_some_and(|n| n == "cover.jpg"))
        .collect();
    assert_eq!(covers.len(), 1, "{written:?}");
    // The outermost base wins, so the dropped `Band` folder survives.
    assert_eq!(covers[0], &out.join("Band/Album/cover.jpg"));
    assert!(!out.join("Album/cover.jpg").exists());
}

#[test]
fn parent_traversal_cannot_escape_the_output_root() {
    let sources = vec![src("/music/../outside.wav", "/music")];
    assert!(plan_batch(&sources, Path::new("/export"), ConvertFormat::Flac).is_err());
}

#[test]
fn sources_with_the_same_output_name_are_rejected() {
    let sources = vec![
        src("/music/track.wav", "/music"),
        src("/music/track.mp3", "/music"),
    ];
    assert!(plan_batch(&sources, Path::new("/export"), ConvertFormat::Flac).is_err());
}

#[test]
fn passthrough_does_not_recurse_into_output_inside_input() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    let output = music.join("export");
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(music.join("track.wav"), b"audio").unwrap();
    std::fs::write(music.join("cover.jpg"), b"cover").unwrap();
    std::fs::write(output.join("earlier.txt"), b"prior output").unwrap();
    let sources = vec![ConvertSource {
        path: music.join("track.wav"),
        base: music,
    }];
    assert_eq!(
        passthrough_files(&sources, &output).unwrap(),
        vec![output.join("cover.jpg")]
    );
    assert!(!output.join("export").exists());
}

#[test]
fn passthrough_cannot_overwrite_a_converted_track() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    let output = dir.path().join("output");
    std::fs::create_dir_all(&music).unwrap();
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(music.join("track.wav"), b"source").unwrap();
    std::fs::write(music.join("track.flac"), b"neighbour").unwrap();
    std::fs::write(output.join("track.flac"), b"converted audio").unwrap();
    let sources = vec![ConvertSource {
        path: music.join("track.wav"),
        base: music,
    }];
    assert!(passthrough_files(&sources, &output).unwrap().is_empty());
    assert_eq!(
        std::fs::read(output.join("track.flac")).unwrap(),
        b"converted audio"
    );
}

#[cfg(unix)]
#[test]
fn conversion_output_cannot_escape_through_a_descendant_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("output");
    let outside = dir.path().join("outside");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, out.join("Album")).unwrap();
    let sources = vec![src("/music/Album/track.wav", "/music")];
    assert!(plan_batch(&sources, &out, ConvertFormat::Flac).is_err());
}

#[cfg(unix)]
#[test]
fn passthrough_output_cannot_escape_through_a_descendant_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    let out = dir.path().join("output");
    let outside = dir.path().join("outside");
    std::fs::create_dir_all(music.join("Album")).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(music.join("Album/track.wav"), b"audio").unwrap();
    std::fs::write(music.join("Album/cover.png"), b"cover").unwrap();
    std::os::unix::fs::symlink(&outside, out.join("Album")).unwrap();
    let sources = vec![ConvertSource {
        path: music.join("Album/track.wav"),
        base: music,
    }];
    assert!(passthrough_files(&sources, &out).is_err());
    assert!(!outside.join("cover.png").exists());
}
