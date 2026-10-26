//! Embedded, platform-independent text with clipping and Unicode-safe ellipsis.

use super::canvas::Canvas;
use crate::{Color, Error};
use fontdue::{Font, FontSettings};
use std::sync::{Arc, OnceLock};

pub(super) fn font(custom: Option<&[u8]>) -> Result<Arc<Font>, Error> {
    static BUNDLED: OnceLock<Result<Arc<Font>, String>> = OnceLock::new();
    let parse = |bytes: &[u8]| {
        Font::from_bytes(bytes, FontSettings::default())
            .map(Arc::new)
            .map_err(str::to_string)
    };
    match custom {
        Some(bytes) => parse(bytes).map_err(Error::Render),
        None => BUNDLED
            .get_or_init(|| parse(include_bytes!("../../assets/NotoSans-Variable.ttf")))
            .clone()
            .map_err(Error::Render),
    }
}

pub(super) fn clean(value: &str) -> String {
    value
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(*c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .take(4096)
        .collect()
}

pub(super) fn width(font: &Font, value: &str, size: f32) -> f32 {
    let mut previous = None;
    let mut width = 0.0;
    for character in value.chars() {
        if let Some(previous) = previous {
            width += font
                .horizontal_kern(previous, character, size)
                .unwrap_or(0.0);
        }
        width += font.metrics(character, size).advance_width;
        previous = Some(character);
    }
    width
}

pub(super) fn draw(
    image: &mut Canvas,
    font: &Font,
    value: &str,
    point: (f32, f32),
    size: f32,
    color: Color,
    max_width: f32,
) {
    if max_width <= 0.0 {
        return;
    }
    let mut value = clean(value);
    if width(font, &value, size) > max_width {
        while !value.is_empty() && width(font, &format!("{value}…"), size) > max_width {
            value.pop();
        }
        value.push('…');
    }
    let ascent = font
        .horizontal_line_metrics(size)
        .map(|m| m.ascent)
        .unwrap_or(size);
    let mut x = point.0;
    let mut previous = None;
    for character in value.chars() {
        if let Some(previous) = previous {
            x += font
                .horizontal_kern(previous, character, size)
                .unwrap_or(0.0);
        }
        let (metrics, bitmap) = font.rasterize(character, size);
        let left = x.round() as i32 + metrics.xmin;
        let top = (point.1 + ascent).round() as i32 - metrics.height as i32 - metrics.ymin;
        for (index, &alpha) in bitmap.iter().enumerate() {
            if metrics.width > 0 {
                image.blend(
                    left + (index % metrics.width) as i32,
                    top + (index / metrics.width) as i32,
                    color,
                    alpha,
                );
            }
        }
        x += metrics.advance_width;
        previous = Some(character);
    }
}
