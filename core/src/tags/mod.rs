//! Reading and writing metadata tags (title, artist, album, cover art, …).
//!
//! Unlike the rest of this crate, this module **does** modify the files it
//! touches — [`write_tags`] is the one place in FlacCompagnon that writes to
//! the audio files being analyzed rather than a separate report. It exists to
//! back the tag-editing panel: batch reads across a selection to pre-fill the
//! form, and a single batch write to apply the user's edits once they hit
//! "Enregistrer".
//!
//! Backed by [`lofty`](https://docs.rs/lofty), which gives a single API across
//! FLAC (Vorbis comments), MP3/AIFF/WAV (ID3v2), MP4/M4A (iTunes atoms) and
//! Ogg/Opus (Vorbis comments). DSD (`.dsf`/`.dff`) isn't a format lofty knows
//! how to tag; [`read_tags`]/[`write_tags`] return [`TagError::Parse`] for it,
//! which the caller surfaces as a per-file "not supported" message rather than
//! failing a whole batch.
//!
//! ## Batch-edit semantics
//!
//! The tag panel edits several files at once, and a field the user never
//! touched must not clobber files that had a *different* value than the one
//! shown (the "valeurs multiples" badge case). [`FieldEdit`] encodes this as
//! a three-way choice — `Unset` (leave alone), `Clear` (remove the tag) or
//! `Set` (write this value) — rather than the usual `Option<String>`, which
//! can't distinguish "leave alone" from "set to empty".
//!
//! ## Module layout
//!
//! This module owns the text fields ([`TagSet`], [`FieldEdit`], [`TagEdits`])
//! and the top-level [`read_tags`]/[`write_tags`] orchestration. Everything
//! specific to the embedded picture — [`CoverArt`], [`CoverEdit`], MIME
//! sniffing, mapping a role name to lofty's `PictureType` — lives in
//! [`cover`], which `read_tags`/`write_tags` call into rather than handling
//! inline. The cover types are re-exported here so existing callers
//! (`core::tags::CoverArt`, etc.) don't need to know the split exists.

pub mod cover;
pub use cover::{cover_from_bytes, read_cover_file, write_cover_file, CoverArt, CoverEdit};

use std::path::Path;

use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt as _;
use lofty::read_from_path;
use lofty::tag::{Accessor as _, ItemKey, Tag, TagExt as _};
use serde::{Deserialize, Serialize};

/// Errors that can occur while reading or writing tags.
#[derive(Debug, thiserror::Error)]
pub enum TagError {
    /// The file's format is unsupported for tagging, or its tags could not
    /// be parsed (path, reason).
    #[error("{0}: unreadable or unsupported format for tags ({1})")]
    Parse(String, String),
    /// The tags could not be written back to the file (path, reason).
    #[error("{0}: could not write tags ({1})")]
    Write(String, String),
    /// The replacement cover image data was invalid (path, reason).
    #[error("{0}: invalid cover image data ({1})")]
    Cover(String, String),
}

/// The tags of a single file, as read from disk.
///
/// Fields absent from the file are `None`. `extra` carries every other
/// textual tag item lofty recognized but that isn't one of the common fields
/// above (ISRC, BPM, ReplayGain, custom `TXXX`/user frames, …), keyed by its
/// format-specific tag name — this backs the "tags étendus" panel.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TagSet {
    /// Track title.
    pub title: Option<String>,
    /// Track artist.
    pub artist: Option<String>,
    /// Album name.
    pub album: Option<String>,
    /// Album artist (as distinct from the track artist, for compilations).
    pub album_artist: Option<String>,
    /// Composer.
    pub composer: Option<String>,
    /// Release year, as free text (formats vary: `"2019"`, `"2019-04-05"`, …).
    pub year: Option<String>,
    /// Track number.
    pub track: Option<String>,
    /// Total number of tracks on the disc/release.
    pub track_total: Option<String>,
    /// Disc number, for multi-disc releases.
    pub disc: Option<String>,
    /// Total number of discs in the release.
    pub disc_total: Option<String>,
    /// Genre.
    pub genre: Option<String>,
    /// Free-text comment field.
    pub comment: Option<String>,
    /// `true` when the file is tagged as part of a compilation.
    pub compilation: bool,
    /// Every other textual tag item, keyed by its format-specific tag name.
    pub extra: Vec<(String, String)>,
    /// Embedded cover art, when the file has one.
    pub cover: Option<CoverArt>,
    /// The MusicBrainz Release ID, if this file already carries one (e.g.
    /// tagged previously by Picard, or by a ripper that writes it). Lets the
    /// "Search online" button skip straight to that exact release instead of
    /// a fuzzy text search — still surfaced under its raw tag name in
    /// `extra` too, so it's visible in the extended-tags viewer.
    pub musicbrainz_release_id: Option<String>,
}

