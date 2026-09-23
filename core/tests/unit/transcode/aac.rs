use super::*;

/// A reduced search, sized to run inside `cargo test`.
///
/// `frames`, `scalefactors` and `significance` are changed **together**,
/// to the 8×8 pairing the reference documents. They are not independent
/// knobs: a shorter search accumulates fewer chance hits, so it needs a
/// *higher* threshold, and 8×8 comes with 0.031 rather than 64×64's
/// 0.0125.
///
/// Written the wrong way round the first time — 2 frames and 4
/// scalefactors judged against 64×64's 0.0125 — which flagged clean white
/// noise at 0.0164 and looked exactly like a detector bug. It was not: it
/// was the same mistake this port already had to flag in the author's own
/// `main.m`, which runs 64×64 against the threshold for 8×8. Worth
/// recording, because it is evidently easy to make twice.
///
/// `alignments` is the one parameter reduced on its own, and it is safe
/// to reduce alone: fewer alignments can only *lower* the score, never
/// raise it, so a clean verdict stays clean.
fn params_fast() -> AacParams {
    AacParams {
        frames: 8,
        scalefactors: 8,
        significance: 0.031,
        alignments: 16,
        ..AacParams::default()
    }
}

/// Deterministic broadband noise.
///
/// An xorshift32 sequence, not `sin(i·e)`. That was the first attempt and
/// it was a trap: sampling a sine at an irrational rate *looks* random in
/// the time domain but is a single tone in the frequency domain, with a
/// peak-to-median spectral ratio around 10 000 against a real 3. Nearly
/// every scalefactor band of such a signal sits at -120 dB, where the
/// band's largest coefficient itself rounds to zero — which reads as a
/// perfect lattice fit and scored 0.16 against a 0.0125 threshold. Real
/// music never does this: measured across ordinary content, white noise,
/// a -60 dB passage and a 16 kHz-lowpassed signal, not one band out of
/// 384 fell into that regime.
fn noise(n: usize) -> Vec<f64> {
    let mut state: u32 = 0x2545_F491;
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state as f64 / u32::MAX as f64) * 0.8 - 0.4
        })
        .collect()
}

#[test]
fn untabulated_sample_rates_are_refused() {
    let ch = vec![noise(10 * LONG_LEN)];
    for rate in [96_000u32, 88_200, 22_050, 0] {
        assert!(
            detect(&ch, rate, &params_fast(), &|| false).is_none(),
            "rate {rate} should have no tabulated bands"
        );
    }
}

#[test]
fn files_too_short_to_yield_a_frame_are_refused_without_panicking() {
    for n in [0usize, 1, LONG_LEN, 2 * LONG_LEN] {
        let ch = vec![noise(n)];
        assert!(
            detect(&ch, 44_100, &params_fast(), &|| false).is_none(),
            "n={n} should be too short"
        );
    }
    // No channels at all.
    assert!(detect(&[], 44_100, &params_fast(), &|| false).is_none());
}

#[test]
fn cancellation_stops_the_sweep() {
    let ch = vec![noise(10 * LONG_LEN)];
    assert!(detect(&ch, 44_100, &params_fast(), &|| true).is_none());
}

/// Genuine, lattice-free audio must score below the significance
/// threshold. This is the false-positive direction, and it is the one
/// that matters most: calling a real lossless file a fake is the error a
/// user cannot forgive.
#[test]
fn lattice_free_audio_is_not_flagged() {
    let ch = vec![noise(10 * LONG_LEN)];
    let e = detect(&ch, 44_100, &params_fast(), &|| false).expect("should run");
    assert!(
        !e.detected,
        "noise scored {} against λ={}",
        e.likelihood,
        params_fast().significance
    );
    assert!((0.0..=1.0).contains(&e.likelihood), "{}", e.likelihood);
    // A margin, not just the verdict: a score creeping up towards the
    // threshold is the early warning that something has drifted, and it
    // would otherwise stay invisible until the day it crosses.
    // 0.75 and not 0.5: broadband noise measures around 0.015 at this
    // search size against a 0.031 threshold, so half would leave three
    // ten-thousandths of slack and fail on a rounding difference rather
    // than on a regression.
    assert!(
        e.likelihood < 0.75 * params_fast().significance,
        "no margin left: {} against λ={}",
        e.likelihood,
        params_fast().significance
    );
}

/// Stereo must be accepted and must not depend on channel order for its
/// verdict: swapping L and R swaps the mid/side pair's sign, which the
/// magnitude-based measurement cannot see.
#[test]
fn stereo_is_accepted_and_channel_order_does_not_change_the_verdict() {
    let l = noise(10 * LONG_LEN);
    let r: Vec<f64> = noise(10 * LONG_LEN).iter().map(|v| v * 0.7).collect();
    let p = params_fast();
    let a = detect(&[l.clone(), r.clone()], 44_100, &p, &|| false).expect("lr");
    let b = detect(&[r, l], 44_100, &p, &|| false).expect("rl");
    assert_eq!(a.detected, b.detected);
    assert!(
        (a.likelihood - b.likelihood).abs() < 1e-12,
        "{a:?} vs {b:?}"
    );
}

/// Digital silence has no lattice and no energy. It must come back clean
/// rather than dividing by zero or reporting certainty.
#[test]
fn silence_is_not_flagged() {
    let ch = vec![vec![0.0; 10 * LONG_LEN]];
    let e = detect(&ch, 44_100, &params_fast(), &|| false).expect("should run");
    assert_eq!(e.likelihood, 0.0);
    assert!(!e.detected);
}
