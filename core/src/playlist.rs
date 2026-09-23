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
#[path = "../tests/unit/playlist.rs"]
mod tests;
