//! Compose native spectral measurements with optional track tags for a PNG.

use flaccompagnon_core::spectrogram::{self, AnalysisOptions, ImageLabels, RenderOptions};
use std::path::Path;

/// Build a lossless PNG in memory. The host owns destination validation and writes.
/// Tags are optional; an unreadable or absent title falls back to the file stem.
/// Only DSF/DFF content uses the optional `ffmpeg` decoder.
pub fn render_png(
    input: &Path,
    analysis: &AnalysisOptions,
    style: &RenderOptions,
    generator: &str,
    ffmpeg: Option<&str>,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<u8>, String> {
    let (data, info) =
        spectrogram::analyze_file(input, analysis, ffmpeg, cancelled).map_err(|e| e.to_string())?;
    let tags = crate::tags::read_tags(input).ok();
    let labels = ImageLabels {
        generator: generator.into(),
        title: tags
            .as_ref()
            .and_then(|tags| tags.title.clone())
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| {
                input
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            }),
        artist: tags.and_then(|tags| tags.artist).unwrap_or_default(),
        audio_description: spectrogram::audio_description(&info),
    };
    if cancelled() {
        return Err("Spectrogram generation cancelled".into());
    }
    let png = spectrogram::render_png(&data, &labels, style).map_err(|e| e.to_string())?;
    if cancelled() {
        return Err("Spectrogram generation cancelled".into());
    }
    Ok(png)
}
