//! File-to-spectrogram adapter over the independent Rust renderer.
//!
//! Two bounded decode passes count real frames, then place every FFT window on
//! that exact timeline. Untrusted duration hints cannot crop the end or stretch
//! the axis. No complete track or generated image is written to disk here.

use crate::{AnalysisError, BasicInfo};
pub use flaccompagnon_spectrogram::*;
use std::path::Path;

/// Measure an audio file at its native decoded rate, without an external renderer.
/// DSD64/128 mono/stereo uses the shared native wideband decoder.
/// `ffmpeg` remains a compatibility option for other existing DSD rates.
pub fn analyze_file(
    path: &Path,
    options: &AnalysisOptions,
    ffmpeg: Option<&str>,
    cancelled: &dyn Fn() -> bool,
) -> Result<(Spectrogram, BasicInfo), AnalysisError> {
    if cancelled() {
        return Err(AnalysisError::Decode("cancelled".into()));
    }
    let before = std::fs::metadata(path)?;
    if !before.is_file() {
        return Err(AnalysisError::Decode(
            "audio source is not a regular file".into(),
        ));
    }
    let (info, frames) = visit(path, ffmpeg, &mut |_| Ok(()), cancelled)?;
    let mut builder = SpectrogramBuilder::new(
        SignalInfo {
            sample_rate: info.sample_rate,
            channels: info.channels,
            frames,
        },
        options.clone(),
    )
    .map_err(convert_error)?;
    let (second_info, second_frames) = visit(
        path,
        ffmpeg,
        &mut |samples| {
            builder
                .push_interleaved_cancellable(samples, cancelled)
                .map_err(convert_error)
        },
        cancelled,
    )?;
    let after = std::fs::metadata(path)?;
    if frames != second_frames
        || info.sample_rate != second_info.sample_rate
        || info.channels != second_info.channels
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
    {
        return Err(AnalysisError::Decode(
            "audio source changed during spectrogram generation".into(),
        ));
    }
    let data = builder
        .finish_cancellable(cancelled)
        .map_err(convert_error)?;
    Ok((data, info))
}

/// Format actual decoded properties for a standalone image; unknown depth stays unknown.
pub fn audio_description(info: &BasicInfo) -> String {
    let rate = f64::from(info.sample_rate) / 1000.0;
    let depth = info
        .bits
        .map(|bits| format!("{bits}-bit"))
        .unwrap_or_else(|| info.format.clone());
    let channels = match info.channels {
        1 => "Mono".into(),
        2 => "Stereo".into(),
        count => format!("{count} channels"),
    };
    format!("{rate} kHz · {depth} · {channels}")
}

/// Whether a source uses the optional external DSD decoder rather than native PCM decoding.
pub fn requires_dsd_decoder(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("dsf") || extension.eq_ignore_ascii_case("dff")
        })
}

fn visit(
    path: &Path,
    ffmpeg: Option<&str>,
    sink: &mut dyn FnMut(&[f32]) -> Result<(), AnalysisError>,
    cancelled: &dyn Fn() -> bool,
) -> Result<(BasicInfo, u64), AnalysisError> {
    if requires_dsd_decoder(path) {
        let dsd = crate::dsd::parse(path)?;
        let program = ffmpeg.ok_or_else(|| {
            AnalysisError::Decode(
                "DSD content decoding requires FFmpeg; PCM spectrograms do not".into(),
            )
        })?;
        let rate = dsd.sample_rate / 8;
        let frames =
            crate::decode::dsd::visit_dsd_pcm(program, path, dsd.channels, rate, sink, cancelled)?;
        Ok((
            BasicInfo {
                sample_rate: rate,
                channels: dsd.channels,
                bits: None,
                format: format!("{} (decoded PCM)", dsd.label()),
            },
            frames,
        ))
    } else {
        crate::decode::visit_pcm(path, sink, cancelled)
    }
}

fn convert_error(error: flaccompagnon_spectrogram::Error) -> AnalysisError {
    AnalysisError::Decode(error.to_string())
}

#[cfg(test)]
#[path = "../tests/unit/spectrogram.rs"]
mod tests;
