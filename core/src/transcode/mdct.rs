//! The Modified Discrete Cosine Transform, computed through an FFT.
//!
//! Separate from [`crate::analysis::mdct`], which serves the spectral
//! heuristics and works in `f32` on its own window sizes. This one exists for
//! the transcoding detectors and differs in the two ways that matter to them:
//! it is `f64` throughout, and it reuses one FFT plan across the millions of
//! transforms an offset sweep performs.
//!
//! `f64` is not caution for its own sake. The whole detector rests on
//! measuring how far a coefficient sits from the nearest integer *after* it
//! has been scaled up by a large power of two. In `f32` the scaling itself
//! costs enough significant digits to blur exactly the quantity being
//! measured, and the collapse towards zero that signals a transcode would be
//! partly hidden under the transform's own rounding.

use std::f64::consts::PI;
use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

/// An MDCT of one fixed size, with its FFT plan and scratch buffers kept
/// between calls.
///
/// Built once per window size and reused: a single offset sweep runs on the
/// order of a million transforms, and planning an FFT allocates and computes
/// twiddle factors, so doing it per call would dominate the runtime.
pub struct Mdct {
    /// Number of output coefficients — half the input length.
    n: usize,
    fft: Arc<dyn Fft<f64>>,
    /// Pre-rotation `exp(-i·π·t / 2N)` for `t` in `0..2N`.
    pre: Vec<Complex<f64>>,
    /// Post-rotation `exp(-i·π·(f + ½)(N + 1) / 2N)` for `f` in `0..N`.
    post: Vec<Complex<f64>>,
    buf: Vec<Complex<f64>>,
    scratch: Vec<Complex<f64>>,
}

impl Mdct {
    /// Prepare an MDCT producing `n` coefficients from `2 * n` input samples.
    pub fn new(n: usize) -> Self {
        let len = 2 * n;
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(len);
        let scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];

        let pre = (0..len)
            .map(|t| Complex::from_polar(1.0, -PI * t as f64 / (2.0 * n as f64)))
            .collect();
        let post = (0..n)
            .map(|f| {
                let angle = -PI * (f as f64 + 0.5) * (n as f64 + 1.0) / (2.0 * n as f64);
                Complex::from_polar(1.0, angle)
            })
            .collect();

        Self {
            n,
            fft,
            pre,
            post,
            buf: vec![Complex::new(0.0, 0.0); len],
            scratch,
        }
    }

    /// Number of coefficients this instance produces.
    pub fn len(&self) -> usize {
        self.n
    }

    /// `true` when this MDCT produces no coefficients — impossible for any
    /// instance [`Mdct::new`] is called with a non-zero size, but `clippy`
    /// asks for it wherever `len` exists and a caller may genuinely want it.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Transform `input` (already windowed, `2 * n` samples) into `out`
    /// (`n` coefficients).
    ///
    /// Both lengths are checked rather than asserted: these buffers are sized
    /// from a file's own sample rate and channel count, and this crate does
    /// not panic on anything derived from a file. A wrong size returns
    /// `false` and leaves `out` untouched.
    #[must_use]
    pub fn transform(&mut self, input: &[f64], out: &mut [f64]) -> bool {
        if input.len() != 2 * self.n || out.len() != self.n {
            return false;
        }

        // Pre-twiddle, FFT, keep the first half, post-twiddle, take the real
        // part. This is the standard FFT factorisation of the MDCT; the
        // rotations are what turn a complex DFT into the real cosine
        // transform with the right phase convention.
        for (b, (x, p)) in self.buf.iter_mut().zip(input.iter().zip(&self.pre)) {
            *b = p * *x;
        }
        self.fft
            .process_with_scratch(&mut self.buf, &mut self.scratch);
        for (o, (b, p)) in out.iter_mut().zip(self.buf.iter().zip(&self.post)) {
            *o = (b * p).re;
        }
        true
    }
}

#[cfg(test)]
#[path = "../../tests/unit/transcode/mdct.rs"]
mod tests;