/// A single field's edit instruction for a batch write.
///
/// `Unset` is the default so a [`TagEdits`] built from "only the fields the
/// user actually typed into" naturally leaves everything else untouched.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum FieldEdit {
    /// Leave this field untouched.
    #[default]
    Unset,
    /// Remove this field's tag entirely.
    Clear,
    /// Write this value.
    Set(String),
}

/// A batch of edits to apply to every file in a selection. See the module
/// docs for why fields aren't plain `Option<String>`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagEdits {
    /// Edit instruction for [`TagSet::title`].
    pub title: FieldEdit,
    /// Edit instruction for [`TagSet::artist`].
    pub artist: FieldEdit,
    /// Edit instruction for [`TagSet::album`].
    pub album: FieldEdit,
    /// Edit instruction for [`TagSet::album_artist`].
    pub album_artist: FieldEdit,
    /// Edit instruction for [`TagSet::composer`].
    pub composer: FieldEdit,
    /// Edit instruction for [`TagSet::year`].
    pub year: FieldEdit,
    /// Edit instruction for [`TagSet::track`].
    pub track: FieldEdit,
    /// Edit instruction for [`TagSet::track_total`].
    pub track_total: FieldEdit,
    /// Edit instruction for [`TagSet::disc`].
    pub disc: FieldEdit,
    /// Edit instruction for [`TagSet::disc_total`].
    pub disc_total: FieldEdit,
    /// Edit instruction for [`TagSet::genre`].
    pub genre: FieldEdit,
    /// Edit instruction for [`TagSet::comment`].
    pub comment: FieldEdit,
    /// `None` leaves the compilation flag untouched.
    pub compilation: Option<bool>,
    /// Edit instruction for the embedded cover art.
    pub cover: CoverEdit,
}

/// Every [`ItemKey`] already surfaced as a named [`TagSet`] field — excluded
/// from `TagSet::extra` so the extended-tags panel doesn't duplicate them.
const KNOWN_KEYS: &[ItemKey] = &[
    ItemKey::TrackTitle,
    ItemKey::TrackArtist,
    ItemKey::AlbumTitle,
    ItemKey::Genre,
    ItemKey::Comment,
    ItemKey::AlbumArtist,
    ItemKey::Composer,
    ItemKey::Year,
    ItemKey::RecordingDate,
    ItemKey::TrackNumber,
    ItemKey::TrackTotal,
    ItemKey::DiscNumber,
    ItemKey::DiscTotal,
    ItemKey::FlagCompilation,
];

fn parse_err(path: &Path, e: impl std::fmt::Display) -> TagError {
    TagError::Parse(path.display().to_string(), e.to_string())
}

/// Read every tag FlacCompagnon knows how to show from `path`'s primary tag
/// (the tag type a player would actually look at — see
/// [`lofty::file::FileType::primary_tag_type`]).
pub fn read_tags(path: &Path) -> Result<TagSet, TagError> {
    let tagged = read_from_path(path).map_err(|e| parse_err(path, e))?;
    let mut out = TagSet::default();

    let Some(tag) = tagged.primary_tag() else {
        return Ok(out);
    };

    out.title = tag.title().map(|c| c.into_owned());
    out.artist = tag.artist().map(|c| c.into_owned());
    out.album = tag.album().map(|c| c.into_owned());
    out.genre = tag.genre().map(|c| c.into_owned());
    out.comment = tag.comment().map(|c| c.into_owned());
    out.album_artist = tag.get_string(ItemKey::AlbumArtist).map(str::to_string);
    out.composer = tag.get_string(ItemKey::Composer).map(str::to_string);
    out.year = tag
        .get_string(ItemKey::Year)
        .or_else(|| tag.get_string(ItemKey::RecordingDate))
        .map(str::to_string);
    out.track = tag.get_string(ItemKey::TrackNumber).map(str::to_string);
    out.track_total = tag.get_string(ItemKey::TrackTotal).map(str::to_string);
    out.disc = tag.get_string(ItemKey::DiscNumber).map(str::to_string);
    out.disc_total = tag.get_string(ItemKey::DiscTotal).map(str::to_string);
    out.compilation = tag.get_string(ItemKey::FlagCompilation) == Some("1");
    out.musicbrainz_release_id = tag
        .get_string(ItemKey::MusicBrainzReleaseId)
        .map(str::to_string)
        .filter(|s| !s.is_empty());

    for item in tag.items() {
        if KNOWN_KEYS.contains(&item.key()) {
            continue;
        }
        if let Some(text) = item.value().text() {
            if text.is_empty() {
                continue;
            }
            let label = item
                .key()
                .map_key(tag.tag_type())
                .unwrap_or("?")
                .to_string();
            out.extra.push((label, text.to_string()));
        }
    }

    out.cover = cover::extract(tag);

    Ok(out)
}

