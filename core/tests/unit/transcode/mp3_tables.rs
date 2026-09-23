use super::*;

/// The property that distinguishes the standard's table from a mistyped
/// one, and the reason it is asserted rather than trusted: 512 numbers
/// copied by hand or by script are exactly the kind of thing that goes
/// wrong silently.
#[test]
fn the_analysis_window_is_symmetric_in_magnitude() {
    for i in 1..256 {
        let a = PQMF_WINDOW[i].abs();
        let b = PQMF_WINDOW[512 - i].abs();
        assert!((a - b).abs() < 1e-12, "i={i}: {a} vs {b}");
    }
    // Its peak sits at the centre.
    let peak = PQMF_WINDOW.iter().map(|v| v.abs()).fold(0.0f64, f64::max);
    assert!((PQMF_WINDOW[256].abs() - peak).abs() < 1e-12);
    assert_eq!(PQMF_WINDOW[0], 0.0);
}

#[test]
fn bands_are_contiguous_and_stop_below_the_full_spectrum() {
    for rate in [32u32, 44, 48] {
        for (b, limit, what) in [
            (long_bands(rate).expect("long"), GRANULE_LEN, "long"),
            (
                short_bands(rate).expect("short"),
                GRANULE_LEN / SHORT_COUNT,
                "short",
            ),
        ] {
            assert_eq!(b[0].start, 0, "{rate}k {what}");
            for pair in b.windows(2) {
                assert_eq!(pair[0].end, pair[1].start, "{rate}k {what}: gap");
            }
            for x in &b {
                assert!(x.width() > 0, "{rate}k {what}: empty band {x:?}");
            }
            // The defining difference from AAC: coverage stops short.
            let last = b[b.len() - 1].end;
            assert!(
                last < limit,
                "{rate}k {what}: {last} should stop below {limit}"
            );
        }
    }
    assert!(long_bands(96).is_none());
}

#[test]
fn butterfly_factors_satisfy_their_identity() {
    let (cs, ca) = butterfly_coefficients();
    for i in 0..8 {
        // cs^2 + ca^2 == 1: the butterfly is a rotation, so it preserves
        // energy. A typo in `ci` would break this.
        assert!((cs[i] * cs[i] + ca[i] * ca[i] - 1.0).abs() < 1e-12, "i={i}");
        assert!(cs[i] > 0.0 && ca[i] < 0.0, "i={i}: {} {}", cs[i], ca[i]);
    }
    // The corrections shrink with distance from the band edge.
    for i in 1..8 {
        assert!(ca[i].abs() < ca[i - 1].abs(), "i={i}");
    }
}
