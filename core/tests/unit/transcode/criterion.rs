use super::*;

/// Values of erf from published tables, independent of the approximation
/// being tested. The stated accuracy of the fit is 1.2e-7.
#[test]
fn erf_matches_published_values() {
    for (x, want) in [
        (0.0, 0.0),
        (0.1, 0.112_462_916),
        (0.5, 0.520_499_878),
        (1.0, 0.842_700_793),
        (1.5, 0.966_105_146),
        (2.0, 0.995_322_265),
        (3.0, 0.999_977_910),
    ] {
        // The fit's stated bound is a *fractional* error of 1.2e-7 on
        // erfc; carried into erf that is worst near x = 0, where erfc ≈ 1.
        assert!((erf(x) - want).abs() < 3e-7, "erf({x}) = {} ", erf(x));
        // erf is odd — a sign bug is invisible on positive inputs alone.
        assert!((erf(-x) + want).abs() < 3e-7, "erf({}) ", -x);
    }
}

#[test]
fn erfinv_inverts_erf() {
    for i in -99..=99 {
        if i == 0 {
            // Skipped on purpose, and the reason is worth recording: the
            // rational fit is not exactly odd — it evaluates to −3e-8 at
            // zero rather than to zero. `erfinv(0)` short-circuits to the
            // mathematically exact 0, so the round trip through the
            // *approximation* misses by that 3e-8. Everywhere else the
            // solve lands at machine precision, which is what this test
            // is actually here to prove.
            continue;
        }
        let y = i as f64 / 100.0;
        let x = erfinv(y);
        assert!(
            (erf(x) - y).abs() < 1e-12,
            "erfinv({y}) = {x}, erf of that = {}",
            erf(x)
        );
    }
    assert_eq!(erfinv(0.0), 0.0);
    assert!(erfinv(1.0).is_infinite());
    assert!(erfinv(-1.0).is_infinite());
    // Out of domain must not loop forever or return a silent wrong value.
    assert!(erfinv(1.5).is_infinite());
    assert!(erfinv(-1.5).is_infinite());
}

/// τ must sit below the lossless mean of 1/12 (it is a left-tail bound),
/// must rise towards that mean as the subband narrows (a short band's
/// average is noisier, so the tail is fatter), and must never go negative
/// — a negative threshold would make the test unsatisfiable.
#[test]
fn thresholds_are_a_left_tail_of_the_lossless_distribution() {
    let widths = [4usize, 8, 16, 32, 64, 128, 1024];
    let taus = subband_thresholds(&widths, TARGET_PROBABILITY);
    assert_eq!(taus.len(), widths.len());

    for (k, tau) in widths.iter().zip(&taus) {
        assert!(*tau > 0.0, "K={k}: τ={tau} must be positive");
        assert!(*tau < 1.0 / 12.0, "K={k}: τ={tau} must be below μ");
    }
    for pair in taus.windows(2) {
        assert!(
            pair[1] > pair[0],
            "τ must grow towards μ as K grows: {pair:?}"
        );
    }
    // A band with no coefficients: nothing can fall below zero.
    assert_eq!(subband_thresholds(&[0], TARGET_PROBABILITY), vec![0.0]);
}

/// The defining property, checked against eq. (7) rather than against a
/// remembered number: τ is the point where a genuine lossless subband's
/// error falls below it with probability exactly P.
#[test]
fn thresholds_satisfy_their_defining_probability() {
    const MU: f64 = 1.0 / 12.0;
    for k in [4usize, 25, 96, 1024] {
        let tau = subband_thresholds(&[k], TARGET_PROBABILITY)[0];
        let sigma = (1.0 / (180.0 * k as f64)).sqrt();
        let s2 = std::f64::consts::SQRT_2 * sigma;
        // Eq. (7): prob{E < τ} = ½[erf(μ/√2σ) − erf((μ−τ)/√2σ)]
        let p = 0.5 * (erf(MU / s2) - erf((MU - tau) / s2));
        assert!(
            (p - TARGET_PROBABILITY).abs() < 1e-9,
            "K={k}: prob = {p}, want {TARGET_PROBABILITY}"
        );
    }
}

