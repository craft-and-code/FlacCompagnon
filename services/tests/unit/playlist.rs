use super::*;

#[test]
fn builds_extm3u_with_tags() {
    let entries = vec![PlaylistEntry {
        path: "/music/a.flac".into(),
        duration_secs: 183.4,
        title: Some("Song".into()),
        artist: Some("Artist".into()),
    }];
    let m3u = build_extended_m3u(&entries);
    assert!(m3u.starts_with("#EXTM3U\n"));
    assert!(m3u.contains("#EXTINF:183,Artist - Song\n"));
    assert!(m3u.contains("/music/a.flac\n"));
}

#[test]
fn falls_back_to_file_stem_without_tags() {
    let entries = vec![PlaylistEntry {
        path: "/music/b.flac".into(),
        duration_secs: 10.0,
        title: None,
        artist: None,
    }];
    let m3u = build_extended_m3u(&entries);
    assert!(m3u.contains("#EXTINF:10,b\n"));
}

#[test]
fn falls_back_to_artist_plus_file_stem_without_a_title() {
    let entries = vec![PlaylistEntry {
        path: "/music/c.flac".into(),
        duration_secs: 5.0,
        title: None,
        artist: Some("Artist".into()),
    }];
    let m3u = build_extended_m3u(&entries);
    assert!(m3u.contains("#EXTINF:5,Artist - c\n"));
}

#[test]
fn preserves_given_order() {
    let entries = vec![
        PlaylistEntry {
            path: "/m/2.flac".into(),
            duration_secs: 1.0,
            title: Some("Two".into()),
            artist: None,
        },
        PlaylistEntry {
            path: "/m/1.flac".into(),
            duration_secs: 1.0,
            title: Some("One".into()),
            artist: None,
        },
    ];
    let m3u = build_extended_m3u(&entries);
    assert!(m3u.find("Two").unwrap() < m3u.find("One").unwrap());
}

#[test]
fn simple_format_is_just_paths() {
    let entries = vec![
        PlaylistEntry {
            path: "/m/1.flac".into(),
            duration_secs: 1.0,
            title: Some("One".into()),
            artist: Some("Artist".into()),
        },
        PlaylistEntry {
            path: "/m/2.flac".into(),
            duration_secs: 2.0,
            title: None,
            artist: None,
        },
    ];
    let m3u = build_simple_m3u(&entries);
    assert_eq!(m3u, "/m/1.flac\n/m/2.flac\n");
    assert!(!m3u.contains("#EXTM3U"));
    assert!(!m3u.contains("#EXTINF"));
}

#[test]
fn build_playlist_dispatches_on_format() {
    let entries = vec![PlaylistEntry {
        path: "/m/1.flac".into(),
        duration_secs: 1.0,
        title: None,
        artist: None,
    }];
    assert_eq!(
        build_playlist(&entries, PlaylistFormat::Simple),
        "/m/1.flac\n"
    );
    assert!(build_playlist(&entries, PlaylistFormat::Extended).starts_with("#EXTM3U"));
}
