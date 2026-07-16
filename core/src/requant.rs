//! AAC re-quantization transcode detector (the Lossless Audio Checker method).
//!
//! An AAC encoder quantizes MDCT coefficients per scale-factor band on the grid
//! `|X| = n^(4/3) · Δ` (n integer, Δ the band's scale). In the `|X|^(3/4)`
//! domain this is a *linear* grid with step `Δ^(3/4)`. The decode to PCM is an
//! IMDCT with overlap-add; thanks to TDAC, re-analyzing the decoded signal with
//! the **same window at the exact frame alignment** recovers those quantized
//! coefficients — so almost every band snaps onto an integer grid. For genuine
//! (never-AAC-encoded) audio no alignment produces that snap.
//!
//! The detector therefore:
//! 1. sweeps all 1024 possible frame onsets at one-sample resolution (the
//!    "needle" is destroyed by a single sample of misalignment);
//! 2. tests both AAC window shapes (sine and KBD α=4 — ffmpeg's encoder uses
//!    KBD) and both stereo representations (L/R and M/S) per band;
//! 3. scores each (frame, band) cell: the grid step is estimated from the
//!    smallest surviving coefficient (quantized value 1, also trying 2 and 3),
//!    and the cell is a *hit* when the mean distance to integers is < 0.05;
//! 4. refines the best onset over many frames. A high hit-rate is essentially
//!    impossible for genuine audio (measured ≤ 0.014 over all onsets on real
//!    music) while real AAC→FLAC transcodes score 0.70–0.97 — including
//!    256 kbps, which spectral methods cannot see.
//!
//! Empirically calibrated on ffmpeg AAC transcodes at 128/192/256 kbps decoded
//! through a 16-bit FLAC chain, against the untouched originals.
//!
//! Cost: one full sweep is ~16k FFT-based MDCTs (a fraction of a second in
//! release builds). Only applies at 44.1/48 kHz, per the LAC paper.

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

/// Band range analyzed (skips the lowest bass bands and the top catch-all).
const BAND_LO: usize = 8;
const BAND_HI: usize = 46;

/// Coefficients below this absolute magnitude are ignored (16-bit PCM noise).
const COEF_FLOOR: f64 = 1e-3;
/// A band cell is a hit when its mean distance-to-integer is below this.
const HIT_RESIDUAL: f64 = 0.05;
/// Minimum surviving coefficients for a band cell to be scored.
const MIN_COEFS: usize = 6;
/// Coarse-sweep candidates above this rate are refined.
const CANDIDATE_RATE: f32 = 0.20;
/// Refined hit-rate at/above which the file is flagged as AAC-transcoded.
pub const DETECT_RATE: f32 = 0.45;
/// Minimum refined cells for a decision.
const MIN_TESTED: usize = 150;

/// Samples per channel the detector needs (onset sweep + refine frames).
pub const SEGMENT_LEN: usize = 1023 + L + REFINE_FRAMES * N;
/// Refinement frame count.
const REFINE_FRAMES: usize = 30;
/// Frame indices used by the coarse sweep.
const COARSE_FRAMES: [usize; 2] = [8, 20];

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

/// AAC sine window, length 2N.
pub fn sine_window() -> Vec<f64> {
    (0..L)
        .map(|n| (std::f64::consts::PI / L as f64 * (n as f64 + 0.5)).sin())
        .collect()
}

/// AAC Kaiser–Bessel-derived window (α = 4), length 2N.
pub fn kbd_window() -> Vec<f64> {
    let a = 4.0 * std::f64::consts::PI;
    let m = N as f64;
    let kernel: Vec<f64> = (0..=N)
        .map(|j| {
            let t = (j as f64 - m / 2.0) / (m / 2.0);
            bessel_i0(a * (1.0 - t * t).max(0.0).sqrt())
        })
        .collect();
    let mut cum = Vec::with_capacity(N + 1);
    let mut acc = 0.0;
    for w in &kernel {
        acc += w;
        cum.push(acc);
    }
    let total = cum[N];
    let half: Vec<f64> = (0..N).map(|j| (cum[j] / total).sqrt()).collect();
    let mut win = half.clone();
    win.extend(half.iter().rev());
    win
}

/// FFT-based forward MDCT (one 2N-point complex FFT per frame).
///
/// Holds a reusable scratch buffer (`RefCell`), so a single instance performs
/// thousands of transforms without per-call allocation. Not `Sync`; use one
/// instance per thread.
pub struct Mdct {
    fft: Arc<dyn Fft<f64>>,
    pre: Vec<Complex<f64>>,
    post: Vec<Complex<f64>>,
    scratch: RefCell<Vec<Complex<f64>>>,
}