/// Coefficients placed exactly on a quantization lattice must be
/// recognised at the scalefactor that generated them, and ordinary
/// coefficients must not be. This is the detector's entire premise, tested
/// on data built by hand rather than by any part of the implementation.
#[test]
fn on_grid_coefficients_are_detected_and_arbitrary_ones_are_not() {
    const BIAS: f64 = 60.0;
    let delta = 0.5f64;

    // Build a subband genuinely on the grid, by inverting the scaling to
    // recover the coefficients an encoder producing chosen integers would
    // have decoded to.
    //
    // The subtlety that broke a first attempt at this test: `prepare`
    // derives `sf_max` from the band's *own* peak, so the grid cannot be
    // chosen first and the coefficients second — picking a peak of 1.0
    // and then generating coefficients that top out at 0.117 shifts
    // `sf_max` by twelve units and the values land nowhere near the grid
    // they were built for. The peak has to be a fixed point.
    //
    // Solving `peak^(3/4)·factor(peak) = m` for the largest integer `m`
    // gives it in closed form, since the exponents collapse to
    // `scaled_peak = p^(0.75(1−δ))·2^(11.25−12.25δ)`.
    let largest = 35.0f64;
    let peak = (largest / (11.25 - 12.25 * delta).exp2()).powf(1.0 / (0.75 * (1.0 - delta)));
    let sf_max = 4.0 * peak.log2() + 16.0 / 3.0 + BIAS;
    let factor = (-3.0 / 16.0 * (delta * sf_max - BIAS)).exp2();

    let mut integers: Vec<f64> = (0..31).map(|i| ((i % 7) + 1) as f64).collect();
    integers.push(largest);
    let on_grid: Vec<f64> = integers
        .iter()
        // |X| = (|X|^(3/4))^(4/3)
        .map(|n| (n / factor).powf(4.0 / 3.0))
        .collect();

    let taus = subband_thresholds(&[on_grid.len()], TARGET_PROBABILITY);
    let mut sb = PreparedSubband::new();

    assert!(sb.prepare(&on_grid, BIAS));
    assert_eq!(
        sb.on_grid_count(&[delta], BIAS, taus[0]),
        1,
        "coefficients placed on the lattice must register"
    );

    // Coefficients with no relationship to any lattice.
    //
    // Pseudo-random, not `sin(i·e)`. That was the first choice, on the
    // reasoning that irrational spacing cannot align with anything — but
    // the statistic here assumes the rounding errors of a band are
    // *independent*, and a deterministic sinusoid's are not. Its errors
    // fell below τ for 16 of 64 scalefactors instead of the 1% the
    // threshold is built for, which would have read as the detector
    // firing on clean data when in fact the test data was not clean in
    // the sense the maths requires.
    let mut state: u32 = 0x1234_ABCD;
    let off_grid: Vec<f64> = (0..2048)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state as f64 / u32::MAX as f64) - 0.5
        })
        .collect();
    let taus_off = subband_thresholds(&[off_grid.len()], TARGET_PROBABILITY);
    let deltas = scalefactor_grid(0.3, 0.7, 64);
    assert!(sb.prepare(&off_grid, BIAS));
    let hits = sb.on_grid_count(&deltas, BIAS, taus_off[0]);
    assert!(
        hits <= 2,
        "arbitrary coefficients should almost never look on-grid, got {hits}/64"
    );
}

/// Low-level random coefficients must not count as a codec lattice just
/// because every quantization index is zero.
#[test]
fn almost_silent_bands_do_not_match_every_scalefactor() {
    const BIAS: f64 = 60.0;
    let deltas = scalefactor_grid(0.3, 0.7, 64);

    let shape: Vec<f64> = {
        let mut state: u32 = 0x9E37_79B9;
        (0..64)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                (state as f64 / u32::MAX as f64) * 2.0 - 1.0
            })
            .collect()
    };
    let quiet: Vec<f64> = shape.iter().map(|v| v * 1e-6).collect();
    let loud: Vec<f64> = shape.iter().map(|v| v * 0.5).collect();

    let taus = subband_thresholds(&[shape.len()], TARGET_PROBABILITY);
    let mut sb = PreparedSubband::new();

    assert!(sb.prepare(&quiet, BIAS));
    assert_eq!(
        sb.on_grid_count(&deltas, BIAS, taus[0]),
        0,
        "all-zero quantization must not count as evidence"
    );

    assert!(sb.prepare(&loud, BIAS));
    let honest = sb.on_grid_count(&deltas, BIAS, taus[0]);
    assert!(honest <= 4, "same data at a normal level: {honest}/64");
}

/// Silence carries no lattice. It must be reported as such rather than
/// producing NaN and relying on NaN comparisons to fall the right way.
#[test]
fn a_silent_subband_is_refused_rather_than_producing_nan() {
    let mut sb = PreparedSubband::new();
    assert!(!sb.prepare(&[0.0; 64], 60.0));
    assert_eq!(sb.on_grid_count(&[0.5], 60.0, 0.05), 0);
    // An empty band, and one holding non-finite values from a broken
    // decode, must be refused the same way.
    assert!(!sb.prepare(&[], 60.0));
    assert!(!sb.prepare(&[f64::NAN, 1.0], 60.0));
    assert!(!sb.prepare(&[f64::INFINITY], 60.0));
}

#[test]
fn scalefactor_grid_spans_its_bounds() {
    let g = scalefactor_grid(0.3, 0.7, 64);
    assert_eq!(g.len(), 64);
    assert!((g[0] - 0.3).abs() < 1e-12);
    assert!((g[63] - 0.7).abs() < 1e-12);
    for pair in g.windows(2) {
        assert!(pair[1] > pair[0], "must be increasing");
    }
    // Degenerate counts must not divide by zero.
    assert_eq!(scalefactor_grid(0.3, 0.7, 1), vec![0.3]);
    assert!(scalefactor_grid(0.3, 0.7, 0).is_empty());
}
