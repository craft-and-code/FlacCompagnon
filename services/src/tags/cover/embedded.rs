//! Extract embedded cover art and apply edits to individual picture roles.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::tag::Tag;

use super::super::TagError;
use super::image::{decode_cover, inspect_image};
use super::{CoverArt, CoverEdit};

/// Maps a picture-role name — one of the strings [`extract_all`] produces for
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

/// Read every embedded picture so the role selector can show each one.
pub(crate) fn extract_all(tag: &Tag, label: &str) -> Result<Vec<CoverArt>, TagError> {
    tag.pictures()
        .iter()
        .map(|picture| {
            let info = inspect_image(picture.data(), label)?;
            Ok(CoverArt {
                mime: info.mime.to_string(),
                width: info.width,
                height: info.height,
                size_bytes: picture.data().len(),
                picture_type: format!("{:?}", picture.pic_type()),
                data_base64: B64.encode(picture.data()),
            })
        })
        .collect()
}

/// Apply sparse picture edits to `tag` in place. `label` identifies the file in a
/// [`TagError::Cover`] (its display path). Called from [`crate::tags::write_tags`],
/// just before the tag is saved.
pub(crate) fn apply_edits(tag: &mut Tag, edits: &[CoverEdit], label: &str) -> Result<(), TagError> {
    let mut touched_roles = Vec::new();
    for edit in edits {
        let role = match edit {
            CoverEdit::Clear { picture_type } | CoverEdit::Set { picture_type, .. } => {
                parse_picture_type(picture_type)
            }
        };
        // A batch import stages one image per role; copying tags can include
        // several images with the same role, which must all survive.
        if !touched_roles.contains(&role) {
            for i in (0..tag.pictures().len()).rev() {
                if tag.pictures()[i].pic_type() == role {
                    tag.remove_picture(i);
                }
            }
            touched_roles.push(role);
        }
        if let CoverEdit::Set { data_base64, .. } = edit {
            let bytes = decode_cover(data_base64, label)?;
            let info = inspect_image(&bytes, label)?;
            let picture = Picture::unchecked(bytes)
                .pic_type(role)
                .mime_type(MimeType::from_str(info.mime))
                .build();
            tag.push_picture(picture);
        }
    }
    Ok(())
}
