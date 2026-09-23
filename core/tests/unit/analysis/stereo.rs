use super::*;

#[test]
fn balance_matches_rms_gain_ratio_and_channel_order() {
    // Halving amplitude quarters energy: 20 log10(1/2) = -6.0205999 dB.
    for (left, right, expected) in [(4.0, 1.0, -6.0206), (1.0, 4.0, 6.0206), (4.0, 4.0, 0.0)] {
        let Some(StereoBalance::Measured { right_minus_left_db }) = analyze_balance(left, right) else {
            panic!("both channels contain signal");
        };
        assert!((right_minus_left_db - expected).abs() < 0.0001);
    }
}

#[test]
fn balance_distinguishes_one_silent_channel_from_two() {
    assert_eq!(analyze_balance(0.0, 1.0), Some(StereoBalance::LeftSilent));
    assert_eq!(analyze_balance(1.0, 0.0), Some(StereoBalance::RightSilent));
    assert_eq!(analyze_balance(0.0, 0.0), None);
    // A tiny non-zero float signal is not digital silence.
    assert!(matches!(analyze_balance(1e-40, 1e-40), Some(StereoBalance::Measured { .. })));
}

#[test]
fn invalid_channel_energies_never_produce_balance_or_phase_readings() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
        assert_eq!(analyze_balance(invalid, 1.0), None);
        assert_eq!(analyze_balance(1.0, invalid), None);
        assert_eq!(analyze_phase(invalid, 1.0, 0.0).correlation, None);
        assert_eq!(analyze_phase(1.0, invalid, 0.0).correlation, None);
    }
    assert_eq!(analyze_phase(1.0, 1.0, f64::NAN).correlation, None);
    assert_eq!(analyze_phase(1.0, 0.0, 0.0).correlation, None);
}

#[test]
fn same_polarity_with_unequal_gains_is_not_inverted() {
    let phase = analyze_phase(100.0, 16.0, 40.0);
    assert_eq!(phase.correlation, Some(1.0));
    assert!(!phase.likely_inverted);
}

#[test]
fn identical_channels_are_fake() {
    assert!(is_fake(0.0, 100.0, 100.0, 1000, 1000));
}

#[test]
fn decorrelated_channels_are_real() {
    // Large difference energy relative to signal.
    assert!(!is_fake(150.0, 100.0, 100.0, 0, 1000));
}

#[test]
fn tiny_difference_is_fake() {
    // Difference 70 dB down -> effectively dual mono.
    let sig = 200.0;
    let diff = sig * 10f64.powf(-70.0 / 10.0);
    assert!(is_fake(diff, 100.0, 100.0, 0, 1000));
}

#[test]
fn opposite_channels_cancel_in_mono_and_have_negative_correlation() {
    // For R = -L, both channel energies equal E and their dot product is -E.
    let phase = analyze_phase(100.0, 100.0, -100.0);
    assert_eq!(phase.correlation, Some(-1.0));
    assert!(phase.likely_inverted);
}

#[test]
fn unrelated_or_silent_channels_are_not_called_inverted() {
    let unrelated = analyze_phase(100.0, 100.0, 0.0);
    assert_eq!(unrelated.correlation, Some(0.0));
    assert!(!unrelated.likely_inverted);
    let silence = analyze_phase(0.0, 0.0, 0.0);
    assert!(silence.correlation.is_none());
    assert!(!silence.likely_inverted);
}

#[test]
fn opposite_channels_with_unequal_gains_are_still_likely_inverted() {
    // R = -0.4 L is still an inversion even though the mono sum does not
    // vanish. Its correlation is -1 independently of the gain difference.
    let phase = analyze_phase(100.0, 16.0, -40.0);
    assert_eq!(phase.correlation, Some(-1.0));
    assert!(phase.likely_inverted);
}

#[test]
fn moderately_negative_correlation_does_not_claim_an_inversion() {
    let phase = analyze_phase(100.0, 100.0, -50.0);
    assert_eq!(phase.correlation, Some(-0.5));
    assert!(!phase.likely_inverted);
}
