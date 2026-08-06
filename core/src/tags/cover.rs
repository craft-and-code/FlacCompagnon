//! Everything specific to cover art: the [`CoverArt`]/[`CoverEdit`] types,
//! sniffing a dropped image's format, and reading/writing the embedded
//! picture on a lofty [`Tag`]. Split out of `tags` proper so that module can
//! stay focused on the text-field read/write path — this one owns
//! everything about the image itself (mapping a role name to lofty's
//! [`PictureType`], extracting the picture on read, replacing it on write).
//!
//! [`extract`] and [`apply_edit`] are the two entry points the parent module
//! calls from [`super::read_tags`]/[`super::write_tags`]; everything else
//! here is either a public type shared with the frontend or a private
//! implementation detail.

use std::path::Path;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use lofty::picture::{MimeType, Picture, PictureInformation, PictureType};
use lofty::tag::Tag;
use serde::{Deserialize, Serialize};

use super::TagError;

/// Embedded cover art, ready for direct use in the frontend (`data_base64` is
/// the raw image bytes, base64-encoded — prefix with `data:{mime};base64,` to
/// use directly as an `<img src>`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoverArt {
    /// MIME type sniffed from the image bytes, e.g. `"image/jpeg"`.
    pub mime: String,
    /// Pixel width, read from the image's own header.
    pub width: u32,
    /// Pixel height, read from the image's own header.
    pub height: u32,
    /// Size of the embedded picture in bytes.
    pub size_bytes: usize,
    /// The picture's role, e.g. `"CoverFront"`, `"CoverBack"`, `"Other"` — the
    /// frontend localizes this into French for display.
    pub picture_type: String,
    /// Raw image bytes, base64-encoded (see the struct's own docs for how to
    /// use this directly as an `<img src>`).
    pub data_base64: String,
}

/// Cover art edit instruction — same three-way shape as [`super::FieldEdit`],
/// plus the image payload for `Set`. `picture_type` is the role to write it
/// under (`"CoverFront"`, `"CoverBack"`, …, matching the strings
/// [`extract`] surfaces via [`CoverArt::picture_type`]) — see
/// [`parse_picture_type`] for how an unrecognized string is handled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum CoverEdit {
    /// Leave the cover alone.
    #[default]
    Unset,
    /// Remove the cover entirely.
    Clear,
    /// Replace the cover with this image.
    Set {
        /// MIME type of the replacement image.
        mime: String,
        /// Replacement image bytes, base64-encoded.
        data_base64: String,
        /// Role to write the image under (see [`parse_picture_type`]).
        picture_type: String,
    },
}

/// Maps a picture-role name — one of the strings [`extract`] produces for
/// [`CoverArt::picture_type`] (Rust's `Debug` output for lofty's
/// [`PictureType`]) — back to the enum, for the tag panel's "change the
/// cover's role" control. Anything unrecognized (including lofty's
/// `Undefined(n)` Debug form, which this frontend never offers as a choice)
/// falls back to `Other` rather than erroring: a role is cosmetic metadata,
/// not worth failing a whole batch save over.
fn parse_picture_type(s: &str) -> PictureType {
    match s {
        "CoverFront" => PictureType::CoverFront,
        "CoverBack" => PictureType::CoverBack,
        "Icon" => PictureType::Icon,
        "OtherIcon" => PictureType::OtherIcon,
        "Leaflet" => PictureType::Leaflet,
        "Media" => PictureType::Media,
        "LeadArtist" => PictureType::LeadArtist,
        "Artist" => PictureType::Artist,
        "Conductor" => PictureType::Conductor,
        "Band" => PictureType::Band,
        "Composer" => PictureType::Composer,
        "Lyricist" => PictureType::Lyricist,
        "RecordingLocation" => PictureType::RecordingLocation,
        "DuringRecording" => PictureType::DuringRecording,
        "DuringPerformance" => PictureType::DuringPerformance,
        "ScreenCapture" => PictureType::ScreenCapture,
        "BrightFish" => PictureType::BrightFish,
        "Illustration" => PictureType::Illustration,
        "BandLogo" => PictureType::BandLogo,
        "PublisherLogo" => PictureType::PublisherLogo,
        _ => PictureType::Other,
    }
}

