//! Whole-stream DC offset, measured independently on each decoded channel.
//!
//! DC is the arithmetic sample mean (no weighting, gating or high-pass).
//! Measuring channels before any downmix preserves opposite signed offsets.
//! See <https://manual.audacityteam.org/man/dc_offset.html> for the convention.

use serde::{Deserialize, Serialize};

/// Signed per-channel means and their largest absolute magnitude.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DcOffset {
    /// Arithmetic means in decoded channel order; 1.0 is positive full scale.
    pub channel_means: Vec<f64>,
    /// Maximum absolute channel mean, in the same normalized units.
    pub max_abs: f64,
}

#[derive(Clone, Default)]
struct ChannelSum {
    sum: f64,
    correction: f64,
}

impl ChannelSum {
    fn push(&mut self, sample: f64) {
        // Compensate rounding in both operand orders. A tiny bias should not
        // disappear when accumulated beside large positive/negative samples.
        let next = self.sum + sample;
        self.correction += if self.sum.abs() >= sample.abs() {
            (self.sum - next) + sample
        } else {
            (sample - next) + self.sum
        };
        self.sum = next;
    }
}

/// Streaming mean with bounded memory for mono through 32-channel audio.
pub struct DcOffsetMeter {
    sums: Vec<ChannelSum>,
    frames: u64,
    valid: bool,
}

impl DcOffsetMeter {
    /// Create a meter. The 32-channel resource cap matches the discontinuity
    /// meter; the arithmetic itself does not depend on the sample rate.
    pub fn new(channels: usize) -> Option<Self> {
        if !(1..=32).contains(&channels) {
            return None;
        }
        Some(Self {
            sums: vec![ChannelSum::default(); channels],
            frames: 0,
            valid: true,
        })
    }

    /// Feed one complete decoded frame. Malformed input withholds the whole
    /// measurement rather than publishing a mean of just the valid prefix.
    pub fn push_frame(&mut self, samples: &[f32]) {
        if !self.valid {
            return;
        }
        if samples.len() != self.sums.len() || samples.iter().any(|x| !x.is_finite()) {
            self.valid = false;
            return;
        }
        let Some(frames) = self.frames.checked_add(1) else {
            self.valid = false;
            return;
        };
        self.frames = frames;
        for (sum, &sample) in self.sums.iter_mut().zip(samples) {
            sum.push(f64::from(sample));
        }
    }

    /// Return means for all real frames, including silence and incomplete
    /// waveform periods. An empty or invalid stream has no reading.
    pub fn finish(self) -> Option<DcOffset> {
        if !self.valid || self.frames == 0 {
            return None;
        }
        let channel_means: Vec<f64> = self
            .sums
            .iter()
            .map(|sum| (sum.sum + sum.correction) / self.frames as f64)
            .collect();
        if channel_means.iter().any(|x| !x.is_finite()) {
            return None;
        }
        let max_abs = channel_means.iter().map(|x| x.abs()).fold(0.0, f64::max);
        Some(DcOffset {
            channel_means,
            max_abs,
        })
    }
}

#[cfg(test)]
#[path = "../../tests/unit/analysis/dc_offset.rs"]
mod tests;
