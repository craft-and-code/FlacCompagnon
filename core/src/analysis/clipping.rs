//! Clipping detection over normalized samples, with independent channel runs.

use crate::ClippingInfo;

/// Minimum run of consecutive full-scale samples that counts as one clip event.
const MIN_RUN: u32 = 3;

/// Incremental clipping detector.
pub struct ClipState {
    threshold: f32,
    clipped_samples: u64,
    clip_events: u64,
    current_runs: Vec<u32>,
    peak: f32,
}

impl ClipState {
    /// Start tracking clipping against `threshold` (a sample magnitude, e.g.
    /// `1.0` for exact full-scale).
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            clipped_samples: 0,
            clip_events: 0,
            current_runs: vec![0],
            peak: 0.0,
        }
    }

    /// Feed one mono sample. Multichannel callers must use [`Self::push_frame`]
    /// so adjacent channels cannot create or interrupt each other's runs.
    pub fn push(&mut self, sample: f32) {
        self.push_frame(&[sample]);
    }

    /// Feed a complete frame, counting runs independently on each channel.
    /// Simultaneous clipping on two channels counts as two channel events.
    pub fn push_frame(&mut self, samples: &[f32]) {
        if self.current_runs.len() != samples.len() {
            self.current_runs.clear();
            self.current_runs.resize(samples.len(), 0);
        }
        for (&sample, run) in samples.iter().zip(&mut self.current_runs) {
            let magnitude = sample.abs();
            self.peak = self.peak.max(magnitude);
            if magnitude >= self.threshold {
                self.clipped_samples += 1;
                if *run == MIN_RUN - 1 {
                    self.clip_events += 1;
                }
                // Once counted, a long run only needs to retain its minimum
                // length; saturating there also prevents hours of DC clipping
                // from overflowing the counter at high sample rates.
                *run = (*run + 1).min(MIN_RUN);
            } else {
                *run = 0;
            }
        }
    }

    /// Track only the sample peak required by dynamics and authenticity.
    pub fn push_peak(&mut self, sample: f32) {
        self.peak = self.peak.max(sample.abs());
    }

    /// Unclamped sample magnitude for crest-factor measurements. Float PCM
    /// may exceed full scale; its peak must use the same gain as its RMS.
    pub(crate) fn unclamped_peak(&self) -> f32 {
        self.peak
    }

    /// Consume the accumulated state into a final [`ClippingInfo`].
    ///
    /// `_channels`/`_frames` are accepted but currently unused here — they
    /// exist so the signature doesn't need to change if a future metric
    /// needs them, and to match the shape of the analyzer's other `finish`
    /// calls.
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
#[path = "../../tests/unit/analysis/clipping.rs"]
mod tests;
