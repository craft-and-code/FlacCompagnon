use super::super::{copy_tags, read_tags, write_tags, TagEdits};
use super::*;

/// A byte-minimal PNG: real 8-byte signature and an `IHDR` chunk carrying
/// `width`/`height`, nothing else. `PictureInformation::from_png` only
/// reads the signature and the IHDR fields (it returns before touching
/// `IDAT`/`IEND` unless the color type is indexed), so this is enough to
/// exercise the cover-art round trip without a real encoder or a
/// hand-copied binary fixture.
fn tiny_png(width: u32, height: u32) -> Vec<u8> {
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend_from_slice(&13u32.to_be_bytes()); // chunk length
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&width.to_be_bytes());
    png.extend_from_slice(&height.to_be_bytes());
    png.push(8); // bit depth
    png.push(2); // color type: truecolor (avoids the indexed/PLTE branch)
    png.push(0); // compression method
    png.push(0); // filter method
    png.push(0); // interlace method
    png.extend_from_slice(&[0, 0, 0, 0]); // CRC (unchecked by lofty here)
    png
}

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

fn set_png(picture_type: &str, png: &[u8]) -> CoverEdit {
    CoverEdit::Set {
        mime: "image/png".to_string(),
        data_base64: B64.encode(png),
        picture_type: picture_type.to_string(),
    }
}

#[test]
fn cover_art_round_trip() {
    let path = synth_wav();
    let png = tiny_png(1, 1);
    write_tags(
        &path,
        &TagEdits {
            pictures: vec![set_png("CoverFront", &png)],
            ..Default::default()
        },
    )
    .unwrap();

    let read = read_tags(&path).unwrap();
    let cover = read.pictures.first().expect("cover art should be present");
    assert_eq!(cover.mime, "image/png");
    assert_eq!(cover.width, 1);
    assert_eq!(cover.height, 1);
    assert_eq!(cover.picture_type, "CoverFront");
    assert_eq!(B64.decode(&cover.data_base64).unwrap(), png);

    // Clearing removes it.
    write_tags(
        &path,
        &TagEdits {
            pictures: vec![CoverEdit::Clear {
                picture_type: "CoverFront".into(),
            }],
            ..Default::default()
        },
    )
    .unwrap();
    assert!(read_tags(&path).unwrap().pictures.is_empty());
}

/// A back image must remain independently selectable after a front image
/// is written, and replacing or clearing one role must preserve the other.
#[test]
fn pictures_of_other_roles_survive_set_and_clear() {
    let path = synth_wav();
    let front = tiny_png(1, 1);
    let back = tiny_png(2, 2);
    write_tags(
        &path,
        &TagEdits {
            pictures: vec![set_png("CoverFront", &front), set_png("CoverBack", &back)],
            ..Default::default()
        },
    )
    .unwrap();
    let pictures = read_tags(&path).unwrap().pictures;
    assert_eq!(pictures.len(), 2);
    assert!(pictures
        .iter()
        .any(|p| p.picture_type == "CoverFront" && p.width == 1));
    assert!(pictures
        .iter()
        .any(|p| p.picture_type == "CoverBack" && p.width == 2));

    write_tags(
        &path,
        &TagEdits {
            pictures: vec![set_png("CoverBack", &tiny_png(3, 3))],
            ..Default::default()
        },
    )
    .unwrap();
    let pictures = read_tags(&path).unwrap().pictures;
    assert_eq!(pictures.len(), 2);
    assert!(pictures
        .iter()
        .any(|p| p.picture_type == "CoverFront" && p.width == 1));
    assert!(pictures
        .iter()
        .any(|p| p.picture_type == "CoverBack" && p.width == 3));

    write_tags(
        &path,
        &TagEdits {
            pictures: vec![CoverEdit::Clear {
                picture_type: "CoverBack".into(),
            }],
            ..Default::default()
        },
    )
    .unwrap();
    let pictures = read_tags(&path).unwrap().pictures;
    assert_eq!(pictures.len(), 1);
    assert_eq!(pictures[0].picture_type, "CoverFront");
}

#[test]
fn copying_tags_keeps_every_picture_role() {
    let source = synth_wav();
    let destination = synth_wav();
    write_tags(
        &source,
        &TagEdits {
            pictures: vec![
                set_png("CoverFront", &tiny_png(1, 1)),
                set_png("CoverBack", &tiny_png(2, 2)),
                set_png("CoverBack", &tiny_png(3, 3)),
            ],
            ..Default::default()
        },
    )
    .unwrap();

    copy_tags(&source, &destination).unwrap();
    let pictures = read_tags(&destination).unwrap().pictures;
    assert_eq!(pictures.len(), 3);
    assert!(pictures.iter().any(|p| p.picture_type == "CoverFront"));
    assert_eq!(
        pictures
            .iter()
            .filter(|p| p.picture_type == "CoverBack")
            .count(),
        2
    );
}

#[test]
fn write_cover_file_round_trips_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("cover.png");
    let png = tiny_png(2, 2);
    write_cover_file(&dest, &B64.encode(&png)).unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), png);
}

/// An unrecognized role string (never offered by the frontend, but
/// defensively handled) falls back to `Other` instead of failing the
/// write — see `parse_picture_type`'s doc comment.
#[test]
fn unrecognized_picture_type_falls_back_to_other() {
    let path = synth_wav();
    let png = tiny_png(1, 1);
    write_tags(
        &path,
        &TagEdits {
            pictures: vec![set_png("SomethingLoftyDoesntKnow", &png)],
            ..Default::default()
        },
    )
    .unwrap();

    let cover = read_tags(&path)
        .unwrap()
        .pictures
        .into_iter()
        .next()
        .expect("cover art should be present");
    assert_eq!(cover.picture_type, "Other");
}