/// Identify an image file's MIME type from its magic bytes — used for a
/// dropped cover image, which (unlike an embedded picture) has no tag
/// metadata to say what it is.
fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.starts_with(b"BM") {
        return Some("image/bmp");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

/// Build a [`CoverArt`] from raw image bytes, from wherever they came from —
/// shared by [`read_cover_file`] (a dropped image) and the online lookup's
/// downloaded artwork (`src-tauri/src/lookup.rs`). `label` is only used to
/// identify the source in an error message (a file path, or a URL).
pub fn cover_from_bytes(bytes: Vec<u8>, label: &str) -> Result<CoverArt, TagError> {
    let mime = sniff_image_mime(&bytes).ok_or_else(|| {
        TagError::Cover(
            label.to_string(),
            "not a recognized image (PNG/JPEG/GIF/BMP/WebP)".to_string(),
        )
    })?;
    let size_bytes = bytes.len();
    let data_base64 = B64.encode(&bytes);
    // Reuses the same `PictureInformation` dimension-sniffing lofty already
    // does for embedded covers in `extract`, rather than a second image
    // decoder just for this.
    let picture = Picture::unchecked(bytes)
        .pic_type(PictureType::CoverFront)
        .mime_type(MimeType::from_str(mime))
        .build();
    let info = PictureInformation::from_picture(&picture).unwrap_or_default();
    Ok(CoverArt {
        mime: mime.to_string(),
        width: info.width,
        height: info.height,
        size_bytes,
        picture_type: "CoverFront".to_string(),
        data_base64,
    })
}

/// Read a plain image file (dropped onto the cover box, not an audio file)
/// into a [`CoverArt`] ready to stage as a [`CoverEdit::Set`] — backs
/// "drop an image to replace every selected file's cover".
pub fn read_cover_file(path: &Path) -> Result<CoverArt, TagError> {
    let bytes =
        std::fs::read(path).map_err(|e| TagError::Cover(path.display().to_string(), e.to_string()))?;
    cover_from_bytes(bytes, &path.display().to_string())
}

/// Write raw base64 image bytes out to `dest` as a plain file — the inverse
/// of [`read_cover_file`], backing the tag panel's "extract this cover next
/// to the audio file" button. `dest`'s extension is whatever the caller
/// already decided from the image's own MIME type; this just decodes and
/// writes, overwriting anything already there.
pub fn write_cover_file(dest: &Path, data_base64: &str) -> Result<(), TagError> {
    let bytes = B64
        .decode(data_base64)
        .map_err(|e| TagError::Cover(dest.display().to_string(), e.to_string()))?;
    std::fs::write(dest, bytes)
        .map_err(|e| TagError::Cover(dest.display().to_string(), e.to_string()))
}

/// Extract the embedded cover from `tag` — the front-cover role if present,
/// otherwise whatever picture comes first (so a file whose only picture is
/// mistagged as something else still shows *something*, with its real role
/// surfaced via `picture_type` rather than lied about). Called from
/// [`super::read_tags`].
pub(crate) fn extract(tag: &Tag) -> Option<CoverArt> {
    let picture = tag
        .get_picture_type(PictureType::CoverFront)
        .or_else(|| tag.pictures().first())?;
    let info = PictureInformation::from_picture(picture).unwrap_or_default();
    Some(CoverArt {
        mime: picture
            .mime_type()
            .map(|m| m.as_str().to_string())
            .unwrap_or_default(),
        width: info.width,
        height: info.height,
        size_bytes: picture.data().len(),
        picture_type: format!("{:?}", picture.pic_type()),
        data_base64: B64.encode(picture.data()),
    })
}

/// Apply a [`CoverEdit`] to `tag` in place. `label` identifies the file in a
/// [`TagError::Cover`] (its display path). Called from [`super::write_tags`],
/// just before the tag is saved.
pub(crate) fn apply_edit(tag: &mut Tag, edit: &CoverEdit, label: &str) -> Result<(), TagError> {
    match edit {
        CoverEdit::Unset => {}
        CoverEdit::Clear => {
            for i in (0..tag.pictures().len()).rev() {
                tag.remove_picture(i);
            }
        }
        CoverEdit::Set {
            mime,
            data_base64,
            picture_type,
        } => {
            let bytes = B64
                .decode(data_base64)
                .map_err(|e| TagError::Cover(label.to_string(), e.to_string()))?;
            let picture = Picture::unchecked(bytes)
                .pic_type(parse_picture_type(picture_type))
                .mime_type(MimeType::from_str(mime))
                .build();
            for i in (0..tag.pictures().len()).rev() {
                tag.remove_picture(i);
            }
            tag.push_picture(picture);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{read_tags, write_tags, TagEdits};
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

    #[test]
    fn cover_art_round_trip() {
        let path = synth_wav();
        let png = tiny_png(1, 1);
        write_tags(
            &path,
            &TagEdits {
                cover: CoverEdit::Set {
                    mime: "image/png".to_string(),
                    data_base64: B64.encode(&png),
                    picture_type: "CoverFront".to_string(),
                },
                ..Default::default()
            },
        )
        .unwrap();

        let read = read_tags(&path).unwrap();
        let cover = read.cover.expect("cover art should be present");
        assert_eq!(cover.mime, "image/png");
        assert_eq!(cover.width, 1);
        assert_eq!(cover.height, 1);
        assert_eq!(cover.picture_type, "CoverFront");
        assert_eq!(B64.decode(&cover.data_base64).unwrap(), png);

        // Clearing removes it.
        write_tags(
            &path,
            &TagEdits {
                cover: CoverEdit::Clear,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(read_tags(&path).unwrap().cover.is_none());
    }

    /// The tag panel's "change the cover's role" control (relabel the
    /// existing artwork as Back Cover, Artist, etc. without touching the
    /// image bytes) round-trips through the same `CoverEdit::Set` path as a
    /// brand-new image — only `picture_type` differs.
    #[test]
    fn cover_art_role_is_written_and_read_back() {
        let path = synth_wav();
        let png = tiny_png(1, 1);
        write_tags(
            &path,
            &TagEdits {
                cover: CoverEdit::Set {
                    mime: "image/png".to_string(),
                    data_base64: B64.encode(&png),
                    picture_type: "CoverBack".to_string(),
                },
                ..Default::default()
            },
        )
        .unwrap();

        let cover = read_tags(&path)
            .unwrap()
            .cover
            .expect("cover art should be present");
        assert_eq!(cover.picture_type, "CoverBack");
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
                cover: CoverEdit::Set {
                    mime: "image/png".to_string(),
                    data_base64: B64.encode(&png),
                    picture_type: "SomethingLoftyDoesntKnow".to_string(),
                },
                ..Default::default()
            },
        )
        .unwrap();

        let cover = read_tags(&path)
            .unwrap()
            .cover
            .expect("cover art should be present");
        assert_eq!(cover.picture_type, "Other");
    }
}
