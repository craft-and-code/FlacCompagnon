//! Clipping / true-peak-ish detection over a stream of normalized samples.

use crate::ClippingInfo;

/// Minimum run of consecutive full-scale samples that counts as one clip event.
const MIN_RUN: u32 = 3;

/// Incremental clipping detector.
pub struct ClipState {
    threshold: f32,
    clipped_samples: u64,
    clip_events: u64,
    current_run: u32,
    peak: f32,
}

impl ClipState {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            clipped_samples: 0,
            clip_events: 0,
            current_run: 0,
            peak: 0.0,
        }
    }

    /// Feed one sample (any channel).
    pub fn push(&mut self, sample: f32) {
        let a = sample.abs();
        if a > self.peak {
            self.peak = a;
        }
        if a >= self.threshold {
            self.clipped_samples += 1;
            self.current_run += 1;
            if self.current_run == MIN_RUN {
                self.clip_events += 1;
            }
        } else {
            self.current_run = 0;
        }
    }

    pub fn finish(self, _channels: usize, _frames: u64) -> ClippingInfo {
        let peak = self.peak.min(1.0);
        let peak_dbfs = if peak > 0.0 {
            20.0 * peak.log10()
        } else {
            f32::NEG_INFINITY
        };
        ClippingInfo {
            clipped_samples: self.clipped_samples,
            clip_events: self.clip_events,
            peak,
            peak_dbfs,
            // Filled in by the analyzer, which owns the oversampling meter.
            true_peak: peak,
            true_peak_dbtp: peak_dbfs,
            clipped: self.clip_events > 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_a_clip_run() {
        let mut s = ClipState::new(0.9997);
        for _ in 0..5 {
            s.push(1.0);
        }
        let info = s.finish(1, 5);
        assert!(info.clipped);
        assert_eq!(info.clip_events, 1);
        assert_eq!(info.clipped_samples, 5);
    }

    #[test]
    fn short_touches_are_not_events() {
        let mut s = ClipState::new(0.9997);
        s.push(1.0);
        s.push(0.2);
        s.push(1.0);
        let info = s.finish(1, 3);
        assert!(!info.clipped);
        assert_eq!(info.clip_events, 0);
    }
}
