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
        phase_correlation: None,
        phase_inverted: false,
        real_bit_depth: None,
        bit_depth_evidence: None,
        dr_db: None,
        integrated_lufs: None,
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

/// A skipped codec check must retain its reason without changing the
/// binary findings summary or inventing a successful lattice measurement.
#[test]
fn skipped_codec_checks_keep_a_binary_summary_and_their_explanation() {
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
    assert_eq!(d.summary, "Clean");
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
    assert_eq!(bad.summary, "Clean");

    // Above 48 kHz the test does not apply at all, and says that instead.
    let hi = classify(
        &summ(40_000.0, 96_000, 5.0, -60.0, None),
        96_000,
        Some(24),
        Some(24),
        Err(LatticeSkip::UnsupportedRate(96_000)),
    );
    assert!(hi.detail.contains("not applicable"), "{}", hi.detail);
    assert_eq!(hi.summary, "Clean");

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
fn the_placeholder_flags_nothing() {
    let d = Detections::not_analyzed();
    assert!(!d.upscaling && !d.upsampling && !d.transcoding);
    assert_eq!(d.summary, "Not analyzed");
}
