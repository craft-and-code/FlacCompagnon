//! Playlist export (Simple or Extended M3U) — built entirely from data the
//! frontend already has in memory (the table's current order, each file's
//! analyzed duration, cached tags), never re-reading the tracks from disk on
//! this side. Absolute paths are written (a selection can span several
//! folders after a multi-drop), so the playlist opens correctly from
//! anywhere, at the cost of breaking if the files are later moved.

use serde::{Deserialize, Serialize};

/// One playlist entry. `title`/`artist` are `None` when tags weren't
/// available for that file — falls back to the file name, same as most
/// players do for an untagged track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistEntry {
    /// Absolute path to the audio file.
    pub path: String,
    /// Track length in seconds, from the analysis already performed.
    pub duration_secs: f64,
    /// Track title, when a tag was available.
    pub title: Option<String>,
    /// Track artist, when a tag was available.
    pub artist: Option<String>,
}

/// Which flavor of M3U to write — picked by the user in the export pop-in
/// (Extended is the default). Externally-tagged as a bare string over the
/// wire ("Simple" / "Extended"), same convention as the tag panel's
/// `FieldEdit`/`CoverEdit` unit variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaylistFormat {
    /// One absolute path per line, nothing else — the original M3U format,
    /// understood by literally everything.
    Simple,
    /// `#EXTM3U` header, then one `#EXTINF:duration,Artist - Title` line
    /// before each path, so a compatible player can show a title without
    /// opening the file itself.
    Extended,
}

/// Build the playlist text in the requested format.
pub fn build_playlist(entries: &[PlaylistEntry], format: PlaylistFormat) -> String {
    match format {
        PlaylistFormat::Simple => build_simple_m3u(entries),
        PlaylistFormat::Extended => build_extended_m3u(entries),
    }
}

/// Simple M3U: just the paths, one per line.
pub fn build_simple_m3u(entries: &[PlaylistEntry]) -> String {
    let mut out = String::new();
    for e in entries {
        out.push_str(&e.path);
        out.push('\n');
    }
    out
}

/// Extended M3U: an `#EXTM3U` header, then one `#EXTINF` + path pair per
/// entry, in the order given.
pub fn build_extended_m3u(entries: &[PlaylistEntry]) -> String {
    let mut out = String::from("#EXTM3U\n");
    for e in entries {
        let label = match (&e.artist, &e.title) {
            (Some(a), Some(t)) if !a.is_empty() && !t.is_empty() => format!("{a} - {t}"),
            (_, Some(t)) if !t.is_empty() => t.clone(),
            (Some(a), _) if !a.is_empty() => format!("{a} - {}", file_stem(&e.path)),
            _ => file_stem(&e.path),
        };
        out.push_str(&format!(
            "#EXTINF:{},{}\n{}\n",
            e.duration_secs.round().max(0.0) as i64,
            label,
            e.path,
        ));
    }
    out
}

fn file_stem(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    match name.rfind('.') {
        Some(i) if i > 0 => name[..i].to_string(),
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
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
            PlaylistEntry { path: "/m/2.flac".into(), duration_secs: 1.0, title: Some("Two".into()), artist: None },
            PlaylistEntry { path: "/m/1.flac".into(), duration_secs: 1.0, title: Some("One".into()), artist: None },
        ];
        let m3u = build_extended_m3u(&entries);
        assert!(m3u.find("Two").unwrap() < m3u.find("One").unwrap());
    }

    #[test]
    fn simple_format_is_just_paths() {
        let entries = vec![
            PlaylistEntry { path: "/m/1.flac".into(), duration_secs: 1.0, title: Some("One".into()), artist: Some("Artist".into()) },
            PlaylistEntry { path: "/m/2.flac".into(), duration_secs: 2.0, title: None, artist: None },
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
        assert_eq!(build_playlist(&entries, PlaylistFormat::Simple), "/m/1.flac\n");
        assert!(build_playlist(&entries, PlaylistFormat::Extended).starts_with("#EXTM3U"));
    }
}
