//! Time-local stereo correlation in the audible range and four frequency bands.
//!
//! The real cross spectrum measures zero-lag opposition, not a phase angle or
//! coherence. Matched periodic Hann windows reduce boundary leakage; subtracting
//! each channel's weighted mean keeps DC from masquerading as stereo agreement.
//! See docs/local-phase.md for equations, gating and interpretation limits.

use std::sync::Arc;

use rustfft::{num_complex::Complex, Fft, FftPlanner};
use serde::{Deserialize, Serialize};

use super::stereo::analyze_phase;

// Project measurement choices, not a standard or an audibility verdict:
// >=200 ms gives at least four 20 Hz periods; a power of two bounds FFT cost.
// The -60 dBFS per-channel band RMS floor excludes silence/very quiet leakage.
const MIN_MEAN_SQUARE: f64 = 1e-6;
const BANDS: [(f64, f64); 4] = [
    (20.0, 200.0),
    (200.0, 2_000.0),
    (2_000.0, 6_000.0),
    (6_000.0, 20_000.0),
];
/// A substantially negative correlation; descriptive, not a polarity verdict.
pub const OPPOSED_CORRELATION: f32 = -0.5;

/// Correlation evidence over eligible complete windows in one frequency range.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhaseSummary {
    /// Correlation of the summed window energies (not the mean of coefficients).
    pub correlation: f32,
    /// Lowest local correlation, in -1..1.
    pub minimum_correlation: f32,
    /// Start of the window with the lowest correlation, relative to file start.
    pub minimum_start_secs: f64,
    /// Share of eligible windows with correlation <= -0.5, not a duration share.
    pub opposed_fraction: f64,
    /// Windows where both channels in this range exceed -60 dBFS RMS.
    pub eligible_windows: u64,
}

/// One nominal band, with its effective upper limit capped at Nyquist.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BandPhase {
    /// Inclusive lower edge, in Hz; may exceed Nyquist in unsupported bands.
    pub low_hz: f64,
    /// Exclusive upper edge, in Hz. No measurement when <= low_hz.
    pub high_hz: f64,
    /// None when no complete window has enough energy in both channels.
    pub summary: Option<PhaseSummary>,
}

/// Bounded summary of local L/R correlation; exactly stereo only.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalPhase {
    /// Actual FFT window length, in seconds (200 to less than 400 ms).
    pub window_secs: f64,
    /// Window step, in seconds; windows overlap by 50%.
    pub hop_secs: f64,
    /// Number of complete windows examined, including ineligible windows.
    pub analyzed_windows: u64,
    /// Broadband 20 Hz to min(20 kHz, Nyquist), under the same gate as bands.
    pub broadband: Option<PhaseSummary>,
    /// Nominal 20–200, 200–2000, 2000–6000, 6000–20000 Hz bands.
    pub bands: [BandPhase; 4],
}

#[derive(Default)]
struct Accumulator {
    energies: [f64; 3],
    summary: Option<PhaseSummary>,
    opposed: u64,
}

impl Accumulator {
    fn push(&mut self, energy: [f64; 3], start: f64) {
        let [left, right, cross] = energy;
        if left <= MIN_MEAN_SQUARE || right <= MIN_MEAN_SQUARE {
            return;
        }
        let Some(correlation) = analyze_phase(left, right, cross).correlation else {
            return;
        };
        let summary = self.summary.get_or_insert(PhaseSummary {
            correlation,
            minimum_correlation: correlation,
            minimum_start_secs: start,
            opposed_fraction: 0.0,
            eligible_windows: 0,
        });
        summary.eligible_windows += 1;
        if correlation < summary.minimum_correlation {
            summary.minimum_correlation = correlation;
            summary.minimum_start_secs = start;
        }
        self.opposed += u64::from(correlation <= OPPOSED_CORRELATION);
        for (total, value) in self.energies.iter_mut().zip(energy) {
            *total += value;
        }
        let [l, r, c] = self.energies;
        summary.correlation = analyze_phase(l, r, c).correlation.unwrap_or(correlation);
        summary.opposed_fraction = self.opposed as f64 / summary.eligible_windows as f64;
    }
}

/// Streaming stereo FFT meter; memory and stored evidence never grow with duration.
pub struct LocalPhaseMeter {
    sample_rate: u32,
    fft: Arc<dyn Fft<f64>>,
    scratch: Vec<Complex<f64>>,
    spectra: [Vec<Complex<f64>>; 2],
    hann: Vec<f64>,
    normalization: f64,
    frames: Vec<[f32; 2]>,
    windows: u64,
    accumulators: [Accumulator; 5],
    valid: bool,
}

