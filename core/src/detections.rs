//! Authenticity detections, mirroring the three independent tests of the
//! original *Lossless Audio Checker* (Lacroix & Prime, AES):
//!
//! * **Upscaling**   — a low-resolution signal (≤16-bit) stored at a higher bit
//!   depth. Detected from the effective vs. declared bit depth.
//! * **Upsampling**  — a low-rate signal placed in a higher-sample-rate
//!   container; its content is band-limited well below the claimed Nyquist.
//! * **Transcoding** — a lossy (MP3/AAC) source re-wrapped as lossless. Detected
//!   from a brick-wall spectral cut-off and, for AAC specifically, from a flat
//!   high-frequency "dead zone" in the sine-window MDCT domain (the *AAC-SIN*
//!   signature).
//!
//! Each test is independent: a file may trip none, one, or several. As in the
//! original, these are heuristics — the reasoning is always surfaced in `detail`.

use serde::{Deserialize, Serialize};

use crate::analyzer::AnalysisSummary;

/// Outcome of the transcoding test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscodeState {
    None,
    Suspected,
    Detected,
}

/// The three independent detections plus a human summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detections {
    pub upscaling: bool,
    pub upsampling: bool,
    pub transcoding: TranscodeState,
    /// One-line explanation of the flagged issues (or why the file looks clean).
    pub detail: String,
    /// Quick status word: "Clean", "Suspicious", or "Flagged".
    pub summary: String,
}

impl Detections {
    pub fn unknown() -> Self {
        Detections {
            upscaling: false,
            upsampling: false,
            transcoding: TranscodeState::None,
            detail: "Not yet analyzed.".to_string(),
            summary: "Unknown".to_string(),
        }
    }
}

// --- Tunable thresholds -----------------------------------------------------
/// At/above this fraction of Nyquist, content is considered full-band.
const CLEAN_RATIO: f64 = 0.88;
/// Rates above this are "hi-res" containers subject to the upsampling check.
const HIRES_RATE: u32 = 48_000;
/// In a hi-res (> 48 kHz) container, content confined below this frequency
/// (CD/DVD range, with margin) means the extra bandwidth is empty == upsampled.
const UPSAMPLE_MAX_HZ: f64 = 30_000.0;
/// STFT: minimum drop (dB) at the cut-off to call it a brick wall.
const CLIFF_DB: f32 = 22.0;
/// STFT: the band above the cut-off must be at least this quiet (dead zone).
const DEAD_ZONE_DB: f32 = -82.0;
/// MDCT: fraction of frames that must show a dead zone. Calibrated on real
/// transcodes: genuine music tops out around 0.30 (dark 1990s metal masters),
/// while 128–192 kbps AAC sits at ~1.0, so 0.70 gives a safe margin both ways.
const MDCT_DEAD_FRACTION: f32 = 0.70;
/// MDCT: mean cut-off must be below this fraction of N to matter.
const MDCT_CUTOFF_RATIO: f64 = 0.90;
/// MDCT: the dead zone must be at least this flat/deep (dB rel. frame peak).
const MDCT_DEAD_DB: f32 = -75.0;

/// Whether the STFT shows a genuine lossy-style brick wall.
fn stft_brickwall(s: &AnalysisSummary) -> bool {
    s.cliff_db >= CLIFF_DB && s.above_db < DEAD_ZONE_DB
}

/// Whether the MDCT shows the AAC-SIN dead-zone signature.
fn mdct_signature(s: &AnalysisSummary) -> bool {
    match (s.mdct_dead_fraction, s.mdct_cutoff_ratio, s.mdct_dead_db) {
        (Some(frac), Some(cutoff), Some(dead)) => {
            frac >= MDCT_DEAD_FRACTION && cutoff < MDCT_CUTOFF_RATIO && dead < MDCT_DEAD_DB
        }
        _ => false,
    }
}

