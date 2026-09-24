//! Maximum momentary (400 ms) and short-term (3 s) loudness with their locations.
//!
//! EBU Tech 3341 §2.1–2.2 requires rectangular, ungated M/S measurements.
//! The shared loudness meter supplies its rolling K-weighted energy sums, so
//! these maxima need neither another filter pass nor another sample history.
//! Every complete sample-aligned window is eligible, including off-hop tails.

use serde::{Deserialize, Serialize};

use super::loudness::power_to_lufs;

/// Maximum loudness over one complete real-audio window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LoudnessPeak {
    /// K-weighted, ungated loudness in LUFS.
    pub lufs: f32,
    /// Start of the maximum window, in seconds from the first decoded frame.
    pub start_secs: f64,
}

/// File maxima; either timescale can be unavailable independently.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LoudnessPeaks {
    /// Maximum over complete 400 ms windows; none for silence or shorter audio.
    pub momentary: Option<LoudnessPeak>,
    /// Maximum over complete 3 s windows; also absent for unrepresentable power.
    pub short_term: Option<LoudnessPeak>,
}

struct MaximumWindow {
    frames: u64,
    power: f64,
    start_frame: u64,
}

impl MaximumWindow {
    fn new(frames: u64) -> Self {
        Self {
            frames,
            power: 0.0,
            start_frame: 0,
        }
    }

    fn push(&mut self, sum: f64, end_frame: u64) {
        // Reject partial startup windows; no zero padding or silence gate.
        if end_frame >= self.frames && sum.is_finite() && sum > self.power {
            self.power = sum;
            self.start_frame = end_frame - self.frames;
        }
    }

    fn result(&self, sample_rate: u32) -> Option<LoudnessPeak> {
        Some(LoudnessPeak {
            lufs: power_to_lufs(self.power / self.frames as f64)?,
            start_secs: self.start_frame as f64 / sample_rate as f64,
        })
    }
}

pub(super) struct LoudnessPeaksMeter {
    sample_rate: u32,
    momentary: MaximumWindow,
    short_term: MaximumWindow,
}

impl LoudnessPeaksMeter {
    pub(super) fn new(sample_rate: u32, momentary_frames: usize, short_frames: usize) -> Self {
        Self {
            sample_rate,
            momentary: MaximumWindow::new(momentary_frames as u64),
            short_term: MaximumWindow::new(short_frames as u64),
        }
    }

    pub(super) fn push(&mut self, momentary_sum: f64, short_sum: f64, frames: u64) {
        self.momentary.push(momentary_sum, frames);
        self.short_term.push(short_sum, frames);
    }

    pub(super) fn result(&self, valid: bool, short_valid: bool) -> Option<LoudnessPeaks> {
        if !valid {
            return None;
        }
        let momentary = self.momentary.result(self.sample_rate);
        let short_term = short_valid
            .then(|| self.short_term.result(self.sample_rate))
            .flatten();
        (momentary.is_some() || short_term.is_some()).then_some(LoudnessPeaks {
            momentary,
            short_term,
        })
    }
}

#[cfg(test)]
#[path = "../../tests/unit/analysis/loudness_peaks.rs"]
mod tests;
