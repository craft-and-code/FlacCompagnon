//! WebP RIFF/VP8X/VP8L geometry from Google's container and lossless specs.
//! <https://developers.google.com/speed/webp/docs/riff_container>
//! <https://developers.google.com/speed/webp/docs/webp_lossless_bitstream_specification>

use super::{bounded_dimensions, le_u16, le_u24, le_u32};

pub(super) fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let end = usize::try_from(le_u32(bytes, 4)?).ok()?.checked_add(8)?;
    scan_chunks(bytes.get(12..end)?, true)
}

fn scan_chunks(bytes: &[u8], allow_frames: bool) -> Option<(u32, u32)> {
    let mut at = 0usize;
    let mut canvas = None;
    let mut image = None;
    while at < bytes.len() {
        let id = bytes.get(at..at.checked_add(4)?)?;
        let len = usize::try_from(le_u32(bytes, at.checked_add(4)?)?).ok()?;
        let start = at.checked_add(8)?;
        let end = start.checked_add(len)?;
        let data = bytes.get(start..end)?;
        let size = match id {
            b"VP8X" if allow_frames && canvas.is_none() && data.len() == 10 => {
                let size = bounded_dimensions(
                    le_u24(data, 4)?.checked_add(1)?,
                    le_u24(data, 7)?.checked_add(1)?,
                )?;
                canvas = Some(size);
                None
            }
            b"VP8 " => Some(vp8_dimensions(data)?),
            b"VP8L" => {
                if data.first() != Some(&0x2f) {
                    return None;
                }
                let bits = le_u32(data, 1)?;
                if bits >> 29 != 0 {
                    return None;
                }
                Some(bounded_dimensions(
                    (bits & 0x3fff) + 1,
                    ((bits >> 14) & 0x3fff) + 1,
                )?)
            }
            b"ANMF" if allow_frames => {
                let size = bounded_dimensions(
                    le_u24(data, 6)?.checked_add(1)?,
                    le_u24(data, 9)?.checked_add(1)?,
                )?;
                let screen = canvas?;
                if le_u24(data, 0)?.checked_mul(2)?.checked_add(size.0)? > screen.0
                    || le_u24(data, 3)?.checked_mul(2)?.checked_add(size.1)? > screen.1
                    || scan_chunks(data.get(16..)?, false)? != size
                {
                    return None;
                }
                Some(size)
            }
            b"VP8X" | b"ANMF" => return None,
            _ => None,
        };
        if let Some(size) = size {
            if let Some(screen) = canvas {
                if size.0 > screen.0 || size.1 > screen.1 || (id != b"ANMF" && size != screen) {
                    return None;
                }
            }
            image = Some(size);
        }
        at = end.checked_add(len & 1)?;
        bytes.get(end..at)?;
    }
    // An extended header alone is not an image, and must not disguise a
    // missing/corrupt bitstream whose dimensions could not be checked.
    image?;
    canvas.or(image)
}

// VP8 key-frame header, RFC 6386 §9.1: 14-bit dimensions follow the start code.
fn vp8_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if *data.first()? & 1 != 0 || data.get(3..6) != Some(b"\x9d\x01\x2a") {
        return None;
    }
    bounded_dimensions(
        u32::from(le_u16(data, 6)? & 0x3fff),
        u32::from(le_u16(data, 8)? & 0x3fff),
    )
}
