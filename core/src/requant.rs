//! AAC re-quantization transcode detector, following O. Derrien, *"Detection
//! of Genuine Lossless Audio Files: Application to the MPEG-AAC Codec"*,
//! J. Audio Eng. Soc. 67(3), 2019 — the method behind Lossless Audio Checker.
//!
//! An AAC encoder quantizes MDCT coefficients per scale-factor band on the grid
//! `|X| = n^(4/3) · Δ` (n integer, Δ set by the band's scalefactor). In the
//! `|X|^(3/4)` domain this is a *linear* grid. The decode to PCM is an IMDCT
//! with overlap-add; thanks to TDAC, re-analyzing the decoded signal with the
//! **same window at the exact frame alignment** recovers those quantized
//! coefficients. For genuine (never-AAC-encoded) audio no alignment does.
//!
//! Per the paper, the detector:
//! 1. sweeps all 1024 possible frame onsets at one-sample resolution;
//! 2. tests both AAC window shapes (sine, KBD α=4), all four channel
//!    representations (L, R, M, S — the encoder may have used MS stereo), and
//!    **both block sizes**: the long 1024-coefficient MDCT and the 8 short
//!    128-coefficient sub-blocks of an EIGHT_SHORT_SEQUENCE frame (encoders
//!    switch to short blocks on transients; those frames are invisible to a
//!    long-window analysis);
//! 3. sweeps, per band, [`NSF`] candidate scalefactors across the plausible
//!    range `δ ∈ [0.3, 0.7]` of the dead-zone bound `φdz = 16/3 + 4·log2(max|X|)`
//!    (eq. 2; 90% of real encoder scalefactors land in that window), computes
//!    the rounding-error energy `E(s) = Σ ε²` and flags the band when
//!    `E < τ(s)` — the statistical threshold from the Gaussian model of
//!    uniform quantization noise (eq. 8, P = 0.005). A guard requires ≥ 4 (or
//!    K/4) non-zero quantization indices so near-empty bands can't pass by
//!    chance. A scale-free fallback estimator (grid step from the band minimum,
//!    quantized 1..3) additionally catches coarse grids the sweep may straddle;
//! 4. counts each band at most once (binary): the per-frame likelihood is
//!    `hit bands / scored bands`, maximized over channels;
//! 5. refines the best onsets over the 16 highest-energy frames and scores the
//!    file by the **3rd-highest** per-frame likelihood — a transcode stays
//!    on-grid at the same onset in every frame, while genuine flukes almost
//!    never repeat three times at one onset.
//!
//! Calibrated on ffmpeg AAC transcodes at 128/192/256/320 kbps through a
//! 16-bit chain vs. genuine originals: transcodes score 0.28–1.0, genuine
//! ≤ 0.23 (pathological sparse synthetics; realistic material ≤ 0.15). The
//! detection threshold [`DETECT_RATE`] = 0.25 gave **zero false positives and
//! 24/24 recall** — the short-block analysis is what recovers the very
//! bright/transient content at ≥ 192 kbps that long-window analysis alone
//! misses (measured: 0.13 → 0.82 on such a file at 320 kbps).
//! Only applies at 44.1/48 kHz, per the paper.

use std::cell::RefCell;
use std::sync::Arc;

use rustfft::{num_complex::Complex, Fft, FftPlanner};

/// MDCT half-length (AAC long block).
pub const N: usize = 1024;
/// MDCT input frame length.
pub const L: usize = 2 * N;

/// Scale-factor band offsets for AAC long windows at 44.1/48 kHz.
pub const SWB_4448: [usize; 50] = [
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 48, 56, 64, 72, 80, 88, 96, 108, 120, 132, 144, 160,
    176, 196, 216, 240, 264, 292, 320, 352, 384, 416, 448, 480, 512, 544, 576, 608, 640, 672, 704,
    736, 768, 800, 832, 864, 896, 928, 1024,
];

