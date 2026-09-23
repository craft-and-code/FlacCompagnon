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
#[path = "../../tests/unit/transcode/aac_tables.rs"]
mod tests;
