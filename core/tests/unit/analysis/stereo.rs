use super::*;

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
