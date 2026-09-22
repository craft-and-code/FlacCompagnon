//! MPEG-AAC constants: scalefactor band layouts and window shapes.
//!
//! The band tables are **from the standard**, ISO/IEC 13818-7 (MPEG-2 AAC),
//! §"Scalefactor bands and grouping" — they define where one scalefactor's
//! reach ends and the next begins, and a detector that got them wrong would
//! be averaging its measurement across boundaries the encoder never crossed.
//! They are reproduced here as published; they are not anyone's invention and
//! carry no authorship.
//!
//! Only the three sample rates a lossy transcode realistically involves are
//! tabulated. 44.1 and 48 kHz share a layout in the standard; 32 kHz gets two
//! extra bands at the top because its Nyquist sits lower, so the same
//! coefficient index covers less bandwidth and the upper bands can be
//! narrower.
//!
//! # Window shapes
//!
//! AAC switches between one long transform and eight short ones, with two
//! transition shapes bridging them. All four are built from the same sine
//! window, and only sine windows are implemented: the paper checked that
//! Kaiser-Bessel-derived windows "does not alter the detection performance in
//! a significant way" (§1.2), and sine is what common AAC encoders emit.

/// Coefficients in one long transform.
pub const LONG_LEN: usize = 1024;
/// Coefficients in one short transform.
pub const SHORT_LEN: usize = 128;
/// Short transforms packed into the span of one long transform.
pub const SHORT_COUNT: usize = LONG_LEN / SHORT_LEN;
/// Where the first short transform starts inside the long window.
pub const SHORT_OFFSET: usize = (LONG_LEN - SHORT_LEN) / 2;

/// Scalefactor coding bias for AAC.
///
/// See [`super::criterion::PreparedSubband::prepare`] for what this is and
/// why the search runs in biased units.
pub const SF_BIAS: f64 = 60.0;

use super::Band;

/// Band start offsets for the long transform, 44.1 and 48 kHz.
const LONG_44_48: [usize; 49] = [
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 48, 56, 64, 72, 80, 88, 96, 108, 120, 132, 144, 160,
    176, 196, 216, 240, 264, 292, 320, 352, 384, 416, 448, 480, 512, 544, 576, 608, 640, 672, 704,
    736, 768, 800, 832, 864, 896, 928,
];

/// Band start offsets for the long transform, 32 kHz.
const LONG_32: [usize; 51] = [
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 48, 56, 64, 72, 80, 88, 96, 108, 120, 132, 144, 160,
    176, 196, 216, 240, 264, 292, 320, 352, 384, 416, 448, 480, 512, 544, 576, 608, 640, 672, 704,
    736, 768, 800, 832, 864, 896, 928, 960, 992,
];

/// Band start offsets for the short transform. The same for all three rates.
const SHORT_ALL: [usize; 14] = [0, 4, 8, 12, 16, 20, 28, 36, 44, 56, 68, 80, 96, 112];

fn bands_from_starts(starts: &[usize], total: usize) -> Vec<Band> {
    starts
        .iter()
        .enumerate()
        .map(|(i, &start)| Band {
            start,
            end: starts.get(i + 1).copied().unwrap_or(total),
        })
        .collect()
}

/// The long-transform bands for `rate_khz`, or `None` for an untabulated
/// rate. Use [`super::supported_rate_khz`] to obtain `rate_khz`.
pub fn long_bands(rate_khz: u32) -> Option<Vec<Band>> {
    let starts: &[usize] = match rate_khz {
        44 | 48 => &LONG_44_48,
        32 => &LONG_32,
        _ => return None,
    };
    Some(bands_from_starts(starts, LONG_LEN))
}

/// The short-transform bands for `rate_khz`.
pub fn short_bands(rate_khz: u32) -> Option<Vec<Band>> {
    match rate_khz {
        32 | 44 | 48 => Some(bands_from_starts(&SHORT_ALL, SHORT_LEN)),
        _ => None,
    }
}

/// A symmetric sine window of `len` samples: `w[n] = sin(π(n + ½)/len)`.
///
/// This is the window MPEG defines for both transform sizes; the four shapes
/// below are all assembled from two of these.
fn sine_window(len: usize) -> Vec<f64> {
    (0..len)
        .map(|n| (std::f64::consts::PI * (n as f64 + 0.5) / len as f64).sin())
        .collect()
}

/// The four window shapes AAC can apply to one long frame.
///
/// `start` and `stop` are the transition shapes: an encoder switching from
/// long to short transforms cannot do it abruptly without breaking the
/// transform's perfect-reconstruction property, so it inserts a window that
/// is long on one side and short on the other. Both must be tested, because
/// a frame the encoder used as a transition carries its lattice under that
/// shape and under no other.
pub struct Windows {
    /// Plain long window.
    pub long: Vec<f64>,
    /// Long rising edge, short falling edge.
    pub start: Vec<f64>,
    /// Short rising edge, long falling edge.
    pub stop: Vec<f64>,
    /// The window applied to each of the eight short transforms.
    pub short: Vec<f64>,
}

impl Windows {
    /// Build the four shapes. Deterministic and cheap; built once per
    /// analysis.
    pub fn new() -> Self {
        let long_full = sine_window(2 * LONG_LEN);
        let short_full = sine_window(2 * SHORT_LEN);
        let (long_rise, long_fall) = long_full.split_at(LONG_LEN);
        let (short_rise, short_fall) = short_full.split_at(SHORT_LEN);

        // start: long rise, flat top, short fall, then silence. The flat and
        // zero runs are what let a short transform sit inside a long frame
        // without the two overlapping incorrectly.
        let mut start = Vec::with_capacity(2 * LONG_LEN);
        start.extend_from_slice(long_rise);
        start.extend(std::iter::repeat_n(1.0, SHORT_OFFSET));
        start.extend_from_slice(short_fall);
        start.extend(std::iter::repeat_n(0.0, SHORT_OFFSET));

        // stop: the mirror image.
        let mut stop = Vec::with_capacity(2 * LONG_LEN);
        stop.extend(std::iter::repeat_n(0.0, SHORT_OFFSET));
        stop.extend_from_slice(short_rise);
        stop.extend(std::iter::repeat_n(1.0, SHORT_OFFSET));
        stop.extend_from_slice(long_fall);

        Self {
            long: long_full.clone(),
            start,
            stop,
            short: short_full,
        }
    }
}

impl Default for Windows {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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
        assert!(w.start[0] > 0.0 && w.start[0] < 1e-2, "long rise starts near 0");
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
}