// --- Short blocks (EIGHT_SHORT_SEQUENCE) -------------------------------------
// On transients the AAC encoder switches to 8 short sub-blocks of 256 samples
// (128 MDCT coefficients) per frame, starting at frame_onset + 448 + j·128.
// Analyzing them with the matching 256-point MDCT is what makes transient-heavy
// transcodes detectable — their long-window analysis reads only garbage.
/// Short-block MDCT half-length.
const NS: usize = 128;
/// Short-block MDCT input length.
const LSHORT: usize = 2 * NS;
/// Offset of the first short sub-block inside the 2048-sample frame.
const SUB0: usize = 448;
/// Scale-factor band offsets for AAC short windows at 44.1/48 kHz.
const SWB_SHORT: [usize; 15] = [0, 4, 8, 12, 16, 20, 28, 36, 44, 56, 68, 80, 96, 112, 128];
/// Short bands analyzed (skips the five K=4 micro-bands, like the long path).
const SBAND_LO: usize = 5;
const SBAND_HI: usize = 14;
/// τ(s) for the analyzed short bands (same eq. 8, P = 0.005; K = 8/12/16).
const TAU_SHORT: [f64; 9] = [
    1.343215761580e-01, 1.343215761580e-01, 1.343215761580e-01,
    3.358790501266e-01, 3.358790501266e-01, 3.358790501266e-01,
    5.654492211732e-01, 5.654492211732e-01, 5.654492211732e-01,
];

/// Band range analyzed: skips the ten K=4 micro-bands (4 coefficients carry no
/// statistical weight — they pass `E < τ` by chance) and runs to the last band.
const BAND_LO: usize = 10;
const BAND_HI: usize = 49;

/// Statistical thresholds τ(s) for bands [`BAND_LO`]..[`BAND_HI`], from the
/// paper's eq. (8) with P = 0.005: the probability that a *genuine* band's
/// rounding-error energy dips below τ. Precomputed (they depend only on the
/// fixed band widths K of the 44.1/48 kHz long-window scale-factor table).
const TAU: [f64; 39] = [
    1.343215761580e-01, 1.343215761580e-01, 1.343215761580e-01, 1.343215761580e-01,
    1.343215761580e-01, 1.343215761580e-01, 1.343215761580e-01, 3.358790501266e-01,
    3.358790501266e-01, 3.358790501266e-01, 3.358790501266e-01, 5.654492211732e-01,
    5.654492211732e-01, 8.080635066858e-01, 8.080635066858e-01, 1.059440669626e+00,
    1.059440669626e+00, 1.317412600469e+00, 1.317412600469e+00, 1.580601675300e+00,
    1.580601675300e+00, 1.580601675300e+00, 1.580601675300e+00, 1.580601675300e+00,
    1.580601675300e+00, 1.580601675300e+00, 1.580601675300e+00, 1.580601675300e+00,
    1.580601675300e+00, 1.580601675300e+00, 1.580601675300e+00, 1.580601675300e+00,
    1.580601675300e+00, 1.580601675300e+00, 1.580601675300e+00, 1.580601675300e+00,
    1.580601675300e+00, 1.580601675300e+00, 6.118880248218e+00,
];

/// Scalefactor sweep: candidate count and relative bounds (fraction of the
/// dead-zone bound). Per the paper, 90% of encoder scalefactors fall in
/// δ ∈ [0.3, 0.7] of φdz.
const NSF: usize = 16;
const DELTA_MIN: f64 = 0.3;
const DELTA_MAX: f64 = 0.7;
/// PCM is scaled to the codec-native 16-bit integer domain before analysis so
/// the δ window matches the scale it was calibrated in.
const PCM_SCALE: f64 = 32768.0;

