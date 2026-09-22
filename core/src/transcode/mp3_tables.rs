//! MPEG-1 Layer III constants: the polyphase analysis window, the scalefactor
//! band layouts, and the aliasing-reduction coefficients.
//!
//! All three are **from the standard**, ISO/IEC 11172-3. The analysis window
//! is Table C.1 (the 512 coefficients `Ci` of the polyphase filterbank); the
//! band layouts are the Layer III scalefactor band tables; the butterfly
//! coefficients are the eight `ci` of the alias-reduction stage. They are
//! reproduced as published — nobody's invention, and no authorship attaches
//! to them.
//!
//! # Why MP3 needs all this and AAC needed almost none of it
//!
//! AAC transforms the signal once. MP3 transforms it three times over: a
//! 32-band polyphase filter, an 18-point MDCT inside each of those bands, then
//! a butterfly stage that partially undoes the aliasing the first two
//! introduced. The quantization lattice this detector looks for only exists at
//! the *end* of that chain, so every stage has to be reproduced exactly —
//! including the alias reduction, which is not optional cosmetics but part of
//! what defines where the coefficients land.
//!
//! This file is one long table by design; see the module ceiling exemption in
//! CLAUDE.md.

use super::Band;

/// Subbands the polyphase filterbank splits the signal into.
pub const PQMF_BANDS: usize = 32;
/// Samples the analysis window spans.
pub const PQMF_WINDOW_LEN: usize = 512;
/// MDCT coefficients per subband, per granule.
pub const GRANULE_SIZE: usize = 18;
/// Samples in one granule: 32 subbands x 18 = 576.
pub const GRANULE_LEN: usize = PQMF_BANDS * GRANULE_SIZE;
/// Short transforms packed into one granule's span.
pub const SHORT_COUNT: usize = 3;
/// Long MDCT length, in subband samples.
pub const N_LONG: usize = 36;
/// Short MDCT length, in subband samples.
pub const N_SHORT: usize = 12;
/// Where the first short transform starts inside a granule.
pub const SHORT_OFFSET: usize = 12;

/// Scalefactor coding bias for MP3.
///
/// 80, where AAC uses 60 — the two standards bias their transmitted
/// scalefactors differently, and the search runs in biased units. See
/// [`crate::transcode::criterion::PreparedSubband::prepare`].
pub const SF_BIAS: f64 = 80.0;

/// Alias-reduction butterfly coefficients `ci`, ISO/IEC 11172-3.
const CI: [f64; 8] = [-0.6, -0.535, -0.33, -0.185, -0.095, -0.041, -0.0142, -0.0037];

