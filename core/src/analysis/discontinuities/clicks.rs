//! Isolated short-pulse detection with context before and after both edges.

use super::{context_frames, pulse_frames, DiscontinuityEvent, EventSummary};
use crate::analysis::compensated_sum::CompensatedSum;

// Project thresholds: at least 0.05 full-scale jump (~−26 dBFS), and 8 times
// the surrounding first-difference RMS (~18 dB). Both edges must stand out.
// A 0.5 ms pulse limit plus 5 ms of context rejects the sustained tones and
// damped attacks in synthetic controls. This deliberately misses softer
// clicks; the thresholds are not calibrated against a musical corpus.
const MIN_JUMP: f64 = 0.05;
const EDGE_RATIO: f64 = 8.0;
// Edges must largely cancel: a pulse returns toward its preceding waveform.
const RETURN_FRACTION: f64 = 0.25;

pub(super) struct ClickDetector {
    samples: Vec<f64>,
    prefix: Vec<CompensatedSum>,
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
            prefix: Vec::with_capacity(2 * context + hop + max_width + 2),
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
            self.scan(output, self.context + self.hop + 1);
            self.samples.drain(..self.hop);
            self.base += self.hop as u64;
        }
    }

    pub(super) fn finish(&mut self, output: &mut EventSummary) {
        // The retained left context belongs to already-scanned candidates;
        // scan() starts beyond it, including on the final partial batch.
        // The final context must fit the candidate's actual width, rather
        // than reserving the maximum width and losing shorter end pulses.
        let end = self.samples.len().saturating_sub(self.context + 1);
        self.scan(output, end);
    }

    fn scan(&mut self, output: &mut EventSummary, end: usize) {
        let mut skip_until = self.skip_until;
        self.prefix.clear();
        let last_stop = self.samples.len().saturating_sub(self.context + 1);
        for start in self.context + 1..end {
            if self.base + (start as u64) < skip_until {
                continue;
            }
            let edge = self.samples[start] - self.samples[start - 1];
            if edge.abs() < MIN_JUMP {
                continue;
            }
            if self.prefix.is_empty() {
                // Smooth batches never need energy prefixes. Once a candidate
                // does, include the whole buffer: a late first edge still
                // needs its earlier context and every possible right context.
                self.prefix.extend([CompensatedSum::default(); 2]);
                let mut sum = CompensatedSum::default();
                for pair in self.samples.windows(2) {
                    sum.push((pair[1] - pair[0]).powi(2));
                    self.prefix.push(sum);
                }
            }
            let prefix = &self.prefix;
            // Bounds follow from full left context and reserved right context.
            let before = prefix[start]
                .difference(prefix[start - self.context])
                .max(0.0)
                / self.context as f64;
            if edge * edge < EDGE_RATIO.powi(2) * before {
                continue;
            }
            for stop in start + 1..=(start + self.max_width).min(last_stop) {
                let exit = self.samples[stop] - self.samples[stop - 1];
                if edge * exit >= 0.0
                    || exit.abs() < MIN_JUMP
                    || (edge + exit).abs() > RETURN_FRACTION * edge.abs().max(exit.abs())
                {
                    continue;
                }
                let after = prefix[stop + self.context + 1]
                    .difference(prefix[stop + 1])
                    .max(0.0)
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
