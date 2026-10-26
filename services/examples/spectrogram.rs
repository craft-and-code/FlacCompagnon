//! Generate an inspection PNG with the same service the desktop application uses.

use flaccompagnon_core::spectrogram::{AnalysisOptions, Palette, RenderOptions};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let input = Path::new(
        args.get(1)
            .ok_or("usage: spectrogram INPUT OUTPUT [green|blue|amber|violet] [WIDTH HEIGHT]")?,
    );
    let output = Path::new(args.get(2).ok_or("missing PNG output")?);
    let palette = match args.get(3).map(String::as_str).unwrap_or("green") {
        "green" => Palette::green(),
        "blue" => Palette::blue(),
        "amber" => Palette::amber(),
        "violet" => Palette::violet(),
        _ => return Err("unknown palette".into()),
    };
    let mut analysis = AnalysisOptions::default();
    if let Some(width) = args.get(4) {
        analysis.width = width.parse()?;
    }
    if let Some(height) = args.get(5) {
        analysis.height = height.parse()?;
    }
    let style = RenderOptions {
        palette,
        ..Default::default()
    };
    flaccompagnon_core::report::validate_destination(output)?;
    let ffmpeg = std::env::var("FLACCOMPAGNON_FFMPEG").ok();
    let png = flaccompagnon_services::spectrogram::render_png(
        input,
        &analysis,
        &style,
        "FlacCompagnon",
        ffmpeg.as_deref(),
        &|| false,
    )?;
    flaccompagnon_core::report::write_atomic_bytes(output, &png)?;
    println!(
        "{} ({} × {} plot; lossless PNG)",
        output.display(),
        analysis.width,
        analysis.height
    );
    Ok(())
}
