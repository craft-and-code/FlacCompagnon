//! The extended-tags pop-in's "+" picker: a curated list of common
//! [`ItemKey`]s, resolved against one file's tag type into the raw
//! format-specific keys [`super::write_tags`] can actually take.
//!
//! Split out of `tags` proper for the same reason [`super::cover`] is: the
//! curated list and its resolution are one self-contained piece the main
//! read/write orchestration doesn't need inline, and keeping it here is what
//! lets that module stay under this codebase's size cap (see CLAUDE.md).

use std::path::Path;

use lofty::file::TaggedFileExt as _;
use lofty::read_from_path;
use lofty::tag::ItemKey;
use serde::Serialize;

use super::TagError;

/// Curated `ItemKey`s worth offering in the extended-tags pop-in's "+"
/// picker — common fields a listener might plausibly want to add by hand
/// (ISRC, BPM, ReplayGain, catalog/barcode, credits, ...) that aren't one of
/// the panel's own named fields. Not exhaustive: lofty pins `ItemKey` as
/// `#[non_exhaustive]` with no full-enumeration API (107 variants and
/// counting, per its docs), so this is hand-picked the same way
/// `super::KNOWN_KEYS` is, rather than derived from the crate.
const ADDABLE_KEYS: &[(ItemKey, &str)] = &[
    (ItemKey::Isrc, "ISRC"),
    (ItemKey::Bpm, "BPM"),
    (ItemKey::IntegerBpm, "BPM (integer)"),
    (ItemKey::InitialKey, "Initial key"),
    (ItemKey::Mood, "Mood"),
    (ItemKey::Conductor, "Conductor"),
    (ItemKey::Remixer, "Remixer"),
    (ItemKey::Producer, "Producer"),
    (ItemKey::Publisher, "Publisher"),
    (ItemKey::Label, "Record label"),
    (ItemKey::CatalogNumber, "Catalog number"),
    (ItemKey::Barcode, "Barcode"),
    (ItemKey::Language, "Language"),
    (ItemKey::Lyrics, "Lyrics"),
    (ItemKey::Work, "Work"),
    (ItemKey::Movement, "Movement"),
    (ItemKey::OriginalArtist, "Original artist"),
    (ItemKey::OriginalAlbumTitle, "Original album"),
    (ItemKey::EncodedBy, "Encoded by"),
    (ItemKey::EncoderSoftware, "Encoder"),
    (ItemKey::CopyrightMessage, "Copyright"),
    (ItemKey::ReplayGainTrackGain, "ReplayGain track gain"),
    (ItemKey::ReplayGainTrackPeak, "ReplayGain track peak"),
    (ItemKey::ReplayGainAlbumGain, "ReplayGain album gain"),
    (ItemKey::ReplayGainAlbumPeak, "ReplayGain album peak"),
    (ItemKey::MusicBrainzArtistId, "MusicBrainz Artist ID"),
    (ItemKey::MusicBrainzTrackId, "MusicBrainz Track ID"),
];

/// One entry in the extended-tags pop-in's "+" picker: the raw
/// format-specific key to write (matches `TagSet::extra`'s keys) paired
/// with a human label.
#[derive(Debug, Clone, Serialize)]
pub struct AddableTag {
    /// Raw format-specific tag name, as `write_tags`'s `edits.extra` expects.
    pub key: String,
    /// Human-readable label for the picker.
    pub label: String,
}

/// `ADDABLE_KEYS` resolved against `path`'s tag type and filtered to the
/// ones that format actually supports (`ItemKey::map_key` returns `None`
/// for a variant with no mapping in that scheme) — so the picker only ever
/// offers a tag that can actually be written to the file(s) at hand. The
/// batch always applies one edit to the whole selection, so one
/// representative file's format is enough to decide what's offered.
pub fn addable_tags(path: &Path) -> Result<Vec<AddableTag>, TagError> {
    let tagged = read_from_path(path)
        .map_err(|e| TagError::Parse(path.display().to_string(), e.to_string()))?;
    let tag_type = tagged.primary_tag_type();
    Ok(ADDABLE_KEYS
        .iter()
        .filter_map(|(key, label)| {
            key.map_key(tag_type).map(|raw| AddableTag {
                key: raw.to_string(),
                label: (*label).to_string(),
            })
        })
        .collect())
}

#[cfg(test)]
#[path = "../../tests/unit/tags/addable.rs"]
mod tests;