/// The polyphase analysis window, ISO/IEC 11172-3 Table C.1.
///
/// Verified symmetric in magnitude about its centre (`|C[i]| == |C[512-i]|`
/// exactly, for i in 1..256) — the property the standard's table has and a
/// mistyped one would not.
pub const PQMF_WINDOW: [f64; PQMF_WINDOW_LEN] = [
    0.0, -0.000000477, -0.000000477, -0.000000477,
    -0.000000477, -0.000000477, -0.000000477, -0.000000954,
    -0.000000954, -0.000000954, -0.000000954, -0.000001431,
    -0.000001431, -0.000001907, -0.000001907, -0.000002384,
    -0.000002384, -0.000002861, -0.000003338, -0.000003338,
    -0.000003815, -0.000004292, -0.000004768, -0.000005245,
    -0.000006199, -0.000006676, -0.000007629, -0.000008106,
    -0.00000906, -0.000010014, -0.000011444, -0.000012398,
    -0.000013828, -0.000014782, -0.000016689, -0.00001812,
    -0.00001955, -0.000021458, -0.000023365, -0.000025272,
    -0.000027657, -0.000030041, -0.000032425, -0.000034809,
    -0.00003767, -0.000040531, -0.000043392, -0.000046253,
    -0.000049591, -0.000052929, -0.00005579, -0.000059605,
    -0.000062943, -0.00006628, -0.000070095, -0.000073433,
    -0.000076771, -0.000080585, -0.000083923, -0.000087261,
    -0.000090599, -0.00009346, -0.000096321, -0.000099182,
    0.000101566, 0.000103951, 0.000105858, 0.000107288,
    0.000108242, 0.000108719, 0.000108719, 0.000108242,
    0.000106812, 0.000105381, 0.00010252, 0.000099182,
    0.000095367, 0.000090122, 0.0000844, 0.000077724,
    0.000069618, 0.000060558, 0.000050545, 0.000039577,
    0.00002718, 0.000013828, -0.000000954, -0.000017166,
    -0.000034332, -0.000052929, -0.000072956, -0.000093937,
    -0.000116348, -0.00014019, -0.000165462, -0.000191212,
    -0.000218868, -0.000247478, -0.000277042, -0.00030756,
    -0.000339031, -0.000371456, -0.000404358, -0.000438213,
    -0.000472546, -0.000507355, -0.000542164, -0.000576973,
    -0.000611782, -0.000646591, -0.000680923, -0.000714302,
    -0.000747204, -0.000779152, -0.000809669, -0.000838757,
    -0.000866413, -0.000891685, -0.000915051, -0.000935555,
    -0.000954151, -0.000968933, -0.000980854, -0.000989437,
    -0.000994205, -0.000995159, -0.000991821, -0.000983715,
    0.000971317, 0.000953674, 0.000930786, 0.000902653,
    0.000868797, 0.00082922, 0.00078392, 0.000731945,
    0.000674248, 0.000610352, 0.000539303, 0.000462532,
    0.000378609, 0.000288486, 0.000191689, 0.000088215,
    -0.000021458, -0.000137329, -0.000259876, -0.000388145,
    -0.000522137, -0.00066185, -0.000806808, -0.000956535,
    -0.001111031, -0.001269817, -0.001432419, -0.001597881,
    -0.001766682, -0.001937389, -0.002110004, -0.002283096,
    -0.002457142, -0.002630711, -0.002803326, -0.002974033,
    -0.00314188, -0.003306866, -0.003467083, -0.003622532,
    -0.003771782, -0.003914356, -0.004048824, -0.004174709,
    -0.004290581, -0.004395962, -0.004489899, -0.004570484,
    -0.004638195, -0.004691124, -0.004728317, -0.004748821,
    -0.004752159, -0.004737377, -0.004703045, -0.004649162,
    -0.004573822, -0.004477024, -0.004357815, -0.00421524,
    -0.004049301, -0.003858566, -0.003643036, -0.003401756,
    0.003134727, 0.002841473, 0.002521515, 0.002174854,
    0.001800537, 0.001399517, 0.000971317, 0.000515938,
    0.000033379, -0.000475883, -0.001011848, -0.001573563,
    -0.002161503, -0.002774239, -0.003411293, -0.004072189,
    -0.004756451, -0.00546217, -0.006189346, -0.006937027,
    -0.007703304, -0.008487225, -0.009287834, -0.010103703,
    -0.010933399, -0.011775017, -0.012627602, -0.013489246,
    -0.014358521, -0.015233517, -0.016112804, -0.016994476,
    -0.017876148, -0.018756866, -0.019634247, -0.020506859,
    -0.021372318, -0.022228718, -0.02307415, -0.023907185,
    -0.024725437, -0.025527, -0.026310921, -0.02707386,
    -0.027815342, -0.028532982, -0.029224873, -0.02989006,
    -0.030526638, -0.031132698, -0.03170681, -0.03224802,
    -0.032754898, -0.033225536, -0.033659935, -0.03405571,
    -0.034412861, -0.034730434, -0.035007, -0.035242081,
    -0.0354352, -0.035586357, -0.035694122, -0.035758972,
    0.035780907, 0.035758972, 0.035694122, 0.035586357,
    0.0354352, 0.035242081, 0.035007, 0.034730434,
    0.034412861, 0.03405571, 0.033659935, 0.033225536,
    0.032754898, 0.03224802, 0.03170681, 0.031132698,
    0.030526638, 0.02989006, 0.029224873, 0.028532982,
    0.027815342, 0.02707386, 0.026310921, 0.025527,
    0.024725437, 0.023907185, 0.02307415, 0.022228718,
    0.021372318, 0.020506859, 0.019634247, 0.018756866,
    0.017876148, 0.016994476, 0.016112804, 0.015233517,
    0.014358521, 0.013489246, 0.012627602, 0.011775017,
    0.010933399, 0.010103703, 0.009287834, 0.008487225,
    0.007703304, 0.006937027, 0.006189346, 0.00546217,
    0.004756451, 0.004072189, 0.003411293, 0.002774239,
    0.002161503, 0.001573563, 0.001011848, 0.000475883,
    -0.000033379, -0.000515938, -0.000971317, -0.001399517,
    -0.001800537, -0.002174854, -0.002521515, -0.002841473,
    0.003134727, 0.003401756, 0.003643036, 0.003858566,
    0.004049301, 0.00421524, 0.004357815, 0.004477024,
    0.004573822, 0.004649162, 0.004703045, 0.004737377,
    0.004752159, 0.004748821, 0.004728317, 0.004691124,
    0.004638195, 0.004570484, 0.004489899, 0.004395962,
    0.004290581, 0.004174709, 0.004048824, 0.003914356,
    0.003771782, 0.003622532, 0.003467083, 0.003306866,
    0.00314188, 0.002974033, 0.002803326, 0.002630711,
    0.002457142, 0.002283096, 0.002110004, 0.001937389,
    0.001766682, 0.001597881, 0.001432419, 0.001269817,
    0.001111031, 0.000956535, 0.000806808, 0.00066185,
    0.000522137, 0.000388145, 0.000259876, 0.000137329,
    0.000021458, -0.000088215, -0.000191689, -0.000288486,
    -0.000378609, -0.000462532, -0.000539303, -0.000610352,
    -0.000674248, -0.000731945, -0.00078392, -0.00082922,
    -0.000868797, -0.000902653, -0.000930786, -0.000953674,
    0.000971317, 0.000983715, 0.000991821, 0.000995159,
    0.000994205, 0.000989437, 0.000980854, 0.000968933,
    0.000954151, 0.000935555, 0.000915051, 0.000891685,
    0.000866413, 0.000838757, 0.000809669, 0.000779152,
    0.000747204, 0.000714302, 0.000680923, 0.000646591,
    0.000611782, 0.000576973, 0.000542164, 0.000507355,
    0.000472546, 0.000438213, 0.000404358, 0.000371456,
    0.000339031, 0.00030756, 0.000277042, 0.000247478,
    0.000218868, 0.000191212, 0.000165462, 0.00014019,
    0.000116348, 0.000093937, 0.000072956, 0.000052929,
    0.000034332, 0.000017166, 0.000000954, -0.000013828,
    -0.00002718, -0.000039577, -0.000050545, -0.000060558,
    -0.000069618, -0.000077724, -0.0000844, -0.000090122,
    -0.000095367, -0.000099182, -0.00010252, -0.000105381,
    -0.000106812, -0.000108242, -0.000108719, -0.000108719,
    -0.000108242, -0.000107288, -0.000105858, -0.000103951,
    0.000101566, 0.000099182, 0.000096321, 0.00009346,
    0.000090599, 0.000087261, 0.000083923, 0.000080585,
    0.000076771, 0.000073433, 0.000070095, 0.00006628,
    0.000062943, 0.000059605, 0.00005579, 0.000052929,
    0.000049591, 0.000046253, 0.000043392, 0.000040531,
    0.00003767, 0.000034809, 0.000032425, 0.000030041,
    0.000027657, 0.000025272, 0.000023365, 0.000021458,
    0.00001955, 0.00001812, 0.000016689, 0.000014782,
    0.000013828, 0.000012398, 0.000011444, 0.000010014,
    0.00000906, 0.000008106, 0.000007629, 0.000006676,
    0.000006199, 0.000005245, 0.000004768, 0.000004292,
    0.000003815, 0.000003338, 0.000003338, 0.000002861,
    0.000002384, 0.000002384, 0.000001907, 0.000001907,
    0.000001431, 0.000001431, 0.000000954, 0.000000954,
    0.000000954, 0.000000954, 0.000000477, 0.000000477,
    0.000000477, 0.000000477, 0.000000477, 0.000000477,
];

