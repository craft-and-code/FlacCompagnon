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
        pictures: Vec::new(),
        extra: Vec::new(),
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
    assert_eq!(
        read_tags(&path).unwrap().title.as_deref(),
        Some("Temporary")
    );

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
    let mut tag = tagged
        .primary_tag()
        .cloned()
        .unwrap_or_else(|| Tag::new(tag_type));
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

/// An extended tag written through `edits.extra` round-trips through
/// `read_tags`'s own `extra`, keyed by the same raw format-specific
/// name — the extended-tags pop-in's "edit" and "add" cases.
#[test]
fn extra_tag_round_trips() {
    let path = synth_wav();
    let tagged = read_from_path(&path).unwrap();
    let tag_type = tagged.primary_tag_type();
    let key = ItemKey::Isrc
        .map_key(tag_type)
        .expect("ISRC maps on this format")
        .to_string();

    write_tags(
        &path,
        &TagEdits {
            extra: vec![(key.clone(), FieldEdit::Set("US-S1Z-99-00001".into()))],
            ..Default::default()
        },
    )
    .unwrap();

    let read = read_tags(&path).unwrap();
    assert_eq!(
        read.extra
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_str()),
        Some("US-S1Z-99-00001")
    );
}

/// The extended-tags pop-in's "−" button, staged as `Clear` — removes the
/// key entirely rather than leaving it present with an empty value.
#[test]
fn extra_tag_clear_removes_it() {
    let path = synth_wav();
    let tagged = read_from_path(&path).unwrap();
    let tag_type = tagged.primary_tag_type();
    let key = ItemKey::Isrc
        .map_key(tag_type)
        .expect("ISRC maps on this format")
        .to_string();

    write_tags(
        &path,
        &TagEdits {
            extra: vec![(key.clone(), FieldEdit::Set("US-S1Z-99-00001".into()))],
            ..Default::default()
        },
    )
    .unwrap();
    assert!(read_tags(&path)
        .unwrap()
        .extra
        .iter()
        .any(|(k, _)| *k == key));

    write_tags(
        &path,
        &TagEdits {
            extra: vec![(key.clone(), FieldEdit::Clear)],
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!read_tags(&path)
        .unwrap()
        .extra
        .iter()
        .any(|(k, _)| *k == key));
}

/// A key this file's tag type has no `ItemKey` mapping for must not fail
/// the whole write — the extended-tags panel can't know a mixed-format
/// selection supports every key its "+" list offers (see `write_tags`'s
/// doc comment on `Year`/`RecordingDate` for the same pattern with a
/// named field), so it's silently skipped instead.
#[test]
fn extra_tag_with_unmapped_key_is_silently_skipped() {
    let path = synth_wav();
    let result = write_tags(
        &path,
        &TagEdits {
            extra: vec![(
                "NOT_A_REAL_TAG_KEY_XYZ".into(),
                FieldEdit::Set("value".into()),
            )],
            ..Default::default()
        },
    );
    assert!(result.is_ok());
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
