use super::*;

/// Bands must tile the transform exactly: no gap, no overlap, ending on
/// the last coefficient. A table mistyped by one would otherwise measure
/// a band that straddles two of the encoder's, which is precisely the
/// error that would blunt the detector without breaking it visibly.
#[test]
fn bands_tile_the_transform_without_gaps_or_overlaps() {
    for rate in [32u32, 44, 48] {
        for (bands, total, what) in [
            (long_bands(rate).expect("long"), LONG_LEN, "long"),
            (short_bands(rate).expect("short"), SHORT_LEN, "short"),
        ] {
            assert_eq!(bands[0].start, 0, "{rate}k {what}: must start at 0");
            assert_eq!(
                bands[bands.len() - 1].end,
                total,
                "{rate}k {what}: must end at {total}"
            );
            for pair in bands.windows(2) {
                assert_eq!(pair[0].end, pair[1].start, "{rate}k {what}: not contiguous");
            }
            for b in &bands {
                assert!(b.width() > 0, "{rate}k {what}: empty band {b:?}");
            }
        }
    }
    assert!(long_bands(96).is_none());
    assert!(short_bands(22).is_none());
}

/// Band widths must never decrease with frequency. AAC's bands follow
/// critical bands, which widen as frequency rises — a table entry
/// transposed by mistake would show up as a band narrower than the one
/// below it.
#[test]
fn band_widths_never_shrink_with_frequency() {
    for rate in [32u32, 44, 48] {
        let bands = long_bands(rate).expect("long");
        // The topmost band is the remainder up to 1024 and can be wider
        // or narrower than its neighbour, so it is excluded.
        for pair in bands[..bands.len() - 1].windows(2) {
            assert!(
                pair[1].width() >= pair[0].width(),
                "{rate}k: {:?} then {:?}",
                pair[0],
                pair[1]
            );
        }
    }
}

/// 32 kHz has a lower Nyquist, so the standard gives it more bands over
/// the same 1024 coefficients.
#[test]
fn thirty_two_khz_has_its_own_layout() {
    assert_eq!(long_bands(44).expect("44").len(), 49);
    assert_eq!(long_bands(48).expect("48").len(), 49);
    assert_eq!(long_bands(32).expect("32").len(), 51);
    assert_eq!(long_bands(44), long_bands(48), "44.1 and 48 share a layout");
}

/// The Princen-Bradley condition: for the transform to reconstruct
/// perfectly, a window's two halves must satisfy `w[n]² + w[n+N]² = 1`.
/// This is the property that makes it a valid MDCT window at all, and it
/// is checked rather than assumed because a sign or an off-by-half in the
/// sine argument still *looks* like a smooth window.
#[test]
fn window_halves_satisfy_the_perfect_reconstruction_condition() {
    for len in [2 * SHORT_LEN, 2 * LONG_LEN] {
        let w = sine_window(len);
        let half = len / 2;
        for n in 0..half {
            let sum = w[n] * w[n] + w[n + half] * w[n + half];
            assert!((sum - 1.0).abs() < 1e-12, "len={len} n={n}: {sum}");
        }
    }
}

#[test]
fn the_four_shapes_have_the_right_lengths_and_edges() {
    let w = Windows::new();
    assert_eq!(w.long.len(), 2 * LONG_LEN);
    assert_eq!(w.start.len(), 2 * LONG_LEN);
    assert_eq!(w.stop.len(), 2 * LONG_LEN);
    assert_eq!(w.short.len(), 2 * SHORT_LEN);

    // `start` ends in silence, `stop` begins in it — that asymmetry is
    // the whole point of the transition shapes.
    assert!(w.start[2 * LONG_LEN - 1].abs() < 1e-12);
    assert!(w.stop[0].abs() < 1e-12);
    assert!(
        w.start[0] > 0.0 && w.start[0] < 1e-2,
        "long rise starts near 0"
    );
    assert!(w.stop[2 * LONG_LEN - 1] > 0.0 && w.stop[2 * LONG_LEN - 1] < 1e-2);

    // The flat sections really are flat.
    assert!(w.start[LONG_LEN..LONG_LEN + SHORT_OFFSET]
        .iter()
        .all(|v| (*v - 1.0).abs() < 1e-12));
    assert!(w.stop[SHORT_OFFSET + SHORT_LEN..LONG_LEN]
        .iter()
        .all(|v| (*v - 1.0).abs() < 1e-12));

    // Every sample of every shape stays within [0, 1].
    for (name, shape) in [
        ("long", &w.long),
        ("start", &w.start),
        ("stop", &w.stop),
        ("short", &w.short),
    ] {
        assert!(
            shape.iter().all(|v| (0.0..=1.0).contains(v)),
            "{name} leaves [0,1]"
        );
    }
}
