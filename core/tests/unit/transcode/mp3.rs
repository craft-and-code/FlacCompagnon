use super::*;

fn params_fast() -> Mp3Params {
    // `alignments` is the one parameter safe to reduce alone: fewer
    // alignments can only lower the score, never raise it, so a clean
    // verdict stays clean. `frames`/`scalefactors`/`significance` are left
    // at the 8×8 pairing, with this port's own 0.031 threshold — see
    // `Mp3Params::significance` for why it is not the reference's 0.025.
    Mp3Params {
        alignments: 12,
        ..Mp3Params::default()
    }
}

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
fn the_windows_have_the_right_lengths_and_edges() {
    let w = Windows::new();
    assert_eq!(w.long.len(), N_LONG);
    assert_eq!(w.start.len(), N_LONG);
    assert_eq!(w.stop.len(), N_LONG);
    assert_eq!(w.short.len(), N_SHORT);
    // The transition shapes are asymmetric: `start` ends in silence,
    // `stop` begins in it.
    assert!(w.start[N_LONG - 1].abs() < 1e-12);
    assert!(w.stop[0].abs() < 1e-12);
    for (name, shape) in [("long", &w.long), ("start", &w.start), ("stop", &w.stop)] {
        assert!(
            shape.iter().all(|v| (0.0..=1.0).contains(v)),
            "{name} leaves [0,1]"
        );
    }
}

/// The layout claim `short_shape` relies on: one band across all three
/// short transforms is a contiguous slice. Checked by filling the
/// spectrum with its own indices and reading a band back.
#[test]
fn a_short_band_is_contiguous_in_the_interleaved_layout() {
    let mut worker = Worker::new();
    for (i, v) in worker.spectrum.iter_mut().enumerate() {
        *v = i as f64;
    }
    // Band covering coefficients 4..9 of each of the three transforms.
    let (s, e) = (4usize, 9usize);
    let slice = &worker.spectrum[s * SHORT_COUNT..e * SHORT_COUNT];
    assert_eq!(slice.len(), (e - s) * SHORT_COUNT);
    // Every (coefficient, transform) pair in the band, and nothing else.
    for c in s..e {
        for w in 0..SHORT_COUNT {
            assert!(
                slice.contains(&((c * SHORT_COUNT + w) as f64)),
                "c={c} w={w}"
            );
        }
    }
}

#[test]
fn untabulated_rates_and_short_files_are_refused() {
    let ch = vec![noise(GRANULE_LEN * 12)];
    for rate in [96_000u32, 88_200, 22_050, 0] {
        assert!(
            detect(&ch, rate, &params_fast(), &|| false).is_none(),
            "{rate}"
        );
    }
    for n in [0usize, 1, GRANULE_LEN, GRANULE_LEN * 4] {
        assert!(
            detect(&[noise(n)], 44_100, &params_fast(), &|| false).is_none(),
            "n={n}"
        );
    }
    assert!(detect(&[], 44_100, &params_fast(), &|| false).is_none());
}

#[test]
fn cancellation_stops_the_sweep() {
    let ch = vec![noise(GRANULE_LEN * 12)];
    assert!(detect(&ch, 44_100, &params_fast(), &|| true).is_none());
}

/// A golden value: the whole chain, end to end, against a number computed
/// outside this codebase.
///
/// Every other test here checks a property — "clean audio is not flagged",
/// "the windows are 36 long". Properties are satisfied by a great many
/// wrong implementations, and one of them bit hard: on a real 128 kbps
/// transcode an independent reference measured 0.0796 while this code
/// reported the file clean, with every property test green.
///
/// So this one pins the *number*. The input is deterministic to the bit
/// (xorshift32 from a fixed seed), the search is fixed at 12 alignments,
/// and 0.020270 is what an independent implementation of the same chain
/// produces on that exact input. A mismatch localises the fault
/// immediately: the transform, the filterbank, the butterflies and the
/// criterion all feed this single figure, and none of them can drift
/// without moving it.
#[test]
fn the_whole_chain_reproduces_an_independently_computed_value() {
    let ch = vec![noise(GRANULE_LEN * 16)];
    let params = Mp3Params {
        alignments: 12,
        ..Mp3Params::default()
    };
    let e = detect(&ch, 44_100, &params, &|| false).expect("should run");
    assert!(
        (e.likelihood - 0.020_270).abs() < 5e-5,
        "expected 0.020270, got {:.6} — the chain has drifted",
        e.likelihood
    );
}

/// The false-positive direction, the one a user cannot forgive.
#[test]
fn lattice_free_audio_is_not_flagged() {
    let ch = vec![noise(GRANULE_LEN * 16)];
    let e = detect(&ch, 44_100, &params_fast(), &|| false).expect("should run");
    assert!(
        !e.detected,
        "noise scored {} against λ={}",
        e.likelihood,
        params_fast().significance
    );
    assert!((0.0..=1.0).contains(&e.likelihood));
}

#[test]
fn silence_is_not_flagged() {
    let ch = vec![vec![0.0; GRANULE_LEN * 16]];
    let e = detect(&ch, 44_100, &params_fast(), &|| false).expect("should run");
    assert_eq!(e.likelihood, 0.0);
    assert!(!e.detected);
}

#[test]
fn stereo_is_accepted() {
    let l = noise(GRANULE_LEN * 16);
    let r: Vec<f64> = l.iter().map(|v| v * 0.7).collect();
    let e = detect(&[l, r], 44_100, &params_fast(), &|| false).expect("stereo");
    assert!(!e.detected, "{}", e.likelihood);
}
