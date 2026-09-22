//! Authenticity detections: three independent tests, each a plain yes/no.
//!
//! * **Upscaling**   — integer samples whose low bits are always zero. This
//!   proves unused precision, not the recording history.
//! * **Upsampling**  — a low-rate signal placed in a higher-sample-rate
//!   container, inferred from limited bandwidth. This is a heuristic, not
//!   proof that the sample rate was changed.
//! * **Transcoding** — a lossy source re-wrapped as lossless. Decided by
//!   [`crate::transcode`], which looks for the codec's own quantization
//!   lattice; this module only reports the answer.
//!
//! Each test is independent: a file may trip none, one, or several. The
//! reasoning is always surfaced in `detail`.
//!
//! ## Why there is no "suspected" any more
//!
//! Transcoding used to have three states, the middle one raised by a gentle
//! spectral roll-off with no sharp cliff. It was the app's main source of
//! false positives, and unavoidably so: acoustic, classical and analog-tape
//! recordings are *naturally* dark, and no threshold separates "quiet above
//! 16 kHz because a codec removed it" from "quiet above 16 kHz because nobody
//! played anything up there". A verdict that fires on a whole genre is not a
//! verdict.
//!
//! It is gone, along with the brick-wall and MDCT dead-zone rules that fed
//! it. Transcoding is now decided solely by the re-quantization detectors,
//! which provide statistical evidence of quantization, not a proof of origin.
//! An incomplete search must remain unknown rather than clearing the file.
//! The cut-off frequency is still measured and still shown in its own
//! column — it is useful information — but it no longer accuses anyone.

use serde::{Deserialize, Serialize};

use super::analyzer::AnalysisSummary;
use crate::transcode::LatticeResult;

/// The three independent detections plus a human summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detections {
    /// `true` when the content uses fewer bits than the container declares.
    pub upscaling: bool,
    /// `true` when a spectral heuristic suggests upsampling, not proof of origin.
    pub upsampling: bool,
    /// `true` when a lossy codec's quantization lattice was found in the
    /// audio. Decided by [`crate::transcode`], not measured here.
    pub transcoding: bool,
    /// One-line explanation of the flagged issues (or why the file looks clean).
    pub detail: String,
    /// Quick status word: "Clean", "Flagged", or "Unknown" when the
    /// checks could not complete or the content is silent, with no issue found.
    pub summary: String,
}

impl Detections {
    /// A neutral placeholder for a file that hasn't been (or couldn't be)
    /// analyzed yet — no detections flagged, summary set to Unknown.
    pub fn unknown() -> Self {
        Detections {
            upscaling: false,
            upsampling: false,
            transcoding: false,
            detail: "Not yet analyzed.".to_string(),
            summary: "Unknown".to_string(),
        }
    }
}

// --- Tunable thresholds -----------------------------------------------------
/// Rates above this are "hi-res" containers subject to the upsampling check.
const HIRES_RATE: u32 = 48_000;
/// In a hi-res (> 48 kHz) container, content confined below this frequency
/// (CD/DVD range, with margin) triggers the legacy bandwidth heuristic.
/// This 30 kHz margin is not a validated boundary between native and resampled audio.
const UPSAMPLE_MAX_HZ: f64 = 30_000.0;
/// MDCT: fraction of frames that must show a dead zone. Calibrated on real
/// transcodes: genuine music tops out around 0.30 (dark 1990s metal masters),
/// while 128–192 kbps AAC sits at ~1.0, so 0.70 gives a safe margin both ways.
const MDCT_DEAD_FRACTION: f32 = 0.70;
/// MDCT: mean cut-off must be below this fraction of N to matter.
const MDCT_CUTOFF_RATIO: f64 = 0.90;
/// MDCT: the dead zone must be at least this flat/deep (dB rel. frame peak).
const MDCT_DEAD_DB: f32 = -75.0;