impl Mdct {
    pub fn new() -> Self {
        let fft = FftPlanner::<f64>::new().plan_fft_forward(L);
        let pre: Vec<Complex<f64>> = (0..L)
            .map(|n| Complex::from_polar(1.0, -std::f64::consts::PI * n as f64 / L as f64))
            .collect();
        let n0 = N as f64 / 2.0 + 0.5;
        let post: Vec<Complex<f64>> = (0..N)
            .map(|k| {
                Complex::from_polar(
                    1.0,
                    -std::f64::consts::PI * n0 * (k as f64 + 0.5) / N as f64,
                )
            })
            .collect();
        Self {
            fft,
            pre,
            post,
            scratch: RefCell::new(Vec::with_capacity(L)),
        }
    }

    /// Transform `frame` (length 2N) windowed by `win` into `out` (length N).
    pub fn forward(&self, frame: &[f64], win: &[f64], out: &mut [f64]) {
        debug_assert_eq!(frame.len(), L);
        let mut buf = self.scratch.borrow_mut();
        buf.clear();
        buf.extend((0..L).map(|n| self.pre[n] * (frame[n] * win[n])));
        self.fft.process(&mut buf);
        for k in 0..N {
            out[k] = (self.post[k] * buf[k]).re;
        }
    }
}

impl Default for Mdct {
    fn default() -> Self {
        Self::new()
    }
}

/// Best mean distance-to-integer for a band's `|coef|^(3/4)` values, with the
/// grid step estimated from the smallest value (quantized 1, 2 or 3).
fn band_residual(y: &[f64]) -> Option<f64> {
    if y.len() < MIN_COEFS {
        return None;
    }
    let ymin = y.iter().cloned().fold(f64::INFINITY, f64::min);
    if !(ymin > 0.0) {
        return None;
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
    Some(best)
}

/// Hit / tested counts for one onset over `frames`, taking the per-band minimum
/// residual across the channel representations (L, R, M, S).
#[allow(clippy::too_many_arguments)]
fn score_onset(
    mdct: &Mdct,
    win: &[f64],
    chans: &[&[f64]],
    onset: usize,
    frames: &[usize],
    coefs: &mut [Vec<f64>],
    y: &mut Vec<f64>,
) -> (usize, usize) {
    let mut hits = 0usize;
    let mut tested = 0usize;
    for &m in frames {
        let start = onset + m * N;
        for (ci, ch) in chans.iter().enumerate() {
            mdct.forward(&ch[start..start + L], win, &mut coefs[ci]);
        }
        for b in BAND_LO..BAND_HI {
            let (lo, hi) = (SWB_4448[b], SWB_4448[b + 1]);
            let mut best: Option<f64> = None;
            for c in coefs.iter() {
                y.clear();
                for &v in &c[lo..hi] {
                    let a = v.abs();
                    if a > COEF_FLOOR {
                        y.push(a.powf(0.75));
                    }
                }
                if let Some(r) = band_residual(y) {
                    best = Some(best.map_or(r, |x: f64| x.min(r)));
                }
            }
            if let Some(r) = best {
                tested += 1;
                if r < HIT_RESIDUAL {
                    hits += 1;
                }
            }
        }
    }
    (hits, tested)
}

/// Result of the re-quantization analysis.
#[derive(Debug, Clone, Copy)]
pub struct RequantResult {
    /// Refined hit-rate at the best onset/window (0..1).
    pub rate: f32,
    /// Onset (mod 1024) where the grid was found.
    pub onset: usize,
    /// Number of (frame, band) cells that backed the refined rate.
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
    let windows = [kbd_window(), sine_window()];
    let mut coefs: Vec<Vec<f64>> = (0..chans.len()).map(|_| vec![0.0; N]).collect();
    let mut y: Vec<f64> = Vec::with_capacity(96);

    let mut best: Option<RequantResult> = None;
    for win in &windows {
        // Coarse pass: every onset, two frames.
        let mut candidates: Vec<(f32, usize)> = Vec::new();
        for onset in 0..N {
            let (h, t) = score_onset(&mdct, win, &chans, onset, &COARSE_FRAMES, &mut coefs, &mut y);
            if t > 0 {
                let rate = h as f32 / t as f32;
                if rate >= CANDIDATE_RATE {
                    candidates.push((rate, onset));
                }
            }
        }
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(4);

        // Refine each candidate over many frames.
        let refine: Vec<usize> = (1..=REFINE_FRAMES).collect();
        for (_, onset) in candidates {
            let (h, t) = score_onset(&mdct, win, &chans, onset, &refine, &mut coefs, &mut y);
            if t >= MIN_TESTED {
                let rate = h as f32 / t as f32;
                if best.map_or(true, |b| rate > b.rate) {
                    best = Some(RequantResult {
                        rate,
                        onset,
                        tested: t,
                    });
                }
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
