//! MP3's polyphase analysis filterbank — the first of its three transform
//! stages.
//!
//! Where AAC takes one MDCT straight off the PCM, Layer III first splits the
//! signal into 32 equal-width subbands with a 512-tap polyphase filter, and
//! only then applies a small MDCT inside each band. This file is that first
//! stage; [`super::mp3`] drives the rest.
//!
//! # The shape of one step
//!
//! Each output step consumes 32 fresh samples and looks back over 512 in
//! total, so consecutive steps overlap by 480. The standard's own formulation
//! — reproduced here because it is what makes the arithmetic cheap — is:
//!
//! 1. take the last 512 samples, **time-reversed**;
//! 2. multiply by the analysis window `C`;
//! 3. fold the 512 products down to 64 by summing every 64th;
//! 4. multiply that 64-vector by a fixed 32×64 cosine matrix.
//!
//! The fold is what makes it polyphase: a naive implementation would need
//! 32 × 512 multiplies per step, this needs 512 + 32 × 64.
//!
//! # Why one granule needs the previous one
//!
//! A granule is 576 samples — 18 steps of 32. The first of those steps still
//! reaches 512 samples into the past, most of which belong to the *previous*
//! granule. [`Pqmf::granule`] therefore takes both, and the caller must feed
//! them in order.

use super::mp3_tables::{GRANULE_LEN, GRANULE_SIZE, PQMF_BANDS, PQMF_WINDOW, PQMF_WINDOW_LEN};

/// Blocks of 32 samples the window looks back over, beyond the current step.
const LOOKBACK_BLOCKS: usize = 16;
/// Where in the previous granule the carried tail begins: `(18 − 16 + 1) × 32
/// = 96` samples in.
const CARRY_START: usize = (GRANULE_SIZE - LOOKBACK_BLOCKS + 1) * PQMF_BANDS;

/// Samples of the previous granule the first step still needs: 480.
///
/// Plus a full 576-sample granule, that is exactly the 1056 the eighteenth
/// step's window ends on. Off by one block in either direction and the last
/// step reads past the buffer or the first one starts in the wrong place —
/// and the tell is subtle, because a steady tone still lands in the right
/// band either way. (This constant was briefly the *offset* rather than the
/// *count*, 96 instead of 480; the test below is what it exists for.)
const CARRY: usize = GRANULE_LEN - CARRY_START;

/// The analysis filterbank, with its cosine matrix built once.
pub struct Pqmf {
    /// `cos((2i + 1)(k − 16)π / 64)` for `i` in `0..32`, `k` in `0..64`,
    /// stored row-major.
    modulation: Vec<f64>,
    /// Scratch: the 512 windowed products, then folded to 64.
    window: Vec<f64>,
    folded: Vec<f64>,
    /// Scratch: `prev`'s tail followed by the current granule.
    buffer: Vec<f64>,
}

impl Pqmf {
    /// Build the filterbank. Cheap, but done once per worker rather than per
    /// granule — a sweep runs this thousands of times.
    pub fn new() -> Self {
        let mut modulation = Vec::with_capacity(PQMF_BANDS * 64);
        for i in 0..PQMF_BANDS {
            for k in 0..64 {
                let angle = (2.0 * i as f64 + 1.0) * (k as f64 - 16.0) * std::f64::consts::PI / 64.0;
                modulation.push(angle.cos());
            }
        }
        Self {
            modulation,
            window: vec![0.0; PQMF_WINDOW_LEN],
            folded: vec![0.0; 64],
            buffer: vec![0.0; CARRY + GRANULE_LEN],
        }
    }

    /// Analyze one granule into `out`, a `32 × 18` array stored **band-major**
    /// (`out[band * 18 + step]`).
    ///
    /// `prev` is the granule immediately before `cur`; both must be
    /// [`GRANULE_LEN`] samples and `out` must hold `32 × 18`. Wrong sizes
    /// return `false` rather than panicking — these come from a decoded file.
    #[must_use]
    pub fn granule(&mut self, prev: &[f64], cur: &[f64], out: &mut [f64]) -> bool {
        if prev.len() != GRANULE_LEN
            || cur.len() != GRANULE_LEN
            || out.len() != PQMF_BANDS * GRANULE_SIZE
        {
            return false;
        }

        self.buffer[..CARRY].copy_from_slice(&prev[GRANULE_LEN - CARRY..]);
        self.buffer[CARRY..].copy_from_slice(cur);

        for step in 0..GRANULE_SIZE {
            let from = step * PQMF_BANDS;
            let slice = &self.buffer[from..from + PQMF_WINDOW_LEN];

            // Time-reversed and windowed. The reversal is part of the
            // standard's definition, not an implementation choice: it is what
            // makes the fold below equivalent to the direct convolution.
            for (n, w) in self.window.iter_mut().enumerate() {
                *w = slice[PQMF_WINDOW_LEN - 1 - n] * PQMF_WINDOW[n];
            }

            // Fold 512 down to 64 by summing every 64th.
            for (k, f) in self.folded.iter_mut().enumerate() {
                let mut acc = 0.0;
                for block in 0..8 {
                    acc += self.window[block * 64 + k];
                }
                *f = acc;
            }

            for band in 0..PQMF_BANDS {
                let row = &self.modulation[band * 64..(band + 1) * 64];
                let mut acc = 0.0;
                for (m, f) in row.iter().zip(&self.folded) {
                    acc += m * f;
                }
                out[band * GRANULE_SIZE + step] = acc;
            }
        }
        true
    }
}

impl Default for Pqmf {
    fn default() -> Self {
        Self::new()
    }
}

