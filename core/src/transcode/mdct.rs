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
        self.fft.process_with_scratch(&mut self.buf, &mut self.scratch);
        for (o, (b, p)) in out.iter_mut().zip(self.buf.iter().zip(&self.post)) {
            *o = (b * p).re;
        }
        true
    }
}

#[cfg(test)]
mod tests {
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
                        let phase = PI / n as f64
                            * (t as f64 + 0.5 + n as f64 / 2.0)
                            * (f as f64 + 0.5);
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
            assert_close(&got, &naive_mdct(&input), 1e-9 * n as f64, &format!("n={n}"));
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
}
