//! Explicit physical annotations, validated against the measured axes.
use super::{canvas::Canvas, text, MarkerAxis, RenderOptions};
use crate::{Error, Spectrogram};
use fontdue::Font;
pub(super) fn draw(
    image: &mut Canvas,
    font: &Font,
    data: &Spectrogram,
    options: &RenderOptions,
) -> Result<(), Error> {
    let [left, top, ..] = options.margins;
    let (width, height) = data.dimensions();
    let right = left + width;
    let bottom = top + height;
    let size = options.font_size;
    let scale = size / 15.0;
    let write = |image: &mut Canvas, label: &str, x: f32, y: f32, px: f32, color, limit| {
        text::draw(image, font, label, (x, y), px, color, limit)
    };
    for marker in &options.markers {
        let fraction = match marker.axis {
            MarkerAxis::Frequency => data.fraction(marker.value),
            MarkerAxis::Time => marker.value / data.duration(),
        };
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return Err(Error::Configuration(
                "marker lies outside the plotted physical axis".into(),
            ));
        }
        let (x, y) = match marker.axis {
            MarkerAxis::Frequency => {
                let y = bottom
                    .saturating_sub((fraction * f64::from(height)) as u32)
                    .min(bottom - 1);
                image.horizontal(left, y, width, marker.color, marker.dashed);
                (left + 12, y.saturating_sub((30.0 * scale) as u32).max(top))
            }
            MarkerAxis::Time => {
                let x = (left + (fraction * f64::from(width)) as u32).min(right - 1);
                image.vertical(x, top, height, marker.color, marker.dashed);
                (x + 8, top + 8)
            }
        };
        write(
            image,
            &marker.label,
            x as f32,
            y as f32,
            size * 0.85,
            marker.color,
            right.saturating_sub(x) as f32,
        );
    }
    Ok(())
}