const LONG_32: [usize; 21] = [
    0, 4, 8, 12, 16, 20, 24, 30, 36, 44, 54, 66, 82, 102, 126, 156, 194, 240, 296, 364, 448,
];
const SHORT_32: [usize; 12] = [
    0, 4, 8, 12, 16, 22, 30, 42, 58, 78, 104, 138,
];
const LONG_44: [usize; 21] = [
    0, 4, 8, 12, 16, 20, 24, 30, 36, 44, 52, 62, 74, 90, 110, 134, 162, 196, 238, 288, 342,
];
const SHORT_44: [usize; 12] = [
    0, 4, 8, 12, 16, 22, 30, 40, 52, 66, 84, 106,
];
const LONG_48: [usize; 21] = [
    0, 4, 8, 12, 16, 20, 24, 30, 36, 42, 50, 60, 72, 88, 106, 128, 156, 190, 230, 276, 330,
];
const SHORT_48: [usize; 12] = [
    0, 4, 8, 12, 16, 22, 28, 38, 50, 64, 80, 100,
];

const LONG_END_32: usize = 550;
const SHORT_END_32: usize = 180;
const LONG_END_44: usize = 418;
const SHORT_END_44: usize = 136;
const LONG_END_48: usize = 384;
const SHORT_END_48: usize = 126;