/// Run the three detections.
pub fn classify(
    summary: &AnalysisSummary,
    sample_rate: u32,
    declared_bits: Option<u32>,
    real_bit_depth: Option<u32>,
) -> Detections {
    let ratio = summary.cutoff_ratio;
    let brick = stft_brickwall(summary);
    let mdct_dead = mdct_signature(summary);
    // The re-quantization grid is near-conclusive evidence of an AAC source
    // (measured: genuine ≤ 0.014 over all onsets, real transcodes ≥ 0.70).
    let requant = summary
        .requant_rate
        .map(|r| r >= crate::requant::DETECT_RATE)
        .unwrap_or(false);
    let mdct_cutoff_hz = summary
        .mdct_cutoff_ratio
        .map(|r| r * (sample_rate as f64 / 2.0))
        .unwrap_or(sample_rate as f64 / 2.0);
    let mdct_khz = mdct_cutoff_hz / 1000.0;
    let stft_khz = summary.cutoff_hz / 1000.0;

    // 1. Upscaling — fake resolution: the content uses fewer bits than the
    //    container declares (the low bits are always zero). Covers 24→16 as well
    //    as 16→8 and similar.
    let upscaling = matches!(
        (declared_bits, real_bit_depth),
        (Some(d), Some(r)) if r < d
    );

    // 2. Upsampling — fake sample rate. In a hi-res container (> 48 kHz), a
    //    consistent MDCT dead zone confined to the CD/DVD range means the extra
    //    bandwidth is empty. (The STFT cut-off is unreliable on real music — it
    //    usually reads full-band — so the MDCT dead zone is used here.)
    let upsampling =
        sample_rate > HIRES_RATE && mdct_dead && mdct_cutoff_hz <= UPSAMPLE_MAX_HZ;

    // 3. Transcoding — lossy source. Per the LAC paper, transcoding detection
    //    only applies at <= 48 kHz; above that a band limit is upsampling.
    //    Evidence strength: re-quantization grid > MDCT dead zone > brick wall.
    let transcoding = if requant && sample_rate <= HIRES_RATE {
        TranscodeState::Detected
    } else if upsampling {
        TranscodeState::None
    } else if sample_rate <= HIRES_RATE && mdct_dead {
        TranscodeState::Detected
    } else if sample_rate <= HIRES_RATE && brick {
        TranscodeState::Detected
    } else if ratio < CLEAN_RATIO {
        TranscodeState::Suspected
    } else {
        TranscodeState::None
    };

    // Build the human explanation.
    let mut reasons: Vec<String> = Vec::new();
    if upscaling {
        reasons.push(format!(
            "Upscaling: ~{}-bit content stored as {}-bit",
            real_bit_depth.unwrap_or(0),
            declared_bits.unwrap_or(0)
        ));
    }
    if upsampling {
        reasons.push(format!(
            "Upsampling: content stops at ~{:.1} kHz in a {:.1} kHz container",
            mdct_khz,
            sample_rate as f64 / 1000.0
        ));
    }
    match transcoding {
        TranscodeState::Detected if requant => reasons.push(format!(
            "Transcoding: AAC re-quantization grid found at a synchronized MDCT onset ({:.0}% of bands on-grid) — near-conclusive AAC source",
            summary.requant_rate.unwrap_or(0.0) * 100.0
        )),
        TranscodeState::Detected if mdct_dead => reasons.push(format!(
            "Transcoding: AAC dead zone in the MDCT domain above ~{mdct_khz:.1} kHz (lossy source)"
        )),
        TranscodeState::Detected => reasons.push(format!(
            "Transcoding: brick-wall cut-off at ~{stft_khz:.1} kHz (lossy source)"
        )),
        TranscodeState::Suspected => reasons.push(format!(
            "Possible transcoding: early roll-off at ~{stft_khz:.1} kHz (could be a naturally dark or acoustic master)"
        )),
        TranscodeState::None => {}
    }

    let flagged = upscaling || upsampling || transcoding == TranscodeState::Detected;
    let summary_word = if flagged {
        "Flagged"
    } else if transcoding == TranscodeState::Suspected {
        "Suspicious"
    } else {
        "Clean"
    };

    let detail = if reasons.is_empty() {
        format!("Clean — full-band content to ~{stft_khz:.1} kHz, no lossy signature.")
    } else {
        reasons.join(" · ")
    };

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
    use crate::ClippingInfo;

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
                clipped: false,
            },
            fake_stereo: false,
            real_bit_depth: None,
            mdct_cutoff_ratio: mcr,
            mdct_dead_db: mdb,
            mdct_dead_fraction: frac,
            requant_rate: None,
        }
    }

    #[test]
    fn clean_full_band() {
        let d = classify(&summ(21000.0, 44100, 4.0, -70.0, None), 44100, Some(16), Some(16));
        assert!(!d.upscaling && !d.upsampling);
        assert_eq!(d.transcoding, TranscodeState::None);
        assert_eq!(d.summary, "Clean");
    }

    #[test]
    fn brickwall_is_transcoded() {
        let d = classify(&summ(16000.0, 44100, 40.0, -110.0, None), 44100, Some(16), Some(16));
        assert_eq!(d.transcoding, TranscodeState::Detected);
    }

    #[test]
    fn requant_grid_alone_is_transcoded() {
        // Full-band spectrum (256 kbps AAC keeps the whole band: no dead zone,
        // no brick wall) but the re-quantization grid was found.
        let mut s = summ(21500.0, 44100, 3.0, -70.0, None);
        s.requant_rate = Some(0.88);
        let d = classify(&s, 44100, Some(16), Some(16));
        assert_eq!(d.transcoding, TranscodeState::Detected);
        assert!(d.detail.contains("re-quantization"));
    }

    #[test]
    fn low_requant_rate_does_not_flag() {
        let mut s = summ(21500.0, 44100, 3.0, -70.0, None);
        s.requant_rate = Some(0.02); // genuine files measure ≤ ~0.014
        let d = classify(&s, 44100, Some(16), Some(16));
        assert_eq!(d.transcoding, TranscodeState::None);
    }

    #[test]
    fn aac_mdct_signature_is_transcoded() {
        // No STFT brick wall, but a clear MDCT dead zone.
        let d = classify(
            &summ(19000.0, 44100, 6.0, -70.0, Some((0.85, 0.80, -90.0))),
            44100,
            Some(16),
            Some(16),
        );
        assert_eq!(d.transcoding, TranscodeState::Detected);
    }

    #[test]
    fn fake_hires_is_upscaled_and_upsampled() {
        // 96 kHz container, 24-bit declared but 16-bit real, content confined to
        // ~21.6 kHz (MDCT dead zone) => both upscaled and upsampled.
        let d = classify(
            &summ(21600.0, 96000, 40.0, -115.0, Some((1.0, 0.45, -120.0))),
            96000,
            Some(24),
            Some(16),
        );
        assert!(d.upscaling);
        assert!(d.upsampling);
        assert_eq!(d.transcoding, TranscodeState::None);
    }

    #[test]
    fn dark_acoustic_is_suspicious_not_transcoded() {
        // Early roll-off, gentle (no cliff), no MDCT dead zone.
        let d = classify(&summ(18000.0, 44100, 8.0, -72.0, Some((0.1, 0.95, -60.0))), 44100, Some(16), Some(16));
        assert_eq!(d.transcoding, TranscodeState::Suspected);
        assert_eq!(d.summary, "Suspicious");
    }
}
