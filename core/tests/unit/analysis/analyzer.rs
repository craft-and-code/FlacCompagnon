use super::*;

/// A pure sine has a crest factor of exactly sqrt(2) == 3.01 dB; the DR
/// estimate on a steady full-scale sine must land there.
#[test]
fn dr_of_a_pure_sine_is_3db() {
    let mut a = StreamAnalyzer::new(1);
    let rate = 44_100.0f64;
    for n in 0..(DYN_BLOCK_FRAMES * 2) {
        let s = (2.0 * std::f64::consts::PI * 1000.0 * n as f64 / rate).sin() as f32 * 0.9;
        a.push_frame(&[s], None);
    }
    let summary = a.finish(44_100, None);
    let dr = summary.dr_db.expect("dr computed");
    assert!((dr - 3.01).abs() < 0.1, "dr = {dr}");
}

/// A hard-clipped ("brickwalled") sine approaches a square wave whose
/// crest factor tends to 0 dB — the loudness-war signature.
#[test]
fn dr_of_a_squashed_sine_is_low() {
    let mut a = StreamAnalyzer::new(1);
    let rate = 44_100.0f64;
    for n in 0..(DYN_BLOCK_FRAMES * 2) {
        let raw = (2.0 * std::f64::consts::PI * 1000.0 * n as f64 / rate).sin() * 8.0;
        let s = raw.clamp(-0.98, 0.98) as f32;
        a.push_frame(&[s], None);
    }
    let summary = a.finish(44_100, None);
    let dr = summary.dr_db.expect("dr computed");
    assert!(dr < 1.0, "dr = {dr}");
}

/// Silence yields no DR value rather than a bogus number.
#[test]
fn dr_of_silence_is_none() {
    let mut a = StreamAnalyzer::new(1);
    for _ in 0..(DYN_BLOCK_FRAMES + 10) {
        a.push_frame(&[0.0], None);
    }
    let summary = a.finish(44_100, None);
    assert!(summary.dr_db.is_none());
}
