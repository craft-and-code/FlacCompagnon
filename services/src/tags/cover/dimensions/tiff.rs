//! Classic TIFF 6.0 IFD geometry (ImageWidth 256 / ImageLength 257).
//! Tag definitions: <https://www.loc.gov/preservation/digital/formats/content/tiff_tags.shtml>.

use std::collections::HashSet;

use super::bounded_dimensions;

pub(super) fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let little = bytes.get(..2) == Some(b"II");
    let u16_at = |at: usize| -> Option<u16> {
        let value = bytes.get(at..at.checked_add(2)?)?.try_into().ok()?;
        Some(if little {
            u16::from_le_bytes(value)
        } else {
            u16::from_be_bytes(value)
        })
    };
    let u32_at = |at: usize| -> Option<u32> {
        let value = bytes.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(if little {
            u32::from_le_bytes(value)
        } else {
            u32::from_be_bytes(value)
        })
    };
    let mut at = usize::try_from(u32_at(4)?).ok()?;
    let mut seen = HashSet::new();
    let mut first = None;
    // Overlapping directories must not make a small input repeatedly scan
    // the same large table; valid distinct entries fit in the file once.
    let mut remaining_entries = bytes.len() / 12;
    while at != 0 {
        if at < 8 || !seen.insert(at) {
            return None;
        }
        let count = usize::from(u16_at(at)?);
        remaining_entries = remaining_entries.checked_sub(count)?;
        let start = at.checked_add(2)?;
        let end = start.checked_add(count.checked_mul(12)?)?;
        bytes.get(start..end)?;
        let mut width = None;
        let mut height = None;
        for i in 0..count {
            let entry = start.checked_add(i.checked_mul(12)?)?;
            let tag = u16_at(entry)?;
            // SubIFDs can contain additional rasters outside the next-IFD
            // chain. Refuse this unsupported tree rather than trust only
            // the first page's geometry.
            if tag == 330 {
                return None;
            }
            if !matches!(tag, 256 | 257) {
                continue;
            }
            if u32_at(entry.checked_add(4)?)? != 1 {
                return None;
            }
            let value_at = entry.checked_add(8)?;
            let value = match u16_at(entry.checked_add(2)?)? {
                3 => u32::from(u16_at(value_at)?),
                4 => u32_at(value_at)?,
                _ => return None,
            };
            let field = if tag == 256 { &mut width } else { &mut height };
            if field.replace(value).is_some() {
                return None;
            }
        }
        let size = bounded_dimensions(width?, height?)?;
        first.get_or_insert(size);
        // Every linked IFD is checked, and cycles are refused: a small first
        // page cannot hide a giant second raster or an unbounded metadata loop.
        at = usize::try_from(u32_at(end)?).ok()?;
    }
    first
}
