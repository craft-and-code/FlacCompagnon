//! The bridge between a file on disk and the lattice detectors.
//!
//! [`crate::transcode`] is deliberately pure: it takes samples and returns a
//! verdict, touches no filesystem, and can therefore be tested on synthetic
//! signals with no fixtures. Getting the samples to it is this file's job, and
//! it lives in the pipeline because that is where decoding and orchestration
//! already belong.
//!
//! # Why this decodes a second time
//!
//! The main analysis pass streams: it consumes packets, folds each into
//! running statistics, and drops them. That is what lets a 200-track folder be
//! analyzed in parallel without filling memory.
//!
//! The detectors cannot work that way. They re-measure the same frames at
//! 1024 different sample alignments, so they need the whole signal resident,
//! in `f64`, for the duration. A five-minute stereo track at 44.1 kHz is about
//! 210 MB in that form — acceptable for one file at a time, ruinous for a
//! dozen decoded in parallel.
//!
//! The file is decoded again here and the buffer is released after its
//! sweeps. Concurrent callers each hold their own buffer; this function
//! does not serialize them or impose a global memory ceiling.

use std::path::Path;

use crate::decode::decode_to_pcm;
use crate::transcode::{
    aac, mp3, supported_rate_khz, AacParams, LatticeResult, LatticeSkip, Mp3Params,
    TranscodeEvidence,
};

/// How many channels the detectors look at.
///
/// Two: a stereo master's content is in the first pair, and the detectors
/// already test that pair four ways (L, R, M, S). Surround channels would
/// multiply the cost of the most expensive step in the app for content that
/// carries the same lattice as the front pair.
const MAX_CHANNELS: usize = 2;

/// Decode `path` and ask **every** codec detector whether the audio sits on
/// its lattice, returning the strongest evidence found.
///
/// Both detectors run, and both must: they look in different transforms, so a
/// file the AAC sweep calls clean can still be an MP3 transcode, and vice
/// versa. Measured on a real 128 kbps MP3 re-wrapped as FLAC, the MP3 sweep
/// scored 0.184 against its threshold while the AAC sweep scored 0.011 against
/// its own 0.0125 — the AAC detector would have declared that file genuine on
/// its own.
///
/// Doubling the sweep doubles the cost, which is the most expensive step in
/// the app. It is not optional all the same: half a detector is a detector
/// that reports "clean" for the wrong reason.
///
/// `Err` when the file could not be decoded, its sample rate has no
/// tabulated bands, it is too short, or `cancelled` fired. An `Err` is not
/// "clean" — it is "no answer" — and it says which of those happened, because
/// a bare "no answer" is exactly as unactionable in a bug report as it is in
/// the results table.
pub fn detect(
    path: &Path,
    aac_params: &AacParams,
    mp3_params: &Mp3Params,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> LatticeResult {
    if cancelled() {
        return Err(LatticeSkip::Cancelled);
    }
    // Check the decoded rate before either sweep so the reason is
    // the *specific* one: the detectors would refuse an untabulated rate too,
    // but by then it is indistinguishable from every other refusal.
    let pcm = decode_to_pcm(path).map_err(|e| LatticeSkip::Undecodable(e.to_string()))?;
    if supported_rate_khz(pcm.sample_rate).is_none() {
        return Err(LatticeSkip::UnsupportedRate(pcm.sample_rate));
    }
    let channels = deinterleave(&pcm.samples, pcm.channels).ok_or(LatticeSkip::NoSamples)?;
    // Dropped before the sweep starts: the interleaved `f32` copy is dead
    // weight from here on, and this is the one place in the app where holding
    // an extra copy of a whole track actually matters.
    drop(pcm.samples);

    let found_aac = aac::detect(&channels, pcm.sample_rate, aac_params, cancelled);
    if cancelled() {
        return Err(LatticeSkip::Cancelled);
    }
    let found_mp3 = mp3::detect(&channels, pcm.sample_rate, mp3_params, cancelled);
    if cancelled() {
        return Err(LatticeSkip::Cancelled);
    }
    combine(
        found_aac,
        found_mp3,
        aac_params.significance,
        mp3_params.significance,
    )
}

fn combine(
    found_aac: Option<TranscodeEvidence>,
    found_mp3: Option<TranscodeEvidence>,
    aac_threshold: f64,
    mp3_threshold: f64,
) -> LatticeResult {
    // A positive from either codec wins outright — the likelihoods are *not*
    // comparable across detectors, since each is a fraction measured against
    // its own threshold, and the two thresholds differ. Comparing the raw
    // numbers would let a near-miss on one codec outrank a clear hit on the
    // other.
    match (found_aac, found_mp3) {
        (Some(a), Some(m)) => Ok(match (a.detected, m.detected) {
            (true, false) => a,
            (false, true) => m,
            // Both agree: report whichever is further past its own threshold.
            _ => {
                let margin = |e: &TranscodeEvidence, lambda: f64| e.likelihood / lambda;
                if margin(&m, mp3_threshold) > margin(&a, aac_threshold) {
                    m
                } else {
                    a
                }
            }
        }),
        // One positive is useful evidence, but one negative cannot clear a
        // codec that lacked enough frames to run its calibrated search.
        (Some(e), None) | (None, Some(e)) if e.detected => Ok(e),
        _ => Err(LatticeSkip::TooShort),
    }
}

/// Split interleaved samples into one `f64` buffer per channel, keeping at
/// most [`MAX_CHANNELS`].
///
/// `None` for a zero-channel or empty decode — both mean there is nothing to
/// analyze, and neither should reach the detector as an empty channel list it
/// would have to guess about.
fn deinterleave(samples: &[f32], channels: usize) -> Option<Vec<Vec<f64>>> {
    if channels == 0 || samples.is_empty() {
        return None;
    }
    let kept = channels.min(MAX_CHANNELS);
    // Integer division: a truncated final frame (a decoder returning half a
    // frame at EOF) is dropped rather than padded with silence, which would
    // put a discontinuity into the very last analysis window.
    let frames = samples.len() / channels;
    if frames == 0 {
        return None;
    }

    let mut out = vec![Vec::with_capacity(frames); kept];
    for f in 0..frames {
        for (c, buf) in out.iter_mut().enumerate() {
            // `get` rather than indexing: `samples` comes from a decoded file,
            // and this crate does not panic on anything a file produced.
            if let Some(s) = samples.get(f * channels + c) {
                buf.push(*s as f64);
            }
        }
    }
    Some(out)
}

#[cfg(test)]
#[path = "../../tests/unit/pipeline/transcode.rs"]
mod tests;
