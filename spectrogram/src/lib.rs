//! Streaming, channel-safe spectral measurements and deterministic PNG rendering.
//!
//! No decoder, filesystem access, Tauri or application dependency. Callers feed
//! interleaved PCM, then render the result with independently configurable labels,
//! palettes and layout. PNG encoding preserves every rendered pixel losslessly.
#![warn(missing_docs)]

mod analysis;
mod options;
mod palette;
mod render;

pub use analysis::{Spectrogram, SpectrogramBuilder};
pub use options::*;
pub use palette::{Color, ColorStop, Palette};
pub use render::{render_png, ImageLabels, Marker, MarkerAxis, RenderOptions, Theme};

/// A configuration, input or rendering failure; no partial image is returned.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid or inconsistent caller-supplied options.
    #[error("invalid spectrogram configuration: {0}")]
    Configuration(String),
    /// Invalid samples, incomplete frames or a changed input length.
    #[error("invalid spectrogram audio: {0}")]
    Audio(String),
    /// The caller cancelled the operation.
    #[error("spectrogram generation cancelled")]
    Cancelled,
    /// Font or PNG rendering failed.
    #[error("spectrogram rendering failed: {0}")]
    Render(String),
}
