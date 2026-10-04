//! Load or export plain cover images with bounded byte and pixel budgets.

use std::{io::Read, path::Path};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

use super::super::TagError;
use super::dimensions::{self, ImageHeader};
use super::CoverArt;

/// Maximum encoded image size accepted from disk, a lookup, or a tag edit.
/// The same 12 MiB limit bounds the bytes sent to the webview in every path.
pub const MAX_COVER_BYTES: usize = 12 * 1024 * 1024;

/// Build a [`CoverArt`] from raw image bytes, from wherever they came from —
/// shared by [`read_cover_file`] (a dropped image) and the online lookup's
/// downloaded artwork (`src-tauri/src/lookup/`). `label` is only used to
/// identify the source in an error message (a file path, or a URL).
/// Geometry must be readable within the byte/pixel budgets. BigTIFF,
/// TIFF SubIFDs and BMP JPEG/PNG wrappers are unsupported.
pub fn cover_from_bytes(bytes: Vec<u8>, label: &str) -> Result<CoverArt, TagError> {
    let info = inspect_image(&bytes, label)?;
    let size_bytes = bytes.len();
    let data_base64 = B64.encode(&bytes);
    Ok(CoverArt {
        mime: info.mime.to_string(),
        width: info.width,
        height: info.height,
        size_bytes,
        picture_type: "CoverFront".to_string(),
        data_base64,
    })
}

/// Read a plain image file (dropped onto the cover box, not an audio file)
/// into a [`CoverArt`] ready to stage as a [`super::CoverEdit::Set`] — backs
/// "drop an image to replace every selected file's cover".
pub fn read_cover_file(path: &Path) -> Result<CoverArt, TagError> {
    let label = path.display().to_string();
    let mut bytes = Vec::new();
    flaccompagnon_core::report::open_regular_file(path)
        .and_then(|file| {
            file.take((MAX_COVER_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
        })
        .map_err(|e| TagError::Cover(label.clone(), e.to_string()))?;
    cover_from_bytes(bytes, &label)
}

/// Write raw base64 image bytes out to `dest` as a plain file — the inverse
/// of [`read_cover_file`], backing the tag panel's "extract this cover next
/// to the audio file" button. `dest`'s extension is whatever the caller
/// already decided from the image's own MIME type. Only image extensions are
/// accepted; an existing regular image is replaced atomically, while links
/// and special files are rejected.
pub fn write_cover_file(dest: &Path, data_base64: &str) -> Result<(), TagError> {
    let extension = dest
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(
        extension.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tif" | "tiff" | "img"
    ) {
        return Err(TagError::Cover(
            dest.display().to_string(),
            "destination must use an image extension".to_string(),
        ));
    }
    let bytes = decode_cover(data_base64, &dest.display().to_string())?;
    inspect_image(&bytes, &dest.display().to_string())?;
    flaccompagnon_core::report::write_atomic_bytes(dest, &bytes)
        .map_err(|e| TagError::Cover(dest.display().to_string(), e.to_string()))
}

pub(super) fn decode_cover(data_base64: &str, label: &str) -> Result<Vec<u8>, TagError> {
    // Check before decoding: base64 adds at most four bytes per three input
    // bytes, so a rejected edit does not allocate a second huge buffer.
    if data_base64.len() > MAX_COVER_BYTES.div_ceil(3) * 4 {
        return Err(TagError::Cover(
            label.to_string(),
            "image exceeds the 12 MiB limit".to_string(),
        ));
    }
    let bytes = B64
        .decode(data_base64)
        .map_err(|e| TagError::Cover(label.to_string(), e.to_string()))?;
    validate_cover_size(bytes.len(), label)?;
    Ok(bytes)
}

pub(super) fn validate_cover_size(size: usize, label: &str) -> Result<(), TagError> {
    if size > MAX_COVER_BYTES {
        return Err(TagError::Cover(
            label.to_string(),
            "image exceeds the 12 MiB limit".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn inspect_image(bytes: &[u8], label: &str) -> Result<ImageHeader, TagError> {
    validate_cover_size(bytes.len(), label)?;
    dimensions::inspect(bytes).ok_or_else(|| {
        TagError::Cover(
            label.to_string(),
            "invalid image header or dimensions (maximum 64 megapixels)".to_string(),
        )
    })
}
