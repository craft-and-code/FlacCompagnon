//! Optional online tag lookup for the tag panel's "Search online" button —
//! the only part of FlacCompagnon that reaches the network. It only ever runs
//! on explicit user action (a button click), never automatically, and never
//! touches a file directly: a search returns candidate releases, picking one
//! fetches its full track list, and the frontend stages the result into the
//! tag panel's fields for the user to review and Save (or discard) exactly
//! like a manual edit.
//!
//! One file per provider ([`musicbrainz`], [`discogs`]) because they share
//! nothing but the shapes below and the HTTP plumbing in [`http`]: different
//! auth, different JSON, different quirks. The result types live here so both
//! providers answer in one vocabulary the frontend can render without caring
//! which one replied.
//!
//! Both providers' JSON responses are parsed defensively through
//! `serde_json::Value` rather than strict `#[derive(Deserialize)]` structs:
//! there's no way to compile-check field names against a live response here
//! (this repo's build environment has no network access to either API), so
//! a slightly-off field name should degrade to "that piece of data is
//! missing" rather than fail the whole request. If a field listed in the
//! providers' docs doesn't show up in practice, that's the first thing to
//! check.

mod http;

pub mod discogs;
pub mod musicbrainz;

use flaccompagnon_core::tags::CoverArt;
use serde::Serialize;

pub use discogs::{discogs_detail, discogs_search};
pub use musicbrainz::{musicbrainz_detail, musicbrainz_search};

/// One entry in a search results list — enough to tell candidates apart and
/// pick one; the full track list is a separate fetch (see `*_detail`).
#[derive(Clone, Serialize)]
pub struct LookupCandidate {
    pub source: String, // "MusicBrainz" | "Discogs"
    pub id: String,
    pub title: String,
    pub artist: String,
    pub year: Option<String>,
    pub track_count: Option<u32>,
}

#[derive(Clone, Serialize)]
pub struct LookupTrack {
    pub position: String,
    pub title: String,
}

/// Full detail for a chosen candidate: enough to fill in the tag panel.
#[derive(Clone, Serialize)]
pub struct LookupRelease {
    pub title: String,
    pub artist: String,
    pub year: Option<String>,
    pub tracks: Vec<LookupTrack>,
    pub cover: Option<CoverArt>,
}

/// Four-digit year from a provider's free-form date string ("1997",
/// "1997-06-16", ""). Both providers hand back dates of varying precision in
/// the same field, and only the year is ever written to a tag.
///
/// Slicing is on bytes, so a non-ASCII value (a provider returning something
/// unexpected) must not be able to split a character mid-way and panic —
/// hence the `is_ascii_digit` check rather than a bare length test.
pub(crate) fn year_prefix(date: &str) -> Option<String> {
    let head = date.get(..4)?;
    head.bytes().all(|b| b.is_ascii_digit()).then(|| head.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn year_prefix_takes_only_a_four_digit_head() {
        assert_eq!(year_prefix("1997").as_deref(), Some("1997"));
        assert_eq!(year_prefix("1997-06-16").as_deref(), Some("1997"));
        assert_eq!(year_prefix(""), None);
        assert_eq!(year_prefix("199"), None);
        assert_eq!(year_prefix("unknown"), None);
    }

    /// A provider returning a multi-byte character where a date was expected
    /// must not panic on the byte slice — this is the crash `s[..4]` would
    /// have caused on, for example, "±1997".
    #[test]
    fn year_prefix_survives_non_ascii() {
        assert_eq!(year_prefix("±1997"), None);
        assert_eq!(year_prefix("日本語です"), None);
    }
}
