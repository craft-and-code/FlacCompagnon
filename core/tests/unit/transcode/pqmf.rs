use super::*;

/// The buffer arithmetic, asserted rather than trusted: the eighteenth
/// step's 512-sample window must end exactly on the last sample of the
/// granule, with nothing left over and nothing missing.
#[test]
fn the_lookback_buffer_is_exactly_long_enough() {
    assert_eq!(
        CARRY_START, 96,
        "the tail starts 96 samples into the granule"
    );
    assert_eq!(CARRY, 480, "480 samples carried from the previous granule");
    assert_eq!(CARRY + GRANULE_LEN, 1056);
    let last_step_end = (GRANULE_SIZE - 1) * PQMF_BANDS + PQMF_WINDOW_LEN;
    assert_eq!(last_step_end, CARRY + GRANULE_LEN, "must end flush");
}

fn tone(hz: f64, sr: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| (2.0 * std::f64::consts::PI * hz * i as f64 / sr).sin() * 0.5)
        .collect()
}

/// A pure tone must land in the subband that covers its frequency, and
/// almost nowhere else. Each of the 32 bands spans Nyquist/32 — 689 Hz at
/// 44.1 kHz.
///
/// This is the check that the modulation matrix, the window and the fold
/// agree with each other: get any one of them wrong and the energy lands
/// in the wrong band, or smears across all of them.
///
/// The tones are placed at band *centres* rather than at round
/// frequencies. A round number can fall near a band edge — 9 kHz sits
/// 42 Hz above the 13/14 boundary — where energy legitimately splits
/// between neighbours (72/28 in that case) and a concentration assertion
/// fails for a reason that has nothing to do with correctness.
#[test]
fn a_tone_lands_in_the_band_that_covers_it() {
    let sr = 44_100.0;
    let band_hz = sr / 2.0 / PQMF_BANDS as f64;
    let mut pqmf = Pqmf::new();
    let mut out = vec![0.0; PQMF_BANDS * GRANULE_SIZE];

    for band in [1usize, 4, 13, 21, 28] {
        let hz = (band as f64 + 0.5) * band_hz;
        let sig = tone(hz, sr, GRANULE_LEN * 6);
        // Granules 2 and 3, well past the filterbank's start-up.
        let prev = &sig[2 * GRANULE_LEN..3 * GRANULE_LEN];
        let cur = &sig[3 * GRANULE_LEN..4 * GRANULE_LEN];
        assert!(pqmf.granule(prev, cur, &mut out));

        let energy: Vec<f64> = (0..PQMF_BANDS)
            .map(|b| {
                out[b * GRANULE_SIZE..(b + 1) * GRANULE_SIZE]
                    .iter()
                    .map(|v| v * v)
                    .sum()
            })
            .collect();
        let loudest = energy
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .expect("32 bands");
        assert_eq!(loudest, band, "{hz:.0} Hz should sit in band {band}");

        // And it must dominate: a filterbank that smeared energy across
        // bands could still pick the right maximum by a hair.
        let total: f64 = energy.iter().sum();
        assert!(
            energy[loudest] / total > 0.99,
            "{hz:.0} Hz: only {:.1}% of energy in its own band",
            100.0 * energy[loudest] / total
        );
    }
}

#[test]
fn silence_analyzes_to_silence() {
    let mut pqmf = Pqmf::new();
    let mut out = vec![f64::NAN; PQMF_BANDS * GRANULE_SIZE];
    let zeros = vec![0.0; GRANULE_LEN];
    assert!(pqmf.granule(&zeros, &zeros, &mut out));
    assert!(out.iter().all(|v| v.abs() < 1e-12), "{:?}", &out[..8]);
}

#[test]
fn wrong_buffer_sizes_are_refused_without_panicking() {
    let mut pqmf = Pqmf::new();
    let g = vec![0.0; GRANULE_LEN];
    let short = vec![0.0; GRANULE_LEN - 1];
    let mut out = vec![0.0; PQMF_BANDS * GRANULE_SIZE];
    assert!(!pqmf.granule(&short, &g, &mut out));
    assert!(!pqmf.granule(&g, &short, &mut out));
    assert!(!pqmf.granule(&g, &g, &mut [0.0; 8]));
    assert!(!pqmf.granule(&[], &[], &mut out));
}

/// Odd bands at odd steps, and nothing else. Written as an explicit map
/// because an off-by-one in either index is invisible in the output.
#[test]
fn the_sign_flip_touches_exactly_the_odd_odd_cells() {
    let steps = 4;
    let mut coeffs = vec![1.0; PQMF_BANDS * steps];
    flip_odd_subbands(&mut coeffs, steps);
    for band in 0..PQMF_BANDS {
        for step in 0..steps {
            let want = if band % 2 == 1 && step % 2 == 1 {
                -1.0
            } else {
                1.0
            };
            assert_eq!(coeffs[band * steps + step], want, "band {band} step {step}");
        }
    }
    // Applying it twice restores the original — it is an involution.
    flip_odd_subbands(&mut coeffs, steps);
    assert!(coeffs.iter().all(|v| *v == 1.0));

    // Degenerate input must not panic.
    flip_odd_subbands(&mut [], 0);
    flip_odd_subbands(&mut [1.0], 1);
}
