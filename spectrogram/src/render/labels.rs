//! Host-provided image headings and footer labels.
use super::{canvas::Canvas, text, ImageLabels, RenderOptions};
use crate::Spectrogram;
use fontdue::Font;
pub(super) fn draw(
    image: &mut Canvas,
    font: &Font,
    data: &Spectrogram,
    labels: &ImageLabels,
    options: &RenderOptions,
) {
    let [left, top, _, bottom_margin] = options.margins;
    let (width, height) = data.dimensions();
    let right = left + width;
    let bottom = top + height;
    let size = options.font_size;
    let scale = size / 15.0;
    let theme = &options.theme;
    let write = |image: &mut Canvas, label: &str, x: f32, y: f32, px: f32, color, limit| {
        text::draw(image, font, label, (x, y), px, color, limit)
    };
    if top as f32 >= size * 4.0 {
        write(
            image,
            "S P E C T R O G R A M",
            left as f32,
            16.0 * scale,
            size * 0.8,
            theme.muted,
            width as f32 * 0.45,
        );
        write(
            image,
            "Frequency × time",
            left as f32,
            42.0 * scale,
            size * 1.6,
            theme.text,
            width as f32 * 0.45,
        );
        let label = text::clean(&labels.generator);
        let label_width = text::width(font, &label, size * 1.9).min(width as f32 * 0.5);
        write(
            image,
            &label,
            right as f32 - label_width,
            18.0 * scale,
            size * 1.9,
            theme.text,
            width as f32 * 0.5,
        );
    }
    if bottom_margin as f32 >= size * 7.0 {
        let divider = bottom + (68.0 * scale) as u32;
        image.horizontal(left, divider, width, theme.border, false);
        let description = text::clean(&labels.audio_description);
        let description_width = text::width(font, &description, size).min(width as f32 * 0.45);
        let title_width = (width as f32 - description_width - size * 2.0).max(0.0);
        write(
            image,
            &labels.title,
            left as f32,
            divider as f32 + 14.0 * scale,
            size * 1.4,
            theme.text,
            title_width,
        );
        write(
            image,
            &description,
            right as f32 - description_width,
            divider as f32 + 18.0 * scale,
            size,
            theme.text,
            description_width + 1.0,
        );
        write(
            image,
            &labels.artist,
            left as f32,
            divider as f32 + 48.0 * scale,
            size * 0.85,
            theme.muted,
            title_width,
        );
        let detail = format!(
            "{:?} {} · {:?} / pixel",
            data.options.window,
            data.fft_size(),
            data.options.aggregation
        );
        let detail_width = text::width(font, &detail, size * 0.7).min(width as f32 * 0.45);
        write(
            image,
            &detail,
            right as f32 - detail_width,
            divider as f32 + 50.0 * scale,
            size * 0.7,
            theme.muted,
            detail_width + 1.0,
        );
    }
}
