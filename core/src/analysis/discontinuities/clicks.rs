//! Isolated short-pulse detection with context before and after both edges.

use super::{context_frames, pulse_frames, DiscontinuityEvent, EventSummary};

// Project thresholds: at least 0.05 full-scale jump (~−26 dBFS), and 8 times
// the surrounding first-difference RMS (~18 dB). Both edges must stand out.
// A 0.5 ms pulse limit plus 5 ms of context rejects sustained high-frequency
// content and most ordinary attacks. This deliberately misses softer clicks.
const MIN_JUMP: f64 = 0.05;
const EDGE_RATIO: f64 = 8.0;
// Edges must largely cancel: a pulse returns toward its preceding waveform.
const RETURN_FRACTION: f64 = 0.25;

pub(super) struct ClickDetector {
    samples: Vec<f64>,
    context: usize,
    max_width: usize,
    hop: usize,
    rate: u32,
    channel: u16,
    base: u64,
    skip_until: u64,
}

impl ClickDetector {
    pub(super) fn new(rate: u32, channel: u16) -> Self {
        let context = context_frames(rate);
        let max_width = pulse_frames(rate);
        let hop = (rate as usize).div_ceil(100);
        Self {
            samples: Vec::with_capacity(2 * context + hop + max_width + 1),
            context,
            max_width,
            hop,
            rate,
            channel,
            base: 0,
            skip_until: 0,
        }
    }

    pub(super) fn push(&mut self, sample: f64, output: &mut EventSummary) {
        self.samples.push(sample);
        if self.samples.len() == 2 * self.context + self.hop + self.max_width + 1 {
            self.scan(output);
            self.samples.drain(..self.hop);
            self.base += self.hop as u64;
        }
    }

    pub(super) fn finish(&mut self, output: &mut EventSummary) {
        // The retained left context belongs to already-scanned candidates;
        // scan() starts beyond it, including on the final partial batch.
        self.scan(output);
    }

    fn scan(&mut self, output: &mut EventSummary) {
        let mut skip_until = self.skip_until;
        let mut prefix = Vec::with_capacity(self.samples.len() + 1);
        prefix.extend([0.0, 0.0]);
        let mut sum = 0.0;
        for pair in self.samples.windows(2) {
            sum += (pair[1] - pair[0]).powi(2);
            prefix.push(sum);
        }
        let end = self
            .samples
            .len()
            .saturating_sub(self.context + self.max_width);
        for start in self.context + 1..end {
            if self.base + (start as u64) < skip_until {
                continue;
            }
            // Bounds follow from full left context and reserved right context.
            let before =
                (prefix[start] - prefix[start - self.context]).max(0.0) / self.context as f64;
            let edge = self.samples[start] - self.samples[start - 1];
            if edge.abs() < MIN_JUMP || edge * edge < EDGE_RATIO.powi(2) * before {
                continue;
            }
            for stop in start + 1..=start + self.max_width {
                let exit = self.samples[stop] - self.samples[stop - 1];
                if edge * exit >= 0.0
                    || exit.abs() < MIN_JUMP
                    || (edge + exit).abs() > RETURN_FRACTION * edge.abs().max(exit.abs())
                {
                    continue;
                }
                let after = (prefix[stop + self.context + 1] - prefix[stop + 1]).max(0.0)
                    / self.context as f64;
                if edge.abs().min(exit.abs()).powi(2) < EDGE_RATIO.powi(2) * before.max(after) {
                    continue;
                }
                output.record(DiscontinuityEvent {
                    channel: self.channel,
                    start_secs: (self.base + start as u64) as f64 / self.rate as f64,
                    duration_secs: (stop - start) as f64 / self.rate as f64,
                });
                // A burst of nearby edges is one cue to audition, not one
                // independent defect per sample. Reserve a 1 ms quiet gap.
                skip_until = self.base + stop as u64 + (self.rate as u64).div_ceil(1_000);
                break;
            }
        }
        self.skip_until = skip_until;
    }
}