fn apply_field(tag: &mut Tag, key: ItemKey, edit: &FieldEdit) {
    match edit {
        FieldEdit::Unset => {}
        FieldEdit::Clear => tag.remove_key(key),
        FieldEdit::Set(value) => {
            tag.insert_text(key, value.clone());
        }
    }
}

/// Apply `edits` to `path`, writing straight to the audio file.
///
/// Reads the file's current primary tag first (so untouched fields, and any
/// tag the panel doesn't know about, survive the round trip), applies only
/// the fields present in `edits`, and saves. A file with no tag yet gets a
/// fresh one of the format's primary [`lofty::tag::TagType`].
pub fn write_tags(path: &Path, edits: &TagEdits) -> Result<(), TagError> {
    let tagged = read_from_path(path).map_err(|e| parse_err(path, e))?;
    let tag_type = tagged.primary_tag_type();
    let mut tag = tagged
        .primary_tag()
        .cloned()
        .unwrap_or_else(|| Tag::new(tag_type));

    apply_field(&mut tag, ItemKey::TrackTitle, &edits.title);
    apply_field(&mut tag, ItemKey::TrackArtist, &edits.artist);
    apply_field(&mut tag, ItemKey::AlbumTitle, &edits.album);
    apply_field(&mut tag, ItemKey::AlbumArtist, &edits.album_artist);
    apply_field(&mut tag, ItemKey::Composer, &edits.composer);
    // `ItemKey::Year` has no mapping at all for ID3v2 (only `RecordingDate`
    // does — see lofty's `ID3V2_MAP`), so writing MP3/WAV/AIFF/AAC tags with
    // `Year` silently drops the field. `RecordingDate` round-trips on every
    // format lofty supports (ID3v2 TDRC, Vorbis DATE, MP4 ©day, RIFF ICRD),
    // so the "year" field is always written there; `read_tags` still checks
    // the legacy `Year` key first for files tagged by other tools.
    apply_field(&mut tag, ItemKey::RecordingDate, &edits.year);
    apply_field(&mut tag, ItemKey::TrackNumber, &edits.track);
    apply_field(&mut tag, ItemKey::TrackTotal, &edits.track_total);
    apply_field(&mut tag, ItemKey::DiscNumber, &edits.disc);
    apply_field(&mut tag, ItemKey::DiscTotal, &edits.disc_total);
    apply_field(&mut tag, ItemKey::Genre, &edits.genre);
    apply_field(&mut tag, ItemKey::Comment, &edits.comment);

    if let Some(compilation) = edits.compilation {
        tag.insert_text(
            ItemKey::FlagCompilation,
            if compilation { "1" } else { "0" }.to_string(),
        );
    }

    cover::apply_edit(&mut tag, &edits.cover, &path.display().to_string())?;

    tag.save_to_path(path, WriteOptions::default())
        .map_err(|e| TagError::Write(path.display().to_string(), e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    /// A minimal, silent WAV — same fixture style as the rest of the crate's
    /// tests (`hound`-generated, no real audio content needed).
    fn synth_wav() -> tempfile::TempPath {
        let file = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();
        let path = file.into_temp_path();
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for _ in 0..1000 {
            writer.write_sample(0i16).unwrap();
            writer.write_sample(0i16).unwrap();
        }
        writer.finalize().unwrap();
        path
    }

    #[test]
    fn round_trip_basic_fields() {
        let path = synth_wav();
        let edits = TagEdits {
            title: FieldEdit::Set("Title".into()),
            artist: FieldEdit::Set("Artist".into()),
            album: FieldEdit::Set("Album".into()),
            album_artist: FieldEdit::Set("Album Artist".into()),
            composer: FieldEdit::Set("Composer".into()),
            year: FieldEdit::Set("2026".into()),
            track: FieldEdit::Set("3".into()),
            track_total: FieldEdit::Set("12".into()),
            disc: FieldEdit::Set("1".into()),
            disc_total: FieldEdit::Set("2".into()),
            genre: FieldEdit::Set("Electronic".into()),
            comment: FieldEdit::Set("Test comment".into()),
            compilation: Some(true),
            cover: CoverEdit::Unset,
        };
        write_tags(&path, &edits).expect("write_tags should succeed on a fresh WAV");

        let read = read_tags(&path).expect("read_tags should succeed after writing");
        assert_eq!(read.title.as_deref(), Some("Title"));
        assert_eq!(read.artist.as_deref(), Some("Artist"));
        assert_eq!(read.album.as_deref(), Some("Album"));
        assert_eq!(read.album_artist.as_deref(), Some("Album Artist"));
        assert_eq!(read.composer.as_deref(), Some("Composer"));
        assert_eq!(read.year.as_deref(), Some("2026"));
        assert_eq!(read.track.as_deref(), Some("3"));
        assert_eq!(read.track_total.as_deref(), Some("12"));
        assert_eq!(read.disc.as_deref(), Some("1"));
        assert_eq!(read.disc_total.as_deref(), Some("2"));
        assert_eq!(read.genre.as_deref(), Some("Electronic"));
        assert_eq!(read.comment.as_deref(), Some("Test comment"));
        assert!(read.compilation);
    }

    #[test]
    fn unset_field_does_not_touch_existing_value() {
        let path = synth_wav();
        write_tags(
            &path,
            &TagEdits {
                title: FieldEdit::Set("Kept".into()),
                artist: FieldEdit::Set("Also kept".into()),
                ..Default::default()
            },
        )
        .unwrap();

        // Second write only touches `album`; `title`/`artist` are `Unset` by
        // `Default` and must survive untouched — this is the batch-edit
        // guarantee the tag panel relies on.
        write_tags(
            &path,
            &TagEdits {
                album: FieldEdit::Set("New album".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let read = read_tags(&path).unwrap();
        assert_eq!(read.title.as_deref(), Some("Kept"));
        assert_eq!(read.artist.as_deref(), Some("Also kept"));
        assert_eq!(read.album.as_deref(), Some("New album"));
    }

    #[test]
    fn clear_removes_the_field() {
        let path = synth_wav();
        write_tags(
            &path,
            &TagEdits {
                title: FieldEdit::Set("Temporary".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(read_tags(&path).unwrap().title.as_deref(), Some("Temporary"));

        write_tags(
            &path,
            &TagEdits {
                title: FieldEdit::Clear,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(read_tags(&path).unwrap().title, None);
    }

    /// The "already tagged by Picard" shortcut in the tag panel's online
    /// lookup depends on this field being surfaced, so it's read back here
    /// from a tag written directly with lofty (FlacCompagnon itself never
    /// writes the ID — it only ever consumes one that's already there).
    ///
    /// Whether the key survives a write is format-dependent (the same class
    /// of mapping gap documented on `ItemKey::Year` in `write_tags`), so the
    /// assertion is phrased against what lofty itself reports for that key:
    /// this pins down *our* mapping — `read_tags` must surface whatever is
    /// there, and never invent a value — without hard-coding an assumption
    /// about the fixture's tag format.
    #[test]
    fn reads_an_existing_musicbrainz_release_id() {
        let path = synth_wav();
        let mbid = "c1a1c5f0-6b1e-4f3a-9b2d-1a2b3c4d5e6f";

        // Same read-clone-modify-save shape `write_tags` itself uses.
        let tagged = read_from_path(&path).unwrap();
        let tag_type = tagged.primary_tag_type();
        let mut tag = tagged.primary_tag().cloned().unwrap_or_else(|| Tag::new(tag_type));
        tag.insert_text(ItemKey::MusicBrainzReleaseId, mbid.to_string());
        tag.save_to_path(&path, WriteOptions::default()).unwrap();

        let reread = read_from_path(&path).unwrap();
        let stored = reread
            .primary_tag()
            .and_then(|t| t.get_string(ItemKey::MusicBrainzReleaseId))
            .map(str::to_string);

        let read = read_tags(&path).unwrap();
        assert_eq!(read.musicbrainz_release_id, stored);
        // And when the format did keep it, it's the exact value written.
        if stored.is_some() {
            assert_eq!(read.musicbrainz_release_id.as_deref(), Some(mbid));
        }
    }

    /// A file with no MusicBrainz ID must report `None` rather than an empty
    /// string — the lookup treats "absent" and "present but blank" the same,
    /// and only `None` skips the shortcut.
    #[test]
    fn missing_musicbrainz_release_id_is_none() {
        let path = synth_wav();
        write_tags(
            &path,
            &TagEdits {
                title: FieldEdit::Set("Title".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(read_tags(&path).unwrap().musicbrainz_release_id, None);
    }

    #[test]
    fn unsupported_format_returns_a_named_error() {
        // A tiny fake DSF: lofty doesn't have a DSD resolver, so this must
        // come back as a clean per-file error rather than a panic.
        let mut file = tempfile::Builder::new().suffix(".dsf").tempfile().unwrap();
        file.write_all(b"DSD not a real dsf file").unwrap();
        let path = file.into_temp_path();
        assert!(read_tags(&path).is_err());
    }
}