/// Fallback ymin-estimator: coefficients below this (scaled) magnitude are
/// ignored, a band needs ≥ [`MIN_COEFS`] survivors, and hits when the mean
/// distance-to-integer is below [`HIT_RESIDUAL`].
const COEF_FLOOR: f64 = 32.0; // ≈ 1e-3 in the [-1, 1] float domain
const HIT_RESIDUAL: f64 = 0.05;
const MIN_COEFS: usize = 6;

/// Likelihood at/above which the file is flagged as AAC-transcoded (λ).
/// Calibrated for zero false positives on the corpus described above.
pub const DETECT_RATE: f32 = 0.25;

/// Samples per channel the detector needs (onset sweep + refine frames).
pub const SEGMENT_LEN: usize = 1023 + L + REFINE_FRAMES * N;
/// Frames available in a segment; the refine step scores the
/// [`REFINE_TOP`] highest-energy ones.
const REFINE_FRAMES: usize = 30;
const REFINE_TOP: usize = 16;
/// Frame indices used by the coarse onset sweep (spread out so at least some
/// avoid the encoder's short-window transient frames).
const COARSE_FRAMES: [usize; 6] = [3, 5, 8, 13, 20, 27];
/// Coarse candidates retained per window shape.
const TOP_CANDIDATES: usize = 4;

