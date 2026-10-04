use super::*;

#[test]
fn balance_matches_rms_gain_ratio_and_channel_order() {
    // Halving amplitude quarters energy: 20 log10(1/2) = -6.0205999 dB.
    for (left, right, expected) in [(4.0, 1.0, -6.0206), (1.0, 4.0, 6.0206), (4.0, 4.0, 0.0)] {
        let Some(StereoBalance::Measured {
            right_minus_left_db,
        }) = analyze_balance(left, right)
        else {
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
    assert!(matches!(
        analyze_balance(1e-40, 1e-40),
        Some(StereoBalance::Measured { .. })
    ));
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
fn identical_silent_channels_do_not_create_a_fake_stereo_finding() {
    assert!(!is_fake(0.0, 0.0, 0.0, 1000, 1000));
    assert!(!is_fake(0.0, 0.0, 0.0, 0, 0));
    // Preserve exact-dual-mono readings even below the approximate-energy gate.
    assert!(is_fake(0.0, 1e-30, 1e-30, 1000, 1000));
}

#[test]
fn invalid_energies_cannot_create_a_fake_stereo_finding() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
        assert!(!is_fake(invalid, 1.0, 1.0, 1000, 1000));
        assert!(!is_fake(0.0, invalid, 1.0, 1000, 1000));
        assert!(!is_fake(0.0, 1.0, invalid, 1000, 1000));
    }
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

#[test]
fn phase_keeps_its_gain_invariant_coefficient_without_energy_product_overflow() {
    // E_R = 4 E_L and C = -2 E_L describe R = -2 L, at any common gain.
    for scale in [1e-300, 1e-30, 1.0, 1e150, 1e300] {
        let measured = analyze_phase(scale, 4.0 * scale, -2.0 * scale);
        assert_eq!(measured.correlation, Some(-1.0), "scale {scale}");
        assert!(measured.likely_inverted);
    }
    // Vastly unequal but finite channel energies still obey Cauchy-Schwarz.
    assert_eq!(analyze_phase(1e-300, 1e300, 0.5).correlation, Some(0.5));
    assert_eq!(analyze_phase(1e300, 1e-300, 0.5).correlation, Some(0.5));
}

#[test]
fn balance_is_common_gain_invariant_even_when_an_energy_ratio_would_overflow() {
    for scale in [1e-300, 1e-30, 1.0, 1e150, 1e300] {
        let Some(StereoBalance::Measured {
            right_minus_left_db,
        }) = analyze_balance(scale, 4.0 * scale)
        else {
            panic!("finite positive energies");
        };
        assert!((right_minus_left_db - 6.0206).abs() < 1e-4);
    }
    let Some(StereoBalance::Measured {
        right_minus_left_db,
    }) = analyze_balance(1e-300, 1e300)
    else {
        panic!("finite positive energies");
    };
    assert_eq!(right_minus_left_db, 6000.0);
}

#[test]
fn raw_rms_balance_includes_dc_and_does_not_infer_polarity_or_loudness() {
    // Orthogonal AC +/-A with a constant d has mean-square A^2+d^2.
    let ac = [0.25_f64, -0.25, 0.25, -0.25];
    let left = ac.iter().map(|x| x * x).sum();
    let right = ac.iter().map(|x| (x + 0.5).powi(2)).sum();
    let Some(StereoBalance::Measured {
        right_minus_left_db,
    }) = analyze_balance(left, right)
    else {
        panic!("both channels contain signal");
    };
    // AC energy is equal, but right raw mean-square is five times larger.
    assert!((right_minus_left_db - 10.0 * 5.0_f32.log10()).abs() < 1e-5);
}
