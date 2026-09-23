use super::*;

/// The MDCT computed straight from its definition, with no FFT anywhere
/// near it:
///
/// `X(f) = Σ x(t) · cos( π/N · (t + ½ + N/2)(f + ½) )`
///
/// Deliberately the slowest possible implementation. Its whole value is
/// that it shares no code, no library and no factorisation with
/// [`Mdct::transform`] — if both agree, the fast path's twiddle factors
/// and phase convention are right, which is precisely what a test written
/// against the fast path's own output could never tell us.
fn naive_mdct(input: &[f64]) -> Vec<f64> {
    let n = input.len() / 2;
    (0..n)
        .map(|f| {
            (0..2 * n)
                .map(|t| {
                    let phase =
                        PI / n as f64 * (t as f64 + 0.5 + n as f64 / 2.0) * (f as f64 + 0.5);
                    input[t] * phase.cos()
                })
                .sum()
        })
        .collect()
}

fn assert_close(got: &[f64], want: &[f64], tol: f64, what: &str) {
    assert_eq!(got.len(), want.len(), "{what}: length");
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        assert!(
            (g - w).abs() < tol,
            "{what}: bin {i} got {g} want {w} (diff {})",
            (g - w).abs()
        );
    }
}

#[test]
fn matches_an_independently_derived_transform() {
    // Every size the two detectors actually use, and a couple more.
    //
    // 6 and 18 are MP3's, and they were missing from this list for a
    // while — which mattered more than it looks: they are the only sizes
    // here that are **not powers of two**, so they are the only ones that
    // exercise rustfft's mixed-radix path rather than its radix-2 one.
    // A transform that is correct at 128 and wrong at 18 would leave every
    // test in this file green while the MP3 detector quietly reported
    // every file clean.
    for n in [6usize, 8, 9, 18, 128, 1024] {
        let input: Vec<f64> = (0..2 * n)
            .map(|t| {
                let x = t as f64 / (2 * n) as f64;
                (7.0 * PI * x).sin() * 0.6 + (31.0 * PI * x).cos() * 0.3
            })
            .collect();

        let mut mdct = Mdct::new(n);
        let mut got = vec![0.0; n];
        assert!(mdct.transform(&input, &mut got));
        // Tolerance scales with the transform size: an N-point sum
        // accumulates rounding, and the naive version sums in a different
        // order from the FFT.
        assert_close(
            &got,
            &naive_mdct(&input),
            1e-9 * n as f64,
            &format!("n={n}"),
        );
    }
}

/// A plan is reused across calls, so a stale buffer would show up as the
/// second call returning the first call's answer.
#[test]
fn reusing_one_instance_gives_independent_results() {
    let n = 64;
    let mut mdct = Mdct::new(n);

    let a: Vec<f64> = (0..2 * n).map(|t| (t as f64 * 0.11).sin()).collect();
    let b: Vec<f64> = (0..2 * n).map(|t| (t as f64 * 0.37).cos()).collect();

    let (mut got_a, mut got_b) = (vec![0.0; n], vec![0.0; n]);
    assert!(mdct.transform(&a, &mut got_a));
    assert!(mdct.transform(&b, &mut got_b));

    assert_close(&got_a, &naive_mdct(&a), 1e-7, "first");
    assert_close(&got_b, &naive_mdct(&b), 1e-7, "second");
}

#[test]
fn silence_transforms_to_silence() {
    let n = 128;
    let mut mdct = Mdct::new(n);
    let mut out = vec![f64::NAN; n];
    assert!(mdct.transform(&vec![0.0; 2 * n], &mut out));
    assert!(out.iter().all(|c| c.abs() < 1e-12), "{out:?}");
}

/// Buffer sizes come from a file's own header, so a mismatch is a
/// malformed-input case, not a programming error: it must be reported,
/// not panicked on.
#[test]
fn wrong_buffer_sizes_are_refused_without_panicking() {
    let mut mdct = Mdct::new(32);
    let mut out = vec![0.0; 32];
    assert!(!mdct.transform(&[0.0; 63], &mut out), "input too short");
    assert!(!mdct.transform(&[0.0; 65], &mut out), "input too long");
    assert!(!mdct.transform(&[], &mut out), "input empty");

    let input = vec![0.0; 64];
    assert!(!mdct.transform(&input, &mut [0.0; 31]), "output too short");
    assert!(!mdct.transform(&input, &mut []), "output empty");

    // The refused calls must not have written anything.
    let mut fresh = vec![f64::NAN; 32];
    assert!(!mdct.transform(&[0.0; 10], &mut fresh));
    assert!(fresh.iter().all(|v| v.is_nan()), "output was touched");
}
