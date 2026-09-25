//! MP3 encoding via `mp3lame-encoder` (vendors LAME through `mp3lame-sys`, no
//! system library needed).
//!
//! LAME only accepts a short fixed list of input sample rates — 32/44.1/48kHz
//! for MPEG1, 16/22.05/24kHz for MPEG2, 8/11.025/12kHz for MPEG2.5, and
//! nothing above 48kHz at all. That last part matters here specifically:
//! hi-res sources (88.2/96/176.4/192kHz) are exactly what this app spends
//! most of its analysis code detecting, so they are a routine input to this
//! encoder, not an edge case. A source outside the supported set is
//! resampled first with [`super::resample_linear`] (the same helper
//! [`super::opus`] uses) down to the nearest rate LAME accepts, preferring an
//! even division (88.2k -> 44.1k, 96k -> 48k, ...) over the closest rate by
//! raw distance, since that keeps the common hi-res cases landing on the
//! rate a listener would expect.
//!
//! LAME's own encoded output is a bare MP3 frame stream: no ID3 tag is
//! written here (this app's own tag writer handles that separately, the same
//! way it already does for the other three formats), and no VBR — output is
//! constant bitrate at the requested [`Bitrate`] preset, the closest match to
//! the `bitrate_kbps` setting the caller asked for.

use std::mem::MaybeUninit;
use std::path::Path;

use mp3lame_encoder::{Bitrate, Builder, FlushNoGap, InterleavedPcm};

use super::ConvertError;
use crate::decode::PcmAudio;

/// Sample rates LAME accepts as encoder input, across all three MPEG
/// versions it can produce.
const LAME_SUPPORTED_RATES: &[u32] = &[
    8_000, 11_025, 12_000, 16_000, 22_050, 24_000, 32_000, 44_100, 48_000,
];

/// Encode `pcm` as MP3 to `dest`, at the bitrate preset closest to
/// `bitrate_kbps`.
pub(super) fn encode(pcm: &PcmAudio, dest: &Path, bitrate_kbps: u32) -> Result<(), ConvertError> {
    let name = || dest.display().to_string();

    if pcm.channels == 0 || pcm.channels > 2 {
        return Err(ConvertError::Unsupported(
            name(),
            format!(
                "MP3 only supports mono or stereo here (got {} channels)",
                pcm.channels
            ),
        ));
    }

    let target_rate = nearest_supported_rate(pcm.sample_rate);
    let resampled = if target_rate == pcm.sample_rate {
        pcm.samples.clone()
    } else {
        super::resample_linear(&pcm.samples, pcm.channels, pcm.sample_rate, target_rate)
    };

    let mut encoder = Builder::new()
        .ok_or_else(|| {
            ConvertError::Encode(name(), "could not allocate the LAME encoder".to_string())
        })?
        .with_num_channels(pcm.channels as u8)
        .map_err(|e| ConvertError::Encode(name(), format!("channel count: {e:?}")))?
        .with_sample_rate(target_rate)
        .map_err(|e| ConvertError::Encode(name(), format!("sample rate: {e:?}")))?
        .with_brate(nearest_bitrate_preset(bitrate_kbps))
        .map_err(|e| ConvertError::Encode(name(), format!("bitrate: {e:?}")))?
        .with_quality(mp3lame_encoder::Quality::Best)
        .map_err(|e| ConvertError::Encode(name(), format!("quality: {e:?}")))?
        .build()
        .map_err(|e| ConvertError::Encode(name(), format!("{e:?}")))?;

    // `InterleavedPcm<f32>` is fed our normalized `[-1.0, 1.0]` samples
    // as-is, on the assumption that LAME's `ieee_float` input path expects
    // that range (matching every other float-PCM API this app already
    // touches) rather than a 16-bit-full-scale float — unverified without
    // compiling and actually listening to the output, since the crate's own
    // docs don't state the convention explicitly. Worth a listening check
    // before shipping: a scale mismatch here would produce either near-silent
    // or heavily clipped MP3s, not a build failure.
    let frames = resampled.len() / pcm.channels.max(1);
    let mut out = Vec::with_capacity(mp3lame_encoder::max_required_buffer_size(frames));

    // SAFETY: `encode`/`flush` only ever write into the spare capacity they
    // are given and report back exactly how many bytes they initialized;
    // `set_len` below only ever grows the vec by that reported count, never
    // past what was just written — the same contract the crate's own
    // documented example relies on.
    let written = encoder
        .encode(InterleavedPcm(&resampled), spare_capacity(&mut out))
        .map_err(|e| ConvertError::Encode(name(), format!("{e:?}")))?;
    unsafe {
        out.set_len(out.len() + written);
    }

    let flushed = encoder
        .flush::<FlushNoGap>(spare_capacity(&mut out))
        .map_err(|e| ConvertError::Encode(name(), format!("flush: {e:?}")))?;
    unsafe {
        out.set_len(out.len() + flushed);
    }

    std::fs::write(dest, &out).map_err(|e| ConvertError::Io(name(), e.to_string()))
}

/// `Vec::spare_capacity_mut`, reserving room for at least one more flush call
/// first — `encode`/`flush` both refuse to write past the slice they are
/// given rather than growing it themselves.
fn spare_capacity(buf: &mut Vec<u8>) -> &mut [MaybeUninit<u8>] {
    if buf.spare_capacity_mut().len() < 7200 {
        // LAME's own documented minimum for a final flush call.
        buf.reserve(7200);
    }
    buf.spare_capacity_mut()
}

/// The closest rate LAME actually supports. Prefers an integer-division
/// match against the common hi-res multiples this app already detects
/// (88.2k/176.4k -> 44.1k, 96k/192k -> 48k) before falling back to nearest by
/// raw distance for anything else.
fn nearest_supported_rate(rate: u32) -> u32 {
    let mut r = rate;
    while r > 48_000 && r.is_multiple_of(2) {
        r /= 2;
    }
    if LAME_SUPPORTED_RATES.contains(&r) {
        return r;
    }
    LAME_SUPPORTED_RATES
        .iter()
        .min_by_key(|&&s| (s as i64 - rate as i64).abs())
        .copied()
        .unwrap_or(44_100)
}

/// The [`Bitrate`] preset closest to `kbps` — LAME only accepts one of 16
/// fixed steps, not an arbitrary integer.
fn nearest_bitrate_preset(kbps: u32) -> Bitrate {
    const PRESETS: &[(u32, Bitrate)] = &[
        (8, Bitrate::Kbps8),
        (16, Bitrate::Kbps16),
        (24, Bitrate::Kbps24),
        (32, Bitrate::Kbps32),
        (40, Bitrate::Kbps40),
        (48, Bitrate::Kbps48),
        (64, Bitrate::Kbps64),
        (80, Bitrate::Kbps80),
        (96, Bitrate::Kbps96),
        (112, Bitrate::Kbps112),
        (128, Bitrate::Kbps128),
        (160, Bitrate::Kbps160),
        (192, Bitrate::Kbps192),
        (224, Bitrate::Kbps224),
        (256, Bitrate::Kbps256),
        (320, Bitrate::Kbps320),
    ];
    PRESETS
        .iter()
        .min_by_key(|&&(k, _)| (k as i64 - kbps as i64).abs())
        .map(|&(_, b)| b)
        .unwrap_or(Bitrate::Kbps256)
}

#[cfg(test)]
#[path = "../../tests/unit/convert/mp3.rs"]
mod tests;
