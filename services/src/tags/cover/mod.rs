//! Cover-art payloads and the public entry points used by the tag panel.
//!
//! Image loading, format recognition, and resource limits live in `image`;
//! `embedded` owns extracting pictures from tags and applying sparse role edits.
//! The public paths stay available here and through [`crate::tags`].

mod dimensions;
mod embedded;
mod image;

use serde::{Deserialize, Serialize};

pub(crate) use embedded::{apply_edits, extract_all};
pub use image::{cover_from_bytes, read_cover_file, write_cover_file, MAX_COVER_BYTES};

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

/// An edit to one picture role. An empty edit list leaves every picture alone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CoverEdit {
    /// Remove pictures with this role only.
    Clear {
        /// Role of the pictures to remove.
        picture_type: String,
    },
    /// Replace pictures with this role with one image.
    Set {
        /// MIME type of the replacement image.
        mime: String,
        /// Replacement image bytes, base64-encoded.
        data_base64: String,
        /// Role to write the image under (see `parse_picture_type`).
        picture_type: String,
    },
}

#[cfg(test)]
#[path = "../../../tests/unit/tags/cover.rs"]
mod tests;
