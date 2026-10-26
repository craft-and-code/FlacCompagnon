//! Application adapter for the reusable Rust spectrogram and PNG engine.

use flaccompagnon_spectrogram::{AnalysisOptions, Palette, PlotSize, RenderOptions};
use serde::Deserialize;
use std::path::Path;

/// Preferred plot size; the legacy `half` storage value now means 1200×627.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpectrogramSize {
    /// Standard plot, preserving the former 900:470 proportions.
    #[default]
    Half,
    /// Existing large plot, 1800×940.
    Full,
}

impl SpectrogramSize {
    fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Half => PlotSize::Standard.dimensions(),
            Self::Full => PlotSize::Large.dimensions(),
        }
    }
}

/// Monochrome palette selected in the native menu.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpectrogramColor {
    /// Shared green default in FlacCompagnon and Aède.
    #[default]
    Green,
    /// Blue levels.
    Blue,
    /// Amber levels.
    Amber,
    /// Violet levels.
    Violet,
}

impl SpectrogramColor {
    fn palette(self) -> Palette {
        match self {
            Self::Green => Palette::green(),
            Self::Blue => Palette::blue(),
            Self::Amber => Palette::amber(),
            Self::Violet => Palette::violet(),
        }
    }
}

/// Render and atomically publish a lossless PNG, leaving failed/cancelled output intact.
pub fn render(
    input: &Path,
    output: &Path,
    size: SpectrogramSize,
    color: SpectrogramColor,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if !output
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        return Err("Spectrogram destination must use a .png extension.".into());
    }
    flaccompagnon_core::report::validate_destination(output).map_err(|e| e.to_string())?;
    let (width, height) = size.dimensions();
    let options = AnalysisOptions {
        width,
        height,
        ..Default::default()
    };
    let dsd = flaccompagnon_core::spectrogram::requires_dsd_decoder(input);
    let decoder = dsd.then(crate::ffmpeg::resolve_ffmpeg).flatten();
    let style = RenderOptions {
        palette: color.palette(),
        ..Default::default()
    };
    let bytes = flaccompagnon_services::spectrogram::render_png(
        input,
        &options,
        &style,
        "FlacCompagnon",
        decoder.as_deref(),
        cancelled,
    )?;
    flaccompagnon_core::report::write_atomic_bytes(output, &bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "../tests/unit/spectrogram.rs"]
mod tests;
