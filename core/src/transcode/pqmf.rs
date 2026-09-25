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
                let angle =
                    (2.0 * i as f64 + 1.0) * (k as f64 - 16.0) * std::f64::consts::PI / 64.0;
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
#[path = "../../tests/unit/transcode/pqmf.rs"]
mod tests;
