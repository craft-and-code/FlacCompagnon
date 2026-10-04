use super::*;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[test]
fn exclusive_rename_preserves_file_contents_and_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("old.flac");
    let destination = dir.path().join("new.flac");
    fs::write(&source, b"original audio").unwrap();
    let modified = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_500_000_000);
    fs::File::open(&source)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&source, fs::Permissions::from_mode(0o640)).unwrap();
    }
    let before = fs::metadata(&source).unwrap();
    rename_file_noclobber(&source, &destination).unwrap();
    let after = fs::metadata(&destination).unwrap();
    assert!(!source.exists());
    assert_eq!(fs::read(destination).unwrap(), b"original audio");
    assert_eq!(after.len(), before.len());
    assert_eq!(after.modified().unwrap(), before.modified().unwrap());
    assert_eq!(
        after.permissions().readonly(),
        before.permissions().readonly()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(after.ino(), before.ino());
        assert_eq!(after.mode(), before.mode());
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[test]
fn an_existing_destination_never_replaces_either_audio_file() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.flac");
    let destination = dir.path().join("existing.flac");
    fs::write(&source, b"source audio").unwrap();
    fs::write(&destination, b"existing audio").unwrap();
    let error = rename_file_noclobber(&source, &destination).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read(source).unwrap(), b"source audio");
    assert_eq!(fs::read(destination).unwrap(), b"existing audio");
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[test]
fn concurrent_renames_to_one_destination_keep_every_losing_source() {
    use std::sync::{Arc, Barrier};

    let dir = tempfile::tempdir().unwrap();
    let sources: Vec<_> = (0..4)
        .map(|i| dir.path().join(format!("{i}.flac")))
        .collect();
    for (i, source) in sources.iter().enumerate() {
        fs::write(source, format!("audio {i}")).unwrap();
    }
    let destination = dir.path().join("winner.flac");
    let barrier = Arc::new(Barrier::new(sources.len()));
    let results = std::thread::scope(|scope| {
        let workers: Vec<_> = sources
            .iter()
            .map(|source| {
                let barrier = Arc::clone(&barrier);
                let destination = &destination;
                scope.spawn(move || {
                    barrier.wait();
                    rename_file_noclobber(source, destination)
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    for (i, (result, source)) in results.iter().zip(&sources).enumerate() {
        if let Err(error) = result {
            assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
            assert_eq!(fs::read(source).unwrap(), format!("audio {i}").as_bytes());
        } else {
            assert!(!source.exists());
            assert_eq!(
                fs::read(&destination).unwrap(),
                format!("audio {i}").as_bytes()
            );
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[test]
fn concurrent_renames_of_one_source_move_it_exactly_once() {
    use std::sync::Barrier;

    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.flac");
    let destinations = [dir.path().join("a.flac"), dir.path().join("b.flac")];
    fs::write(&source, b"original audio").unwrap();
    let barrier = Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let workers: Vec<_> = destinations
            .iter()
            .map(|destination| {
                let source = &source;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    rename_file_noclobber(source, destination)
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(!source.exists());
    assert_eq!(destinations.iter().filter(|path| path.exists()).count(), 1);
    for destination in destinations.iter().filter(|path| path.exists()) {
        assert_eq!(fs::read(destination).unwrap(), b"original audio");
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn a_dangling_destination_link_is_not_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.flac");
    let destination = dir.path().join("linked.flac");
    let missing = dir.path().join("missing.flac");
    fs::write(&source, b"original audio").unwrap();
    std::os::unix::fs::symlink(&missing, &destination).unwrap();
    let error = rename_file_noclobber(&source, &destination).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read(source).unwrap(), b"original audio");
    assert_eq!(fs::read_link(destination).unwrap(), missing);
}

#[cfg(unix)]
#[test]
fn a_source_link_cannot_rename_its_target() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("linked.flac");
    let target = dir.path().join("target.flac");
    let destination = dir.path().join("new.flac");
    fs::write(&target, b"original audio").unwrap();
    std::os::unix::fs::symlink(&target, &source).unwrap();
    assert_eq!(
        rename_file_noclobber(&source, &destination)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(fs::read(target).unwrap(), b"original audio");
    assert!(fs::symlink_metadata(source)
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(!destination.exists());
}

#[test]
fn directory_sources_and_moves_out_of_the_folder_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("album");
    fs::create_dir(&folder).unwrap();
    assert_eq!(
        rename_file_noclobber(&folder, &dir.path().join("new-album"))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    let source = dir.path().join("track.flac");
    fs::write(&source, b"original audio").unwrap();
    assert_eq!(
        rename_file_noclobber(&source, &folder.join("new.flac"))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(fs::read(source).unwrap(), b"original audio");
}

#[cfg(windows)]
#[test]
fn exclusive_rename_handles_windows_paths_longer_than_max_path() {
    use std::os::windows::ffi::OsStrExt;

    let dir = tempfile::tempdir().unwrap();
    let mut parent = dir.path().to_path_buf();
    for _ in 0..6 {
        parent.push("a".repeat(50));
    }
    fs::create_dir_all(&parent).unwrap();
    let source = parent.join("source.flac");
    let destination = parent.join("new.flac");
    assert!(source.as_os_str().encode_wide().count() > 260);
    fs::write(&source, b"original audio").unwrap();
    rename_file_noclobber(&source, &destination).unwrap();
    assert!(!source.exists());
    assert_eq!(fs::read(destination).unwrap(), b"original audio");
}
