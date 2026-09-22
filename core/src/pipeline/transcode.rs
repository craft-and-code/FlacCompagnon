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
mod tests {
    use super::*;

    #[test]
    fn a_missing_codec_cannot_be_cleared_by_the_other_codecs_negative() {
        let negative = || TranscodeEvidence {
            likelihood: 0.01,
            offset: 0,
            detected: false,
        };
        assert!(matches!(
            combine(None, Some(negative()), 0.0125, 0.031),
            Err(LatticeSkip::TooShort)
        ));
        assert!(matches!(
            combine(Some(negative()), None, 0.0125, 0.031),
            Err(LatticeSkip::TooShort)
        ));
        assert!(
            !combine(Some(negative()), Some(negative()), 0.0125, 0.031)
                .unwrap()
                .detected
        );
    }

    #[test]
    fn a_positive_remains_useful_when_the_other_codec_cannot_run() {
        let positive = || TranscodeEvidence {
            likelihood: 0.1,
            offset: 7,
            detected: true,
        };
        assert!(
            combine(None, Some(positive()), 0.0125, 0.031)
                .unwrap()
                .detected
        );
        assert!(
            combine(Some(positive()), None, 0.0125, 0.031)
                .unwrap()
                .detected
        );
    }

    #[test]
    fn deinterleave_splits_channels_in_order() {
        // L R L R L R
        let inter = [1.0f32, -1.0, 2.0, -2.0, 3.0, -3.0];
        let got = deinterleave(&inter, 2).expect("two channels");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(got[1], vec![-1.0, -2.0, -3.0]);
    }

    #[test]
    fn mono_stays_one_channel() {
        let got = deinterleave(&[1.0f32, 2.0, 3.0], 1).expect("mono");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0], vec![1.0, 2.0, 3.0]);
    }

    /// Surround is truncated to the front pair, and — the part worth
    /// asserting — the front pair must still be the *right* samples, not
    /// whatever the first two slots of a 6-channel frame happen to hold after
    /// a stride mistake.
    #[test]
    fn surround_keeps_the_front_pair_correctly_strided() {
        let mut inter = Vec::new();
        for f in 0..4 {
            for c in 0..6 {
                inter.push((f * 10 + c) as f32);
            }
        }
        let got = deinterleave(&inter, 6).expect("surround");
        assert_eq!(got.len(), MAX_CHANNELS);
        assert_eq!(got[0], vec![0.0, 10.0, 20.0, 30.0]);
        assert_eq!(got[1], vec![1.0, 11.0, 21.0, 31.0]);
    }

    /// A decoder returning a partial final frame must not produce channels of
    /// unequal length: the detector zips them against each other.
    #[test]
    fn a_truncated_final_frame_is_dropped_not_padded() {
        // Five samples across two channels: two whole frames plus a stray.
        let got = deinterleave(&[1.0f32, 2.0, 3.0, 4.0, 5.0], 2).expect("ragged");
        assert_eq!(got[0].len(), got[1].len(), "channels must match in length");
        assert_eq!(got[0], vec![1.0, 3.0]);
        assert_eq!(got[1], vec![2.0, 4.0]);
    }

    #[test]
    fn degenerate_inputs_are_refused_without_panicking() {
        assert!(deinterleave(&[], 2).is_none(), "empty");
        assert!(deinterleave(&[1.0, 2.0], 0).is_none(), "zero channels");
        // Fewer samples than one frame: nothing to analyze.
        assert!(deinterleave(&[1.0], 2).is_none(), "partial single frame");
    }

    /// The reason matters as much as the refusal: these two used to be the
    /// same `None` as "clean-but-short", and that ambiguity is what made a
    /// dash in the Lattice column impossible to act on.
    #[test]
    fn an_undecodable_file_says_so_rather_than_giving_a_verdict() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("not-audio.flac");
        std::fs::write(&path, b"this is not a FLAC file").expect("write");
        assert!(matches!(
            detect(&path, &AacParams::default(), &Mp3Params::default(), &|| {
                false
            }),
            Err(LatticeSkip::Undecodable(_))
        ));
        // A missing file, too.
        assert!(matches!(
            detect(
                dir.path().join("absent.wav").as_path(),
                &AacParams::default(),
                &Mp3Params::default(),
                &|| false
            ),
            Err(LatticeSkip::Undecodable(_))
        ));
    }

    #[test]
    fn cancellation_is_checked_before_the_decode() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("whatever.flac");
        assert_eq!(
            detect(&path, &AacParams::default(), &Mp3Params::default(), &|| {
                true
            }),
            Err(LatticeSkip::Cancelled)
        );
    }

    /// Every reason must read as a sentence a user can act on — an empty or
    /// duplicated one would leave the table saying nothing again.
    #[test]
    fn every_skip_reason_is_distinct_and_non_empty() {
        let all = [
            LatticeSkip::Undecodable("boom".into()),
            LatticeSkip::NoSamples,
            LatticeSkip::UnsupportedRate(96_000),
            LatticeSkip::TooShort,
            LatticeSkip::Cancelled,
        ];
        let texts: Vec<String> = all.iter().map(|s| s.to_string()).collect();
        for t in &texts {
            assert!(!t.is_empty());
        }
        for (i, a) in texts.iter().enumerate() {
            for b in texts.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
        }
        assert!(texts[0].contains("boom"), "{}", texts[0]);
        assert!(texts[2].contains("96.0 kHz"), "{}", texts[2]);
    }
}
