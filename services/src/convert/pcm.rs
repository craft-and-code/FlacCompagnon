//! The PCM pipeline between a decoder and an encoder: reading a source into
//! samples, resampling them, and quantizing them to integers.
//!
//! Split out of [`super`] rather than left beside the batch-planning code it
//! has nothing to do with — this is the whole of the conversion path that
//! touches sample values themselves, and none of it is specific to one
//! output format ([`super::opus`]/[`super::mp3`] resample, all four
//! quantize, every one of them decodes first). The two helpers stay
//! re-exported from [`super`] so the encoders keep calling
//! `super::f32_to_ints` / `super::resample_linear` unchanged.

use std::path::Path;

use super::ConvertError;
use crate::decode;

/// [`decode::decode_to_pcm`], but polling `is_cancelled` between packets.
/// Deliberately not a flag added to `decode_to_pcm` itself: that one is also
/// the playback engine's path, where there is nothing to cancel, and it would
/// pay for a callback it never uses on every file the app plays.
pub(super) fn decode_cancellable(
    src: &Path,
    is_cancelled: &dyn Fn() -> bool,
) -> Result<decode::PcmAudio, ConvertError> {
    let name = || src.display().to_string();
    let mut decoder = decode::PcmStreamDecoder::open(src)
        .map_err(|e| ConvertError::Decode(name(), e.to_string()))?;
    let sample_rate = decoder.sample_rate;
    let channels = decoder.channels;

    let mut samples: Vec<f32> = Vec::new();
    loop {
        if is_cancelled() {
            return Err(ConvertError::Cancelled(name()));
        }
        let chunk = decoder
            .next_chunk()
            .map_err(|e| ConvertError::Decode(name(), e.to_string()))?;
        match chunk {
            Some(chunk) => samples.extend_from_slice(&chunk),
            None => break,
        }
    }

    if samples.is_empty() {
        return Err(ConvertError::Decode(
            name(),
            "no audio data decoded".to_string(),
        ));
    }
    Ok(decode::PcmAudio {
        samples,
        sample_rate,
        channels,
    })
}

/// Simple linear-interpolation resampler, shared by [`super::opus`] and
/// [`super::mp3`] — both formats only accept a short, fixed list of sample
/// rates (Opus: 8/12/16/24/48kHz; MP3/LAME: the MPEG1/2/2.5 rates), so a
/// source at any other rate (44.1kHz being the common case for music, and any
/// of this app's own hi-res detections being another) needs resampling before
/// either encoder will accept it. Not a mastering-grade resampler with proper
/// anti-aliasing filtering — a windowed-sinc/polyphase resampler would do
/// better, at the cost of a filter design neither format's own lossy coding
/// otherwise needs; both already discard far more information than this
/// step adds at any ordinary music sample rate, which is why the simpler
/// approach was judged good enough here — worth revisiting if it turns out
/// to matter in practice.
pub(crate) fn resample_linear(
    samples: &[f32],
    channels: usize,
    from_hz: u32,
    to_hz: u32,
) -> Vec<f32> {
    if from_hz == to_hz || channels == 0 {
        return samples.to_vec();
    }
    let frames_in = samples.len() / channels;
    if frames_in == 0 {
        return Vec::new();
    }
    let ratio = to_hz as f64 / from_hz as f64;
    let frames_out = ((frames_in as f64) * ratio).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(frames_out * channels);
    for i in 0..frames_out {
        let src_pos = i as f64 / ratio;
        let i0 = (src_pos.floor() as usize).min(frames_in - 1);
        let i1 = (i0 + 1).min(frames_in - 1);
        let frac = (src_pos - i0 as f64) as f32;
        for c in 0..channels {
            let a = samples[i0 * channels + c];
            let b = samples[i1 * channels + c];
            out.push(a + (b - a) * frac);
        }
    }
    out
}

/// Converts normalized `[-1.0, 1.0]` samples to signed integers at
/// `bit_depth`, clamped to that depth's representable range — the reverse of
/// the reconstruction `decode::stream` does for analysis (see its
/// `int_scale` comment there).
pub(crate) fn f32_to_ints(samples: &[f32], bit_depth: u32) -> Vec<i32> {
    let scale = 2f32.powi(bit_depth as i32 - 1);
    let max = scale - 1.0;
    let min = -scale;
    samples
        .iter()
        .map(|&s| (s * scale).round().clamp(min, max) as i32)
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/convert/pcm.rs"]
mod tests;
