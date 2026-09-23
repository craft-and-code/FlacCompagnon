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
