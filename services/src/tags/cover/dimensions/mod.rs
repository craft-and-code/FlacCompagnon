//! Read image geometry without allocating or decoding a raster.
//!
//! Lofty's documented metadata reader supports PNG/JPEG only. The other
//! accepted formats use bounded reads of their specified headers instead;
//! zero or indeterminate dimensions never bypass the resource budget.

mod gif;
mod tiff;
mod webp;

use lofty::picture::PictureInformation;

// 64 megapixels already require 256 MB as decoded RGBA.
const MAX_PIXELS: u64 = 64_000_000;

pub(super) struct ImageHeader {
    pub(super) mime: &'static str,
    pub(super) width: u32,
    pub(super) height: u32,
}

pub(super) fn inspect(bytes: &[u8]) -> Option<ImageHeader> {
    let (mime, dimensions) = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        let info = PictureInformation::from_png(bytes).ok()?;
        ("image/png", (info.width, info.height))
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        let info = PictureInformation::from_jpeg(bytes).ok()?;
        ("image/jpeg", (info.width, info.height))
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        ("image/gif", gif::dimensions(bytes)?)
    } else if bytes.starts_with(b"BM") {
        ("image/bmp", bmp_dimensions(bytes)?)
    } else if bytes.get(..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        ("image/webp", webp::dimensions(bytes)?)
    } else if bytes.starts_with(b"II\x2a\0") || bytes.starts_with(b"MM\0\x2a") {
        ("image/tiff", tiff::dimensions(bytes)?)
    } else {
        return None;
    };
    let (width, height) = bounded_dimensions(dimensions.0, dimensions.1)?;
    Some(ImageHeader {
        mime,
        width,
        height,
    })
}

fn bounded_dimensions(width: u32, height: u32) -> Option<(u32, u32)> {
    (width > 0 && height > 0 && u64::from(width) * u64::from(height) <= MAX_PIXELS)
        .then_some((width, height))
}

// Microsoft's BITMAPCOREHEADER/BITMAPINFOHEADER specs: width is positive;
// a negative INFO height denotes a top-down raster, not a negative size.
// https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapinfoheader
fn bmp_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let dib_size = usize::try_from(le_u32(bytes, 14)?).ok()?;
    bytes.get(14..14usize.checked_add(dib_size)?)?;
    if dib_size == 12 {
        if le_u16(bytes, 22)? != 1 {
            return None;
        }
        return bounded_dimensions(u32::from(le_u16(bytes, 18)?), u32::from(le_u16(bytes, 20)?));
    }
    if !matches!(dib_size, 40 | 52 | 56 | 108 | 124) || le_u16(bytes, 26)? != 1 {
        return None;
    }
    // BI_JPEG/BI_PNG contain an independently encoded raster; DIB geometry
    // alone cannot bound it. Keep these uncommon wrappers unsupported.
    if matches!(le_u32(bytes, 30)?, 4 | 5) {
        return None;
    }
    let width = i32::from_le_bytes(bytes.get(18..22)?.try_into().ok()?);
    let height = i32::from_le_bytes(bytes.get(22..26)?.try_into().ok()?).checked_abs()?;
    bounded_dimensions(u32::try_from(width).ok()?, u32::try_from(height).ok()?)
}

fn le_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn le_u24(bytes: &[u8], at: usize) -> Option<u32> {
    let value = bytes.get(at..at.checked_add(3)?)?;
    Some(
        u32::from(value.first().copied()?)
            | (u32::from(value.get(1).copied()?) << 8)
            | (u32::from(value.get(2).copied()?) << 16),
    )
}

fn le_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

#[cfg(test)]
#[path = "../../../../tests/unit/tags/cover/dimensions.rs"]
mod tests;