/// Whether the MDCT shows a consistent high-frequency dead zone.
///
/// Limited bandwidth can also come from native low-pass-filtered material.
/// Even a spectrogram cannot distinguish identical samples with different
/// histories; this measurement must never be described as proof of origin.
fn mdct_signature(s: &AnalysisSummary) -> bool {
    match (s.mdct_dead_fraction, s.mdct_cutoff_ratio, s.mdct_dead_db) {
        (Some(frac), Some(cutoff), Some(dead)) => {
            frac >= MDCT_DEAD_FRACTION && cutoff < MDCT_CUTOFF_RATIO && dead < MDCT_DEAD_DB
        }
        _ => false,
    }
}

/// Run the three detections.
///
/// `transcode` is decided by the caller rather than measured here: it comes
/// from [`crate::transcode`], which needs the whole decoded signal and a
/// sweep of its own, neither of which belongs in a module whose job is to
/// turn finished measurements into a verdict.
///
/// It arrives as the full [`crate::transcode::TranscodeEvidence`] rather than
/// a bare `bool` so that the *score* reaches `detail`. That distinction
/// matters in both directions: a user told only "Clean" cannot tell a file
/// that scored 0.008 from one the detector never ran on, and a user told only
/// "Transcoded" cannot tell a borderline finding from an overwhelming one.
///
/// [`crate::transcode::LatticeSkip`] is the third case — not tested — and it
/// carries *why*, which ends up in `detail` verbatim. A dash in the Lattice
/// column with no explanation is a dead end for the user and for whoever
/// receives their bug report.
pub fn classify(
    summary: &AnalysisSummary,
    sample_rate: u32,
    declared_bits: Option<u32>,
    real_bit_depth: Option<u32>,
    transcode: LatticeResult,
) -> Detections {
    // Zero samples carry no evidence of their recording resolution. The
    // former one-bit display was just the minimum of the bit-count formula,
    // not a measurement that could justify either Upscaled or Clean.
    if real_bit_depth.is_none() && summary.clipping.peak == 0.0 {
        return Detections {
            detail: "Digital silence: all decoded samples are zero. Effective bit depth and source authenticity cannot be determined.".into(),
            ..Detections::unknown()
        };
    }
    let mdct_dead = mdct_signature(summary);
    let mdct_cutoff_hz = summary
        .mdct_cutoff_ratio
        .map(|r| r * (sample_rate as f64 / 2.0))
        .unwrap_or(sample_rate as f64 / 2.0);
    let mdct_khz = mdct_cutoff_hz / 1000.0;
    let stft_khz = summary.cutoff_hz / 1000.0;

    // 1. Upscaling — lower-resolution content in a wider container. Exact
    //    zero padding and a tightly dithered 16-bit grid are both measured.
    let upscaling = matches!(
        (declared_bits, real_bit_depth),
        (Some(d), Some(r)) if (1..=32).contains(&d) && r > 0 && r < d
    );

    // 2. Upsampling candidate. In a hi-res container (> 48 kHz), a
    //    consistent MDCT dead zone confined to the CD/DVD range suggests the extra
    //    bandwidth is empty. (The STFT cut-off is unreliable on real music — it
    //    usually reads full-band — so the MDCT dead zone is used here.)
    let upsampling = sample_rate > HIRES_RATE && mdct_dead && mdct_cutoff_hz <= UPSAMPLE_MAX_HZ;

    // 3. Transcoding — lossy source. Handed in by the caller; the only
    //    decision left here is that it does not apply above 48 kHz, where the
    //    lattice detectors have no tabulated bands.
    let applies = sample_rate <= HIRES_RATE;
    let transcoding = transcode.as_ref().is_ok_and(|e| e.detected) && applies;

    // Build the human explanation.
    let mut reasons: Vec<String> = Vec::new();
    if upscaling {
        reasons.push(if summary.bit_depth_dithered {
            format!(
                "Upscaling: {}-bit samples follow a 16-bit quantization grid with low-level dither",
                declared_bits.unwrap_or(0)
            )
        } else {
            format!(
                "Upscaling: samples fit exactly in {} bits within a {}-bit container (zero low bits)",
                real_bit_depth.unwrap_or(0),
                declared_bits.unwrap_or(0)
            )
        });
    }
    if upsampling {
        reasons.push(format!(
            "Possible upsampling: content stops at ~{:.1} kHz in a {:.1} kHz container; limited bandwidth is not proof of resampling",
            mdct_khz,
            sample_rate as f64 / 1000.0
        ));
    }
    if transcoding {
        reasons.push(format!(
            "Transcoding: statistical evidence of a codec quantization lattice (score {:.4}); this score is not a probability or proof of origin",
            transcode.as_ref().map(|e| e.likelihood).unwrap_or(0.0)
        ));
    }

    let flagged = upscaling || upsampling || transcoding;
    let summary_word = if flagged {
        "Flagged"
    } else if transcode.is_err() || real_bit_depth.is_none() {
        "Unknown"
    } else {
        "Clean"
    };

    // Always say what the lattice search found, flagged or not. A bare
    // "Clean" hides the difference between "measured, and nothing there" and
    // "never measured", and those deserve different amounts of trust.
    let lattice = match (&transcode, applies) {
        _ if transcoding => String::new(),
        (Ok(e), true) => format!(
            " No significant codec lattice evidence (score {:.4}).",
            e.likelihood
        ),
        (_, false) => format!(
            " Lattice search not applicable at {:.1} kHz; a lossy source cannot be ruled out.",
            sample_rate as f64 / 1000.0
        ),
        // The *reason*, not just the fact. A row whose Lattice column is a
        // dash is otherwise indistinguishable from one the search cleared,
        // and the user has no way to tell which — nor to report it usefully.
        (Err(skip), true) => format!(" Lattice search did not complete: {skip}."),
    };

    let mut detail = if reasons.is_empty() {
        format!("{summary_word} — measured bandwidth to ~{stft_khz:.1} kHz.{lattice}")
    } else {
        format!("{}{lattice}", reasons.join(" · "))
    };

    if real_bit_depth.is_none() {
        detail.push_str(" Integer bit-depth check unavailable; source resolution is not verified.");
    } else if !upscaling {
        detail.push_str(" No zero-padding found; dither or processing can hide a lower-bit-depth source. Used bits do not establish the original resolution.");
    }

    Detections {
        upscaling,
        upsampling,
        transcoding,
        detail,
        summary: summary_word.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcode::{LatticeSkip, TranscodeEvidence};
    use crate::ClippingInfo;

    /// A detector result to hand to `classify`. An `Err` means "not tested",
    /// which is a third case and not a synonym for clean.
    fn evidence(detected: bool) -> LatticeResult {
        Ok(TranscodeEvidence {
            likelihood: if detected { 0.184 } else { 0.008 },
            offset: 289,
            detected,
        })
    }

    fn summ(
        cutoff_hz: f64,
        sample_rate: u32,
        cliff_db: f32,
        above_db: f32,
        mdct: Option<(f32, f64, f32)>,
    ) -> AnalysisSummary {
        let nyq = sample_rate as f64 / 2.0;
        let (frac, mcr, mdb) = match mdct {
            Some((f, c, d)) => (Some(f), Some(c), Some(d)),
            None => (None, None, None),
        };
        AnalysisSummary {
            cutoff_hz,
            cutoff_ratio: cutoff_hz / nyq,
            cliff_db,
            above_db,
            spectrum_db: vec![],
            clipping: ClippingInfo {
                clipped_samples: 0,
                clip_events: 0,
                peak: 0.5,
                peak_dbfs: -6.0,
                true_peak: 0.5,
                true_peak_dbtp: -6.0,
                clipped: false,
            },
            fake_stereo: false,
            real_bit_depth: None,
            bit_depth_dithered: false,
            dr_db: None,
            mdct_cutoff_ratio: mcr,
            mdct_dead_db: mdb,
            mdct_dead_fraction: frac,
            requant_rate: None,
        }
    }

    /// A file with nothing wrong with it. The cut-off arguments are varied
    /// across these tests precisely to prove they no longer influence the
    /// transcoding verdict — before, an early roll-off here would have
    /// produced "Suspicious".
    #[test]
    fn a_clean_file_is_clean_whatever_its_cutoff() {
        for cutoff in [21_000.0, 19_000.0, 16_000.0, 14_000.0] {
            let d = classify(
                &summ(cutoff, 44_100, 30.0, -95.0, None),
                44_100,
                Some(16),
                Some(16),
                evidence(false),
            );
            assert!(!d.transcoding, "cutoff {cutoff} should not accuse anyone");
            assert!(!d.upscaling && !d.upsampling);
            assert_eq!(d.summary, "Clean");
        }
    }

    /// The regression this rewrite exists for. A naturally dark master — a
    /// steep roll-off at 15 kHz into near-silence — used to be reported as
    /// transcoded or suspected on the strength of its spectrum alone. It is
    /// exactly what an acoustic or analog-tape recording looks like, and the
    /// lattice detector says nothing about it, so neither does the verdict.
    #[test]
    fn a_naturally_dark_master_is_no_longer_accused() {
        let dark = summ(15_000.0, 44_100, 35.0, -100.0, Some((1.0, 0.6, -90.0)));
        let d = classify(&dark, 44_100, Some(16), Some(16), evidence(false));
        assert!(!d.transcoding, "{}", d.detail);
        assert_eq!(d.summary, "Clean");
    }

    #[test]
    fn transcoding_is_reported_when_the_detector_says_so() {
        let d = classify(
            &summ(20_000.0, 44_100, 5.0, -60.0, None),
            44_100,
            Some(16),
            Some(16),
            evidence(true),
        );
        assert!(d.transcoding);
        assert_eq!(d.summary, "Flagged");
        assert!(d.detail.contains("lattice"), "{}", d.detail);
    }

    /// Above 48 kHz the detectors have no
    /// tabulated scalefactor bands anyway — so a transcoding verdict there is
    /// suppressed even if one is handed in.
    #[test]
    fn transcoding_does_not_apply_to_hi_res_rates() {
        let d = classify(
            &summ(40_000.0, 96_000, 5.0, -60.0, None),
            96_000,
            Some(24),
            Some(24),
            evidence(true),
        );
        assert!(!d.transcoding, "{}", d.detail);
    }

    #[test]
    fn upscaling_is_detected_from_the_bit_depths() {
        let d = classify(
            &summ(21_000.0, 44_100, 5.0, -60.0, None),
            44_100,
            Some(24),
            Some(16),
            evidence(false),
        );
        assert!(d.upscaling);
        assert_eq!(d.summary, "Flagged");
        assert!(d.detail.contains("24-bit"), "{}", d.detail);
        // Equal depths are not upscaling.
        let ok = classify(
            &summ(21_000.0, 44_100, 5.0, -60.0, None),
            44_100,
            Some(24),
            Some(24),
            evidence(false),
        );
        assert!(!ok.upscaling);
    }

    /// The legacy bandwidth flag is retained, but its explanation must
    /// distinguish a spectral observation from a proven resampling history.
    #[test]
    fn upsampling_still_uses_the_mdct_dead_zone() {
        let up = summ(22_000.0, 96_000, 30.0, -95.0, Some((1.0, 0.45, -90.0)));
        let d = classify(&up, 96_000, Some(24), Some(24), evidence(false));
        assert!(d.upsampling, "{}", d.detail);
        assert!(d.detail.contains("not proof of resampling"));
        assert_eq!(d.summary, "Flagged");

        // The same dead zone at CD rate is not upsampling: there is no extra
        // bandwidth to be empty.
        let cd = classify(
            &summ(15_000.0, 44_100, 30.0, -95.0, Some((1.0, 0.45, -90.0))),
            44_100,
            Some(16),
            Some(16),
            evidence(false),
        );
        assert!(!cd.upsampling);
    }

    #[test]
    fn several_detections_can_fire_at_once() {
        let up = summ(22_000.0, 96_000, 30.0, -95.0, Some((1.0, 0.45, -90.0)));
        let d = classify(&up, 96_000, Some(24), Some(16), evidence(false));
        assert!(d.upscaling && d.upsampling);
        assert!(
            d.detail.contains(" \u{b7} "),
            "both reasons should be listed: {}",
            d.detail
        );
    }

    /// An `Err` is a third answer, not a synonym for clean, and `detail` has to
    /// say which one it is. A user reading "Clean" on a file the detector
    /// never ran on is being told something the app does not know.
    #[test]
    fn an_untested_file_says_so_rather_than_reading_as_clean() {
        // Too short / undecodable: the detector returned no answer at a rate
        // it *does* support.
        let d = classify(
            &summ(21_000.0, 44_100, 5.0, -60.0, None),
            44_100,
            Some(16),
            Some(16),
            Err(LatticeSkip::TooShort),
        );
        assert!(!d.transcoding);
        assert_eq!(d.summary, "Unknown");
        assert!(d.detail.contains("did not complete"), "{}", d.detail);
        // And it names the cause, so the row is reportable as-is.
        assert!(d.detail.contains("too short"), "{}", d.detail);

        // A decode failure is a different cause and must read differently.
        let bad = classify(
            &summ(21_000.0, 44_100, 5.0, -60.0, None),
            44_100,
            Some(16),
            Some(16),
            Err(LatticeSkip::Undecodable(
                "no decoder: unsupported codec".into(),
            )),
        );
        assert!(bad.detail.contains("no decoder"), "{}", bad.detail);

        // Above 48 kHz the test does not apply at all, and says that instead.
        let hi = classify(
            &summ(40_000.0, 96_000, 5.0, -60.0, None),
            96_000,
            Some(24),
            Some(24),
            Err(LatticeSkip::UnsupportedRate(96_000)),
        );
        assert!(hi.detail.contains("not applicable"), "{}", hi.detail);

        // And when it did run and found nothing, the score is shown.
        let ok = classify(
            &summ(21_000.0, 44_100, 5.0, -60.0, None),
            44_100,
            Some(16),
            Some(16),
            evidence(false),
        );
        assert!(ok.detail.contains("0.0080"), "{}", ok.detail);
    }

    #[test]
    fn digital_silence_is_unknown_even_when_the_lattice_test_is_negative() {
        let mut silent = summ(0.0, 44_100, 0.0, -120.0, None);
        silent.clipping.peak = 0.0;
        for declared in [16, 24, 32] {
            let d = classify(&silent, 44_100, Some(declared), None, evidence(false));
            assert_eq!(d.summary, "Unknown");
            assert!(!d.upscaling && !d.upsampling && !d.transcoding);
            assert!(d.detail.contains("Digital silence"));
        }
    }

    #[test]
    fn missing_bit_measurement_is_not_a_clean_resolution_verdict() {
        let d = classify(
            &summ(21_000.0, 44_100, 5.0, -60.0, None),
            44_100,
            Some(32),
            None,
            evidence(false),
        );
        assert_eq!(d.summary, "Unknown");
        assert!(d.detail.contains("Integer bit-depth check unavailable"));
    }

    #[test]
    fn occupied_low_bits_do_not_certify_the_original_resolution() {
        let d = classify(
            &summ(21_000.0, 44_100, 5.0, -60.0, None),
            44_100,
            Some(24),
            Some(24),
            evidence(false),
        );
        assert!(!d.upscaling);
        assert!(d.detail.contains("dither or processing"));
    }

    #[test]
    fn the_placeholder_flags_nothing() {
        let d = Detections::unknown();
        assert!(!d.upscaling && !d.upsampling && !d.transcoding);
        assert_eq!(d.summary, "Unknown");
    }
}
