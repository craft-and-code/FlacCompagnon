//! Authenticity detections: three independent tests, each a plain yes/no.
//!
//! * **Upscaling**   — a low-resolution signal (≤16-bit) stored at a higher bit
//!   depth. Detected from the effective vs. declared bit depth.
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
//! Missing checks are explained in detail; Clean means no finding, not that
//! every check was available or that the recording history was verified.
//! The cut-off frequency is still measured and still shown in its own
//! column — it is useful information — but it no longer accuses anyone.

use serde::{Deserialize, Serialize};

use super::analyzer::AnalysisSummary;
use super::bitdepth::BitDepthMethod;
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
    /// "Clean" for no findings, "Flagged" for any finding, or "Not analyzed"
    /// for a placeholder. Missing checks remain explicit in `detail`.
    pub summary: String,
}

impl Detections {
    /// Build a binary summary from findings, independently of check availability.
    pub fn from_findings(
        upscaling: bool,
        upsampling: bool,
        transcoding: bool,
        detail: String,
    ) -> Self {
        Self {
            upscaling,
            upsampling,
            transcoding,
            detail,
            summary: if upscaling || upsampling || transcoding {
                "Flagged"
            } else {
                "Clean"
            }
            .into(),
        }
    }

    /// A neutral placeholder for a file that hasn't been (or couldn't be)
    /// analyzed yet. A read failure is displayed separately as a file error.
    pub fn not_analyzed() -> Self {
        Detections {
            upscaling: false,
            upsampling: false,
            transcoding: false,
            detail: "Not yet analyzed.".to_string(),
            summary: "Not analyzed".to_string(),
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
    let mdct_dead = mdct_signature(summary);
    let mdct_cutoff_hz = summary
        .mdct_cutoff_ratio
        .map(|r| r * (sample_rate as f64 / 2.0))
        .unwrap_or(sample_rate as f64 / 2.0);
    let mdct_khz = mdct_cutoff_hz / 1000.0;
    let stft_khz = summary.cutoff_hz / 1000.0;

    // 1. Upscaling: exact unused bits, or a persistent lower-depth grid with
    //    a bounded residual. The latter is identified as an estimate in detail.
    let upscaling = matches!(
        (declared_bits, real_bit_depth),
        (Some(d), Some(r)) if r < d
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
        let real = real_bit_depth.unwrap_or(0);
        let declared = declared_bits.unwrap_or(0);
        reasons.push(match summary.bit_depth_evidence {
            Some(e) if e.method == BitDepthMethod::NarrowGrid => format!(
                "Upscaling: estimated {real}-bit depth in {declared}-bit PCM from per-channel quantization grids with low-level residuals; stored samples occupy {} bits", e.stored_bits
            ),
            _ if summary.clipping.peak == 0.0 => format!(
                "Upscaling: digital silence in {declared}-bit PCM (all samples are zero; reported as 1 bit by convention)"
            ),
            _ => format!("Upscaling: samples fit exactly in {real} bits within {declared}-bit PCM (zero low bits)"),
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

    let mut detections =
        Detections::from_findings(upscaling, upsampling, transcoding, String::new());

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

    detections.detail = if reasons.is_empty() {
        format!("{} — no anomaly detected by the completed checks; measured bandwidth to ~{stft_khz:.1} kHz.{lattice}", detections.summary)
    } else {
        format!("{}{lattice}", reasons.join(" · "))
    };

    detections
}

#[cfg(test)]
#[path = "../../tests/unit/analysis/detections.rs"]
mod tests;