/// Flip the sign of odd subbands at odd time steps.
///
/// Layer III's filterbank alternates the phase of every second subband, and
/// the decoder undoes it before the inverse MDCT. An analysis that skipped
/// this would produce coefficients that are individually the right magnitude
/// but collectively on the wrong lattice — the detector would find nothing,
/// and nothing about the output would look wrong.
///
/// `coeffs` is `32 × steps`, band-major.
pub fn flip_odd_subbands(coeffs: &mut [f64], steps: usize) {
    if steps == 0 {
        return;
    }
    let mut band = 1;
    while band < PQMF_BANDS {
        let row = band * steps;
        let mut step = 1;
        while step < steps {
            if let Some(v) = coeffs.get_mut(row + step) {
                *v = -*v;
            }
            step += 2;
        }
        band += 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The buffer arithmetic, asserted rather than trusted: the eighteenth
    /// step's 512-sample window must end exactly on the last sample of the
    /// granule, with nothing left over and nothing missing.
    #[test]
    fn the_lookback_buffer_is_exactly_long_enough() {
        assert_eq!(CARRY_START, 96, "the tail starts 96 samples into the granule");
        assert_eq!(CARRY, 480, "480 samples carried from the previous granule");
        assert_eq!(CARRY + GRANULE_LEN, 1056);
        let last_step_end = (GRANULE_SIZE - 1) * PQMF_BANDS + PQMF_WINDOW_LEN;
        assert_eq!(last_step_end, CARRY + GRANULE_LEN, "must end flush");
    }

    fn tone(hz: f64, sr: f64, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * hz * i as f64 / sr).sin() * 0.5)
            .collect()
    }

    /// A pure tone must land in the subband that covers its frequency, and
    /// almost nowhere else. Each of the 32 bands spans Nyquist/32 — 689 Hz at
    /// 44.1 kHz.
    ///
    /// This is the check that the modulation matrix, the window and the fold
    /// agree with each other: get any one of them wrong and the energy lands
    /// in the wrong band, or smears across all of them.
    ///
    /// The tones are placed at band *centres* rather than at round
    /// frequencies. A round number can fall near a band edge — 9 kHz sits
    /// 42 Hz above the 13/14 boundary — where energy legitimately splits
    /// between neighbours (72/28 in that case) and a concentration assertion
    /// fails for a reason that has nothing to do with correctness.
    #[test]
    fn a_tone_lands_in_the_band_that_covers_it() {
        let sr = 44_100.0;
        let band_hz = sr / 2.0 / PQMF_BANDS as f64;
        let mut pqmf = Pqmf::new();
        let mut out = vec![0.0; PQMF_BANDS * GRANULE_SIZE];

        for band in [1usize, 4, 13, 21, 28] {
            let hz = (band as f64 + 0.5) * band_hz;
            let sig = tone(hz, sr, GRANULE_LEN * 6);
            // Granules 2 and 3, well past the filterbank's start-up.
            let prev = &sig[2 * GRANULE_LEN..3 * GRANULE_LEN];
            let cur = &sig[3 * GRANULE_LEN..4 * GRANULE_LEN];
            assert!(pqmf.granule(prev, cur, &mut out));

            let energy: Vec<f64> = (0..PQMF_BANDS)
                .map(|b| out[b * GRANULE_SIZE..(b + 1) * GRANULE_SIZE].iter().map(|v| v * v).sum())
                .collect();
            let loudest = energy
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .expect("32 bands");
            assert_eq!(loudest, band, "{hz:.0} Hz should sit in band {band}");

            // And it must dominate: a filterbank that smeared energy across
            // bands could still pick the right maximum by a hair.
            let total: f64 = energy.iter().sum();
            assert!(
                energy[loudest] / total > 0.99,
                "{hz:.0} Hz: only {:.1}% of energy in its own band",
                100.0 * energy[loudest] / total
            );
        }
    }

    #[test]
    fn silence_analyzes_to_silence() {
        let mut pqmf = Pqmf::new();
        let mut out = vec![f64::NAN; PQMF_BANDS * GRANULE_SIZE];
        let zeros = vec![0.0; GRANULE_LEN];
        assert!(pqmf.granule(&zeros, &zeros, &mut out));
        assert!(out.iter().all(|v| v.abs() < 1e-12), "{:?}", &out[..8]);
    }

    #[test]
    fn wrong_buffer_sizes_are_refused_without_panicking() {
        let mut pqmf = Pqmf::new();
        let g = vec![0.0; GRANULE_LEN];
        let short = vec![0.0; GRANULE_LEN - 1];
        let mut out = vec![0.0; PQMF_BANDS * GRANULE_SIZE];
        assert!(!pqmf.granule(&short, &g, &mut out));
        assert!(!pqmf.granule(&g, &short, &mut out));
        assert!(!pqmf.granule(&g, &g, &mut [0.0; 8]));
        assert!(!pqmf.granule(&[], &[], &mut out));
    }

    /// Odd bands at odd steps, and nothing else. Written as an explicit map
    /// because an off-by-one in either index is invisible in the output.
    #[test]
    fn the_sign_flip_touches_exactly_the_odd_odd_cells() {
        let steps = 4;
        let mut coeffs = vec![1.0; PQMF_BANDS * steps];
        flip_odd_subbands(&mut coeffs, steps);
        for band in 0..PQMF_BANDS {
            for step in 0..steps {
                let want = if band % 2 == 1 && step % 2 == 1 { -1.0 } else { 1.0 };
                assert_eq!(
                    coeffs[band * steps + step], want,
                    "band {band} step {step}"
                );
            }
        }
        // Applying it twice restores the original — it is an involution.
        flip_odd_subbands(&mut coeffs, steps);
        assert!(coeffs.iter().all(|v| *v == 1.0));

        // Degenerate input must not panic.
        flip_odd_subbands(&mut [], 0);
        flip_odd_subbands(&mut [1.0], 1);
    }
}
