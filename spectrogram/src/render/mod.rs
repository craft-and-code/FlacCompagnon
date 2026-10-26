//! Image composition and lossless encoding, independent of measured powers.

mod axes;
mod canvas;
mod labels;
mod markers;
mod text;

use crate::{Color, Error, Palette, Spectrogram};
use canvas::Canvas;
use std::sync::Arc;

/// Text supplied by the host, with empty strings hiding individual labels.
#[derive(Debug, Clone, Default)]
pub struct ImageLabels {
    /// Generating application, drawn at the top right.
    pub generator: String,
    /// Track title, drawn at the bottom left.
    pub title: String,
    /// Artist or supplementary title information.
    pub artist: String,
    /// Actual sample rate/depth/channels, drawn at the bottom right.
    pub audio_description: String,
}

/// Which physical axis carries a marker value.
#[derive(Debug, Clone, Copy)]
pub enum MarkerAxis {
    /// A horizontal marker at a frequency in Hz.
    Frequency,
    /// A vertical marker at a time in seconds.
    Time,
}

/// Optional measured annotation; a marker is not an authenticity verdict.
#[derive(Debug, Clone)]
pub struct Marker {
    /// Axis and physical unit of `value`.
    pub axis: MarkerAxis,
    /// Frequency in Hz or time in seconds, within the plotted view.
    pub value: f64,
    /// Optional caption.
    pub label: String,
    /// Opaque sRGB line/text color.
    pub color: Color,
    /// Use a dashed rather than solid line.
    pub dashed: bool,
}

/// Frame colors, independent of the spectral palette.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Image background.
    pub background: Color,
    /// Main labels.
    pub text: Color,
    /// Secondary labels and axis ticks.
    pub muted: Color,
    /// Frame, dividers and optional grid.
    pub border: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: [17, 18, 19],
            text: [241, 240, 235],
            muted: [164, 166, 167],
            border: [48, 50, 52],
        }
    }
}

/// Appearance options; changing these does not repeat decoding or FFT work.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Continuous color map, monochrome green by default.
    pub palette: Palette,
    /// Lower displayed dBFS/bin; quieter values use the first palette color.
    pub min_db: f64,
    /// Upper displayed dBFS/bin; louder values use the last palette color.
    pub max_db: f64,
    /// Colors of the surrounding frame.
    pub theme: Theme,
    /// Left, top, right and bottom margins in pixels.
    pub margins: [u32; 4],
    /// Base text size in pixels; headings use proportional sizes.
    pub font_size: f32,
    /// Optional custom font bytes; `None` uses embedded OFL Noto Sans.
    pub font: Option<Arc<[u8]>>,
    /// Draw frequency/time ticks and units.
    pub show_axes: bool,
    /// Draw the level color bar and its dB labels.
    pub show_legend: bool,
    /// Draw a subtle grid over the measured image.
    pub show_grid: bool,
    /// Optional frequency/time annotations.
    pub markers: Vec<Marker>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            palette: Palette::default(),
            min_db: -120.0,
            max_db: 0.0,
            theme: Theme::default(),
            margins: [104, 108, 144, 168],
            font_size: 15.0,
            font: None,
            show_axes: true,
            show_legend: true,
            show_grid: false,
            markers: Vec::new(),
        }
    }
}

/// Encode calibrated measurements as a lossless, tagged sRGB PNG in memory.
/// No filesystem or external process is used, including for text rendering.
pub fn render_png(
    data: &Spectrogram,
    labels: &ImageLabels,
    options: &RenderOptions,
) -> Result<Vec<u8>, Error> {
    options.palette.validate()?;
    if !options.min_db.is_finite()
        || !options.max_db.is_finite()
        || options.min_db >= options.max_db
        || !options.font_size.is_finite()
        || !(6.0..=96.0).contains(&options.font_size)
        || options.markers.len() > 256
        || options.margins.iter().any(|&n| n > 4096)
        || options
            .font
            .as_ref()
            .is_some_and(|font| font.len() > 16 * 1024 * 1024)
    {
        return Err(Error::Configuration(
            "invalid rendering bounds, font size or marker count".into(),
        ));
    }
    let (plot_width, plot_height) = data.dimensions();
    let [left, top, right, bottom] = options.margins;
    let width = plot_width
        .checked_add(left)
        .and_then(|n| n.checked_add(right));
    let height = plot_height
        .checked_add(top)
        .and_then(|n| n.checked_add(bottom));
    let (width, height) = width
        .zip(height)
        .filter(|&(w, h)| u64::from(w) * u64::from(h) <= crate::options::MAX_PIXELS as u64)
        .ok_or_else(|| Error::Configuration("final image exceeds the pixel limit".into()))?;
    let font = text::font(options.font.as_deref())?;
    let mut image = Canvas::new(width, height, options.theme.background)?;
    for row in 0..plot_height {
        for column in 0..plot_width {
            let db = data
                .level_db(column, row)
                .ok_or_else(|| Error::Render("inconsistent spectral raster".into()))?;
            let color = options
                .palette
                .color((db - options.min_db) / (options.max_db - options.min_db));
            image.pixel(left + column, top + plot_height - 1 - row, color);
        }
    }
    axes::draw(&mut image, &font, data, options);
    labels::draw(&mut image, &font, data, labels, options);
    markers::draw(&mut image, &font, data, options)?;
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
        let render_error = |e: png::EncodingError| Error::Render(e.to_string());
        encoder
            .add_itxt_chunk("Software".into(), text::clean(&labels.generator))
            .map_err(render_error)?;
        encoder
            .add_itxt_chunk("Title".into(), text::clean(&labels.title))
            .map_err(render_error)?;
        if options.font.is_none() {
            // Keep the bundled font's full redistribution notice in every
            // standalone client binary as well as the self-contained PNG.
            encoder
                .add_ztxt_chunk(
                    "FontLicense".into(),
                    include_str!("../../assets/OFL.txt").into(),
                )
                .map_err(render_error)?;
        }
        encoder.add_text_chunk("Spectrogram".into(), format!(
            "FFT={}; hop={}; window={:?}; channels={:?}; pooling={:?}; dBFS/bin={}..{}; sample_rate={}; frames={}",
            data.fft_size(), data.hop_size(), data.options.window, data.options.channels,
            data.options.aggregation, options.min_db, options.max_db,
            data.signal().sample_rate, data.signal().frames)).map_err(render_error)?;
        let mut writer = encoder.write_header().map_err(render_error)?;
        writer
            .write_image_data(&image.pixels)
            .map_err(render_error)?;
        writer.finish().map_err(render_error)?;
    }
    Ok(bytes)
}

#[cfg(test)]
#[path = "../../tests/unit/render.rs"]
mod tests;
