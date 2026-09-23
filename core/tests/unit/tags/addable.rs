use super::*;
use lofty::config::WriteOptions;
use lofty::tag::{Tag, TagExt as _, TagType};

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
    // Give the file a tag so `read_from_path` has a `primary_tag_type`
    // to report rather than falling back to the container's default.
    let tag = Tag::new(TagType::Id3v2);
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    path
}

/// Every key `addable_tags` offers must resolve back to an `ItemKey` for
/// the same file's tag type — the exact round trip `write_tags` relies on
/// when the picker's choice comes back as an edit.
#[test]
fn addable_tags_round_trip_for_their_format() {
    let path = synth_wav();
    let tags = addable_tags(&path).unwrap();
    assert!(!tags.is_empty());

    let tagged = read_from_path(&path).unwrap();
    let tag_type = tagged.primary_tag_type();
    for t in &tags {
        assert!(
            ItemKey::from_key(tag_type, &t.key).is_some(),
            "{} did not round-trip",
            t.key
        );
    }
}