/// Precomputed alias-reduction factors `cs[i]` and `ca[i]`.
///
/// `cs = 1/sqrt(1 + ci^2)`, `ca = ci/sqrt(1 + ci^2)` — derived rather than
/// tabulated, so the two can never drift apart from the `ci` above.
pub fn butterfly_coefficients() -> ([f64; 8], [f64; 8]) {
    let mut cs = [0.0; 8];
    let mut ca = [0.0; 8];
    for (i, &c) in CI.iter().enumerate() {
        let d = (1.0 + c * c).sqrt();
        cs[i] = 1.0 / d;
        ca[i] = c / d;
    }
    (cs, ca)
}

/// The four window shapes Layer III applies to a subband's 36 samples.
///
/// Same idea as AAC's, at a twelfth of the size and one stage further down
/// the chain: these are applied inside each of the 32 subbands, not to the
/// PCM. `start` and `stop` are the transition shapes bridging long and short
/// blocks; both must be tested, because a frame the encoder coded as a
/// transition carries its lattice under that shape and no other.
pub struct Windows {
    /// Plain long window, 36 samples.
    pub long: Vec<f64>,
    /// Long rising edge, short falling edge.
    pub start: Vec<f64>,
    /// Short rising edge, long falling edge.
    pub stop: Vec<f64>,
    /// Applied to each of the three short transforms, 12 samples.
    pub short: Vec<f64>,
}

impl Windows {
    /// Build the four shapes. Cheap; built once per worker.
    pub fn new() -> Self {
        let sine = |len: usize, from: usize, count: usize| -> Vec<f64> {
            (from..from + count)
                .map(|n| (std::f64::consts::PI * (n as f64 + 0.5) / len as f64).sin())
                .collect()
        };
        const FILL: usize = 6;
        let half_long = N_LONG / 2;
        let half_short = N_SHORT / 2;

        let mut start = sine(N_LONG, 0, half_long);
        start.extend(std::iter::repeat_n(1.0, FILL));
        start.extend(sine(N_SHORT, FILL, half_short));
        start.extend(std::iter::repeat_n(0.0, FILL));

        let mut stop = vec![0.0; FILL];
        stop.extend(sine(N_SHORT, 0, half_short));
        stop.extend(std::iter::repeat_n(1.0, FILL));
        stop.extend(sine(N_LONG, half_long, half_long));

        Self {
            long: sine(N_LONG, 0, N_LONG),
            start,
            stop,
            short: sine(N_SHORT, 0, N_SHORT),
        }
    }
}


impl Default for Windows {
    fn default() -> Self {
        Self::new()
    }
}

/// Long-transform scalefactor bands for `rate_khz`.
///
/// Unlike AAC's, these do **not** tile the whole spectrum: the topmost band
/// stops at 418 of 576 coefficients at 44.1 kHz (384 at 48, 550 at 32). Above
/// that is the region a Layer III encoder discards outright, so there is no
/// lattice up there to find and the standard does not assign it a scalefactor.
pub fn long_bands(rate_khz: u32) -> Option<Vec<Band>> {
    let (starts, end): (&[usize], usize) = match rate_khz {
        32 => (&LONG_32, LONG_END_32),
        44 => (&LONG_44, LONG_END_44),
        48 => (&LONG_48, LONG_END_48),
        _ => return None,
    };
    Some(bands(starts, end))
}

/// Short-transform scalefactor bands for `rate_khz`.
pub fn short_bands(rate_khz: u32) -> Option<Vec<Band>> {
    let (starts, end): (&[usize], usize) = match rate_khz {
        32 => (&SHORT_32, SHORT_END_32),
        44 => (&SHORT_44, SHORT_END_44),
        48 => (&SHORT_48, SHORT_END_48),
        _ => return None,
    };
    Some(bands(starts, end))
}

fn bands(starts: &[usize], end: usize) -> Vec<Band> {
    starts
        .iter()
        .enumerate()
        .map(|(i, &start)| Band {
            start,
            end: starts.get(i + 1).copied().unwrap_or(end),
        })
        .collect()
}

#[cfg(test)]
mod tests {
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
                (short_bands(rate).expect("short"), GRANULE_LEN / SHORT_COUNT, "short"),
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
                assert!(last < limit, "{rate}k {what}: {last} should stop below {limit}");
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
}