impl LocalPhaseMeter {
    /// Accept exactly two channels from 8 to 768 kHz; the rate cap bounds FFT memory.
    pub fn new(sample_rate: u32, channels: usize) -> Option<Self> {
        if channels != 2 || !(8_000..=768_000).contains(&sample_rate) {
            return None;
        }
        let size = (sample_rate as usize).div_ceil(5).next_power_of_two();
        let fft = FftPlanner::new().plan_fft_forward(size);
        // Periodic Hann: https://www.mathworks.com/help/signal/ref/hann.html
        let hann: Vec<_> = (0..size)
            .map(|i| 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / size as f64).cos())
            .collect();
        let normalization = 2.0 / (size as f64 * hann.iter().map(|w| w * w).sum::<f64>());
        Some(Self {
            sample_rate,
            scratch: vec![Complex::default(); fft.get_inplace_scratch_len()],
            fft,
            spectra: std::array::from_fn(|_| vec![Complex::default(); size]),
            hann,
            normalization,
            frames: Vec::with_capacity(size),
            windows: 0,
            accumulators: std::array::from_fn(|_| Accumulator::default()),
            valid: true,
        })
    }

    /// Feed a complete L/R frame. A malformed tail invalidates the whole reading.
    pub fn push_frame(&mut self, samples: &[f32]) {
        if !self.valid {
            return;
        }
        let [left, right] = samples else {
            self.valid = false;
            return;
        };
        if !left.is_finite() || !right.is_finite() {
            self.valid = false;
            return;
        }
        self.frames.push([*left, *right]);
        if self.frames.len() == self.hann.len() {
            self.process_window();
            let half = self.frames.len() / 2;
            self.frames.copy_within(half.., 0);
            self.frames.truncate(half);
        }
    }

    fn process_window(&mut self) {
        let size = self.hann.len();
        for (channel, spectrum) in self.spectra.iter_mut().enumerate() {
            let mean = self
                .frames
                .iter()
                .zip(&self.hann)
                .map(|(frame, w)| f64::from(frame[channel]) * w)
                .sum::<f64>()
                / (size as f64 * 0.5);
            for ((value, frame), w) in spectrum.iter_mut().zip(&self.frames).zip(&self.hann) {
                *value = Complex::new((f64::from(frame[channel]) - mean) * w, 0.0);
            }
            self.fft.process_with_scratch(spectrum, &mut self.scratch);
        }
        let mut energies = [[0.0; 3]; 5];
        let [left, right] = &self.spectra;
        // Positive-frequency bins only; neither DC nor Nyquist is counted.
        for (k, (l, r)) in left.iter().zip(right).enumerate().take(size / 2).skip(1) {
            let frequency = k as f64 * self.sample_rate as f64 / size as f64;
            if !(20.0..20_000.0).contains(&frequency) {
                continue;
            }
            let Some(band) = BANDS
                .iter()
                .position(|&(lo, hi)| (lo..hi).contains(&frequency))
            else {
                continue;
            };
            let energy = [l.norm_sqr(), r.norm_sqr(), (l * r.conj()).re];
            for index in [0, band + 1] {
                for (total, value) in energies[index].iter_mut().zip(energy) {
                    *total += value * self.normalization;
                }
            }
        }
        let start = self.windows as f64 * (size / 2) as f64 / self.sample_rate as f64;
        for (accumulator, energy) in self.accumulators.iter_mut().zip(energies) {
            accumulator.push(energy, start);
        }
        self.windows += 1;
    }

    /// Return eligible evidence. Partial tails are not zero-padded or counted;
    /// silent/short/invalid streams have no result, not a fictitious zero correlation.
    pub fn finish(self) -> Option<LocalPhase> {
        if !self.valid || self.accumulators.iter().all(|a| a.summary.is_none()) {
            return None;
        }
        let window_secs = self.hann.len() as f64 / self.sample_rate as f64;
        Some(LocalPhase {
            window_secs,
            hop_secs: window_secs * 0.5,
            analyzed_windows: self.windows,
            broadband: self.accumulators[0].summary,
            bands: std::array::from_fn(|i| BandPhase {
                low_hz: BANDS[i].0,
                high_hz: BANDS[i].1.min(self.sample_rate as f64 * 0.5),
                summary: self.accumulators[i + 1].summary,
            }),
        })
    }
}

#[cfg(test)]
#[path = "../../tests/unit/analysis/local_phase.rs"]
mod tests;