/// Modified zeroth-order Bessel function of the first kind (series expansion).
fn bessel_i0(x: f64) -> f64 {
    let q = x * x / 4.0;
    let mut term = 1.0f64;
    let mut sum = 1.0f64;
    for k in 1..64 {
        term *= q / ((k * k) as f64);
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}

/// AAC sine window of length `l` (2× the MDCT half-length).
fn sine_window_of(l: usize) -> Vec<f64> {
    (0..l)
        .map(|n| (std::f64::consts::PI / l as f64 * (n as f64 + 0.5)).sin())
        .collect()
}

/// AAC Kaiser–Bessel-derived window (α = 4) of length `2·half`.
fn kbd_window_of(half: usize) -> Vec<f64> {
    let a = 4.0 * std::f64::consts::PI;
    let m = half as f64;
    let kernel: Vec<f64> = (0..=half)
        .map(|j| {
            let t = (j as f64 - m / 2.0) / (m / 2.0);
            bessel_i0(a * (1.0 - t * t).max(0.0).sqrt())
        })
        .collect();
    let mut cum = Vec::with_capacity(half + 1);
    let mut acc = 0.0;
    for w in &kernel {
        acc += w;
        cum.push(acc);
    }
    let total = cum[half];
    let first: Vec<f64> = (0..half).map(|j| (cum[j] / total).sqrt()).collect();
    let mut win = first.clone();
    win.extend(first.iter().rev());
    win
}

/// AAC sine window, length 2N (long blocks).
pub fn sine_window() -> Vec<f64> {
    sine_window_of(L)
}

/// AAC Kaiser–Bessel-derived window (α = 4), length 2N (long blocks).
pub fn kbd_window() -> Vec<f64> {
    kbd_window_of(N)
}

/// FFT-based forward MDCT (one 2N-point complex FFT per frame).
///
/// Holds a reusable scratch buffer (`RefCell`), so a single instance performs
/// thousands of transforms without per-call allocation. Not `Sync`; use one
/// instance per thread.
pub struct Mdct {
    half: usize,
    len: usize,
    fft: Arc<dyn Fft<f64>>,
    pre: Vec<Complex<f64>>,
    post: Vec<Complex<f64>>,
    scratch: RefCell<Vec<Complex<f64>>>,
}

impl Mdct {
    /// Long-block MDCT (N = 1024, frame 2048).
    pub fn new() -> Self {
        Self::with_size(N)
    }

    /// Short-block MDCT (N = 128, frame 256) for EIGHT_SHORT_SEQUENCE frames.
    pub fn short() -> Self {
        Self::with_size(NS)
    }

    fn with_size(half: usize) -> Self {
        let len = 2 * half;
        let fft = FftPlanner::<f64>::new().plan_fft_forward(len);
        let pre: Vec<Complex<f64>> = (0..len)
            .map(|n| Complex::from_polar(1.0, -std::f64::consts::PI * n as f64 / len as f64))
            .collect();
        let n0 = half as f64 / 2.0 + 0.5;
        let post: Vec<Complex<f64>> = (0..half)
            .map(|k| {
                Complex::from_polar(
                    1.0,
                    -std::f64::consts::PI * n0 * (k as f64 + 0.5) / half as f64,
                )
            })
            .collect();
        Self {
            half,
            len,
            fft,
            pre,
            post,
            scratch: RefCell::new(Vec::with_capacity(len)),
        }
    }

    /// Transform `frame` (length 2·half) windowed by `win` into `out`
    /// (length half).
    pub fn forward(&self, frame: &[f64], win: &[f64], out: &mut [f64]) {
        debug_assert_eq!(frame.len(), self.len);
        let mut buf = self.scratch.borrow_mut();
        buf.clear();
        buf.extend((0..self.len).map(|n| self.pre[n] * (frame[n] * win[n])));
        self.fft.process(&mut buf);
        for k in 0..self.half {
            out[k] = (self.post[k] * buf[k]).re;
        }
    }
}

impl Default for Mdct {
    fn default() -> Self {
        Self::new()
    }
}

/// Fallback: best mean distance-to-integer for a band's `|coef|^(3/4)` values,
/// with the grid step estimated from the smallest value (quantized 1, 2 or 3).
/// Scale-free and razor-sharp on coarse grids that the scalefactor sweep's
/// finite granularity can straddle.
fn ymin_hit(y: &[f64]) -> bool {
    if y.len() < MIN_COEFS {
        return false;
    }
    let ymin = y.iter().cloned().fold(f64::INFINITY, f64::min);
    if !(ymin > 0.0) {
        return false;
    }
    let mut best = 1.0f64;
    for div in [1.0, 2.0, 3.0] {
        let s = ymin / div;
        let mut acc = 0.0;
        for &v in y {
            let r = v / s;
            acc += (r - r.round()).abs();
        }
        best = best.min(acc / y.len() as f64);
    }
    best < HIT_RESIDUAL
}

/// Whether one band of one coefficient row is on the AAC grid — the paper's
/// statistical criterion (scalefactor sweep + E < τ) or the ymin fallback.
/// `abs` holds |X| in the PCM-scaled domain; `y34` the matching |X|^(3/4).
fn band_hit(abs: &[f64], y34: &[f64], tau: f64, ybuf: &mut Vec<f64>) -> Option<bool> {
    let k = abs.len();
    let mx = abs.iter().cloned().fold(0.0f64, f64::max);
    if mx <= 0.0 {
        return None; // empty band: not scored
    }
    // --- paper criterion: sweep scalefactors across the dead-zone window ---
    let phidz = 16.0 / 3.0 + 4.0 * mx.log2();
    let nnz_min = (k / 4).max(4);
    for i in 0..NSF {
        let delta = DELTA_MIN + (DELTA_MAX - DELTA_MIN) * i as f64 / (NSF - 1) as f64;
        let scale = (2.0f64).powf(-3.0 * delta * phidz / 16.0);
        let mut e = 0.0f64;
        let mut nnz = 0usize;
        for &y in y34 {
            let ysc = y * scale;
            let q = ysc.round();
            let eps = q - ysc;
            e += eps * eps;
            if q != 0.0 {
                nnz += 1;
            }
        }
        if e < tau && nnz >= nnz_min {
            return Some(true);
        }
    }
    // --- fallback: step estimated from the band minimum ---
    ybuf.clear();
    for &a in abs {
        if a > COEF_FLOOR {
            ybuf.push(a.powf(0.75));
        }
    }
    Some(ymin_hit(ybuf))
}

/// Per-frame likelihood at one onset: for each channel representation, the
/// fraction of scored bands that are on-grid; the maximum over channels.
fn frame_likelihood(
    mdct: &Mdct,
    win: &[f64],
    chans: &[&[f64]],
    start: usize,
    coefs: &mut [Vec<f64>],
    ybuf: &mut Vec<f64>,
) -> f64 {
    let mut best = 0.0f64;
    for (ci, ch) in chans.iter().enumerate() {
        mdct.forward(&ch[start..start + L], win, &mut coefs[ci]);
    }
    let mut abs_band = [0.0f64; 96];
    let mut y34_band = [0.0f64; 96];
    for c in coefs.iter() {
        let mut hits = 0usize;
        let mut scored = 0usize;
        for b in BAND_LO..BAND_HI {
            let (lo, hi) = (SWB_4448[b], SWB_4448[b + 1]);
            let k = hi - lo;
            for (j, &v) in c[lo..hi].iter().enumerate() {
                let a = v.abs() * PCM_SCALE;
                abs_band[j] = a;
                y34_band[j] = a.powf(0.75);
            }
            if let Some(hit) = band_hit(&abs_band[..k], &y34_band[..k], TAU[b - BAND_LO], ybuf) {
                scored += 1;
                if hit {
                    hits += 1;
                }
            }
        }
        if scored > 0 {
            best = best.max(hits as f64 / scored as f64);
        }
    }
    best
}

/// Per-frame likelihood under the *short-block* hypothesis: the 8 sub-blocks
/// of an EIGHT_SHORT_SEQUENCE frame are analyzed with the 256-point MDCT and
/// their (sub-block, band) cells aggregated into one fraction per channel;
/// the maximum over channels is returned. Aggregating all 8×9 = 72 cells keeps
/// the small short bands (K = 8..16) statistically meaningful.
fn short_frame_likelihood(
    mdct_s: &Mdct,
    win_s: &[f64],
    chans: &[&[f64]],
    start: usize,
    coefs_s: &mut [f64],
    ybuf: &mut Vec<f64>,
) -> f64 {
    let mut best = 0.0f64;
    let mut abs_band = [0.0f64; 16];
    let mut y34_band = [0.0f64; 16];
    for ch in chans.iter() {
        let mut hits = 0usize;
        let mut scored = 0usize;
        for j in 0..8 {
            let st = start + SUB0 + j * NS;
            mdct_s.forward(&ch[st..st + LSHORT], win_s, coefs_s);
            for b in SBAND_LO..SBAND_HI {
                let (lo, hi) = (SWB_SHORT[b], SWB_SHORT[b + 1]);
                let k = hi - lo;
                for (i, &v) in coefs_s[lo..hi].iter().enumerate() {
                    let a = v.abs() * PCM_SCALE;
                    abs_band[i] = a;
                    y34_band[i] = a.powf(0.75);
                }
                if let Some(hit) =
                    band_hit(&abs_band[..k], &y34_band[..k], TAU_SHORT[b - SBAND_LO], ybuf)
                {
                    scored += 1;
                    if hit {
                        hits += 1;
                    }
                }
            }
        }
        if scored > 0 {
            best = best.max(hits as f64 / scored as f64);
        }
    }
    best
}

/// Result of the re-quantization analysis.
#[derive(Debug, Clone, Copy)]
pub struct RequantResult {
    /// Likelihood of the transcoded case at the best onset/window (0..1):
    /// the 3rd-highest per-frame fraction of on-grid bands.
    pub rate: f32,
    /// Onset (mod 1024) where the grid was found.
    pub onset: usize,
    /// Number of refined frames that backed the likelihood.
    pub tested: usize,
}

/// Run the full detection on one buffered segment.
///
/// `left` / `right` are consecutive samples of the first two channels starting
/// at a sample index that is a multiple of 1024 (so onsets keep their meaning
/// modulo the AAC frame length). Both must be at least [`SEGMENT_LEN`] long
/// (pass `right = None` for mono). Returns `None` when the segment is too
/// short; otherwise the best refined result over both window shapes.
pub fn analyze_segment(left: &[f64], right: Option<&[f64]>) -> Option<RequantResult> {
    if left.len() < SEGMENT_LEN {
        return None;
    }
    // Channel representations: L, R, M, S (encoder may use either matrixing).
    let mut chan_storage: Vec<Vec<f64>> = Vec::new();
    if let Some(r) = right {
        if r.len() < SEGMENT_LEN {
            return None;
        }
        let mid: Vec<f64> = left.iter().zip(r).map(|(a, b)| (a + b) * 0.5).collect();
        let side: Vec<f64> = left.iter().zip(r).map(|(a, b)| (a - b) * 0.5).collect();
        chan_storage.push(left.to_vec());
        chan_storage.push(r.to_vec());
        chan_storage.push(mid);
        chan_storage.push(side);
    } else {
        chan_storage.push(left.to_vec());
    }
    let chans: Vec<&[f64]> = chan_storage.iter().map(|v| v.as_slice()).collect();

    let mdct = Mdct::new();
    let mdct_s = Mdct::short();
    // Window pairs (long, short) of matching shape.
    let windows = [
        (kbd_window(), kbd_window_of(NS)),
        (sine_window(), sine_window_of(LSHORT)),
    ];
    let mut coefs: Vec<Vec<f64>> = (0..chans.len()).map(|_| vec![0.0; N]).collect();
    let mut coefs_s = vec![0.0f64; NS];
    let mut ybuf: Vec<f64> = Vec::with_capacity(96);

    // Refine frames: the REFINE_TOP most energetic of the segment's frames
    // (per the paper — high-energy frames discriminate best). Ranked on the
    // left channel; L and M are equivalent for an energy ranking.
    let mut frame_energy: Vec<(f64, usize)> = (1..=REFINE_FRAMES)
        .map(|m| {
            let st = m * N;
            let e: f64 = chans[0][st..st + L].iter().map(|&v| v * v).sum();
            (e, m)
        })
        .collect();
    frame_energy.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let refine_frames: Vec<usize> = frame_energy
        .iter()
        .take(REFINE_TOP)
        .map(|&(_, m)| m)
        .collect();

    let mut best: Option<RequantResult> = None;
    for (win, win_s) in &windows {
        // Coarse pass: every onset, several spread-out frames; per-onset score
        // is the best single-frame likelihood under either block hypothesis
        // (long 1024-MDCT, or the 8 short 128-MDCT sub-blocks — transient
        // frames are coded short and are invisible to the long analysis).
        let mut per_onset = vec![0.0f64; N];
        for &m in &COARSE_FRAMES {
            for onset in 0..N {
                let start = onset + m * N;
                let l = frame_likelihood(&mdct, win, &chans, start, &mut coefs, &mut ybuf)
                    .max(short_frame_likelihood(&mdct_s, win_s, &chans, start, &mut coefs_s, &mut ybuf));
                if l > per_onset[onset] {
                    per_onset[onset] = l;
                }
            }
        }
        let mut candidates: Vec<(f64, usize)> =
            per_onset.iter().cloned().zip(0..N).collect();
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(TOP_CANDIDATES);

        // Refine each candidate over the high-energy frames; score = the
        // 3rd-highest per-frame likelihood (offset-consistency: a transcode
        // repeats at its onset, genuine flukes don't).
        for &(_, onset) in &candidates {
            let mut scores: Vec<f64> = refine_frames
                .iter()
                .filter(|&&m| onset + m * N + L <= chans[0].len())
                .map(|&m| {
                    let start = onset + m * N;
                    frame_likelihood(&mdct, win, &chans, start, &mut coefs, &mut ybuf)
                        .max(short_frame_likelihood(&mdct_s, win_s, &chans, start, &mut coefs_s, &mut ybuf))
                })
                .collect();
            if scores.len() < 3 {
                continue;
            }
            scores.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
            let rate = scores[2] as f32;
            if best.map_or(true, |b| rate > b.rate) {
                best = Some(RequantResult {
                    rate,
                    onset,
                    tested: scores.len(),
                });
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bessel_matches_reference() {
        // I0(0)=1; I0(1)≈1.2660658; I0(12.566)≈ large — check monotonicity too.
        assert!((bessel_i0(0.0) - 1.0).abs() < 1e-15);
        assert!((bessel_i0(1.0) - 1.2660658777520084).abs() < 1e-12);
        assert!(bessel_i0(4.0 * std::f64::consts::PI) > bessel_i0(10.0));
    }

    #[test]
    fn windows_have_expected_shape() {
        let s = sine_window();
        let k = kbd_window();
        assert_eq!(s.len(), L);
        assert_eq!(k.len(), L);
        // Princen–Bradley: w[n]^2 + w[n+N]^2 == 1 for both AAC windows.
        for n in 0..N {
            assert!((s[n] * s[n] + s[n + N] * s[n + N] - 1.0).abs() < 1e-9);
            assert!((k[n] * k[n] + k[n + N] * k[n + N] - 1.0).abs() < 1e-9);
        }
        // Same property for the short-block windows.
        let ss = sine_window_of(LSHORT);
        let ks = kbd_window_of(NS);
        assert_eq!(ss.len(), LSHORT);
        assert_eq!(ks.len(), LSHORT);
        for n in 0..NS {
            assert!((ss[n] * ss[n] + ss[n + NS] * ss[n + NS] - 1.0).abs() < 1e-9);
            assert!((ks[n] * ks[n] + ks[n + NS] * ks[n + NS] - 1.0).abs() < 1e-9);
        }
    }

    /// The short (256-point) MDCT must match the direct definition, exactly as
    /// the long one does — same twiddle machinery, different size.
    #[test]
    fn short_mdct_matches_direct() {
        let mdct = Mdct::short();
        let win = sine_window_of(LSHORT);
        let mut state = 0xC0FFEEu64;
        let frame: Vec<f64> = (0..LSHORT)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((state >> 33) as f64 / (1u64 << 31) as f64) - 1.0
            })
            .collect();
        let mut fast = vec![0.0; NS];
        mdct.forward(&frame, &win, &mut fast);
        let n0 = NS as f64 / 2.0 + 0.5;
        for &k in &[0usize, 1, 17, 64, 127] {
            let mut acc = 0.0;
            for n in 0..LSHORT {
                acc += frame[n]
                    * win[n]
                    * (std::f64::consts::PI / NS as f64 * (n as f64 + n0) * (k as f64 + 0.5)).cos();
            }
            assert!(
                (acc - fast[k]).abs() < 1e-9 * acc.abs().max(1.0),
                "short bin {k}: direct {acc} vs fft {}",
                fast[k]
            );
        }
    }

    #[test]
    fn fft_mdct_matches_direct() {
        let mdct = Mdct::new();
        let win = kbd_window();
        // Deterministic pseudo-random frame.
        let mut state = 0x12345678u64;
        let frame: Vec<f64> = (0..L)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((state >> 33) as f64 / (1u64 << 31) as f64) - 1.0
            })
            .collect();
        let mut fast = vec![0.0; N];
        mdct.forward(&frame, &win, &mut fast);
        // Direct definition.
        let n0 = N as f64 / 2.0 + 0.5;
        for &k in &[0usize, 1, 17, 500, 1023] {
            let mut acc = 0.0;
            for n in 0..L {
                acc += frame[n]
                    * win[n]
                    * (std::f64::consts::PI / N as f64 * (n as f64 + n0) * (k as f64 + 0.5)).cos();
            }
            assert!(
                (acc - fast[k]).abs() < 1e-9 * acc.abs().max(1.0),
                "bin {k}: direct {acc} vs fft {}",
                fast[k]
            );
        }
    }
}
