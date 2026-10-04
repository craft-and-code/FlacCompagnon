//! GIF89a screen/frame metadata, per <https://www.w3.org/Graphics/GIF/spec-gif89a.txt>.

use super::{bounded_dimensions, le_u16};

pub(super) fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let screen = bounded_dimensions(u32::from(le_u16(bytes, 6)?), u32::from(le_u16(bytes, 8)?))?;
    let mut at = skip_palette(bytes, 13, *bytes.get(10)?)?;
    let mut has_frame = false;
    loop {
        match *bytes.get(at)? {
            0x3b if has_frame => return Some(screen),
            0x21 => {
                at = skip_subblocks(bytes, at.checked_add(2)?)?;
            }
            0x2c => {
                let frame = bytes.get(at..at.checked_add(10)?)?;
                let left = u32::from(le_u16(frame, 1)?);
                let top = u32::from(le_u16(frame, 3)?);
                let (width, height) =
                    bounded_dimensions(u32::from(le_u16(frame, 5)?), u32::from(le_u16(frame, 7)?))?;
                if left.checked_add(width)? > screen.0 || top.checked_add(height)? > screen.1 {
                    return None;
                }
                at = skip_palette(bytes, at.checked_add(10)?, *frame.get(9)?)?;
                let code_size = *bytes.get(at)?;
                if !(2..=8).contains(&code_size) {
                    return None;
                }
                at = skip_subblocks(bytes, at.checked_add(1)?)?;
                has_frame = true;
            }
            _ => return None,
        }
    }
}

fn skip_palette(bytes: &[u8], at: usize, packed: u8) -> Option<usize> {
    let len = if packed & 0x80 == 0 {
        0
    } else {
        3usize.checked_shl(u32::from((packed & 7) + 1))?
    };
    let end = at.checked_add(len)?;
    bytes.get(at..end)?;
    Some(end)
}

fn skip_subblocks(bytes: &[u8], mut at: usize) -> Option<usize> {
    loop {
        let len = usize::from(*bytes.get(at)?);
        at = at.checked_add(1)?;
        let end = at.checked_add(len)?;
        bytes.get(at..end)?;
        at = end;
        if len == 0 {
            return Some(at);
        }
    }
}
