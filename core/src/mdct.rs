//! Modified Discrete Cosine Transform (MDCT) with a sine (Princen–Bradley)
//! window — the transform AAC uses for its "long" blocks.
//!
//! This is used by the AAC-SIN transcoding detector: an AAC encoder quantizes
//! and zeroes MDCT coefficients (whole scale-factor bands, high frequencies),
//! leaving a flat, sharply-bounded high-frequency "dead zone" that survives the
//! decode to PCM. Recomputing the MDCT with AAC's sine window exposes it.
//!
//! The transform is implemented directly from its definition (O(N²) per frame).
//! That is trivially correct — no FFT-twiddle subtleties to get wrong — and the
//! detector only analyzes a bounded number of frames, so it stays fast. The
//! cosine basis is built once and shared across all files.
//!
//! Definition (input length `2N`, output length `N`):
//! `X[k] = Σ_{n=0}^{2N-1} w[n]·x[n]·cos[ (π/N)·(n + ½ + N/2)·(k + ½) ]`

use std::sync::OnceLock;

/// AAC long-block size (number of MDCT coefficients).
pub const AAC_N: usize = 1024;

static SHARED: OnceLock<Mdct> = OnceLock::new();

/// A precomputed MDCT of a fixed size.
pub struct Mdct {
    n: usize,
    /// Sine analysis window, length `2N`.
    window: Vec<f32>,
    /// Cosine basis, row-major `[k][t]`, length `N * 2N`.
    cos: Vec<f32>,
}

impl Mdct {
    /// Build an MDCT of output size `n` (input frames are `2n` samples).
    pub fn new(n: usize) -> Self {
        let len = 2 * n;
        let window: Vec<f32> = (0..len)
            .map(|t| ((std::f32::consts::PI / len as f32) * (t as f32 + 0.5)).sin())
            .collect();
        let n0 = n as f32 / 2.0 + 0.5;
        let mut cos = vec![0.0f32; n * len];
        let scale = std::f32::consts::PI / n as f32;
        for k in 0..n {
            let base = k * len;
            let kf = k as f32 + 0.5;
            for t in 0..len {
                cos[base + t] = (scale * (t as f32 + n0) * kf).cos();
            }
        }
        Self { n, window, cos }
    }

    /// The shared AAC-sized (`N = 1024`) MDCT, built once on first use.
    pub fn shared() -> &'static Mdct {
        SHARED.get_or_init(|| Mdct::new(AAC_N))
    }

    pub fn n(&self) -> usize {
        self.n
    }

    /// Transform one `2N`-sample `frame` into `out` (`N` coefficients).
    pub fn forward(&self, frame: &[f32], out: &mut [f32]) {
        let len = 2 * self.n;
        debug_assert_eq!(frame.len(), len);
        debug_assert_eq!(out.len(), self.n);
        for k in 0..self.n {
            let base = k * len;
            let mut acc = 0.0f32;
            for t in 0..len {
                acc += frame[t] * self.window[t] * self.cos[base + t];
            }
            out[k] = acc;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_energy_concentrates_near_its_bin() {
        // A mid-band sinusoid at the centre frequency of bin k0 should place the
        // largest MDCT coefficient at (or adjacent to) k0.
        let n = 256;
        let mdct = Mdct::new(n);
        let len = 2 * n;
        let k0 = 60usize;
        // MDCT bin k corresponds to normalized frequency (k + 0.5) / (2N).
        let f = (k0 as f32 + 0.5) / (len as f32);
        let frame: Vec<f32> = (0..len)
            .map(|t| (2.0 * std::f32::consts::PI * f * t as f32).cos())
            .collect();
        let mut out = vec![0.0f32; n];
        mdct.forward(&frame, &mut out);

        let mut argmax = 0usize;
        let mut max = 0.0f32;
        for (k, &v) in out.iter().enumerate() {
            if v.abs() > max {
                max = v.abs();
                argmax = k;
            }
        }
        assert!(
            (argmax as isize - k0 as isize).abs() <= 2,
            "argmax {argmax} not near {k0}"
        );
    }

    #[test]
    fn silence_transforms_to_zero() {
        let n = 128;
        let mdct = Mdct::new(n);
        let frame = vec![0.0f32; 2 * n];
        let mut out = vec![9.0f32; n];
        mdct.forward(&frame, &mut out);
        assert!(out.iter().all(|&v| v.abs() < 1e-6));
    }
}
