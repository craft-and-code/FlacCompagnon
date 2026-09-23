//! Conservative, read-only discontinuity heuristics on each decoded channel.
//!
//! Clicks are isolated short pulses; dropouts are abrupt, bounded runs of
//! exact digital silence surrounded by signal. These are listening cues,
//! not proof of damaged audio: intentional edits can have the same shape.
//! Thresholds are project choices, covered by synthetic injection/control
//! tests, not an audio standard or a calibrated probability of error.

mod clicks;
mod dropouts;

use serde::{Deserialize, Serialize};

/// Bound report size even on pathological files; counts remain exhaustive.
const MAX_LOCATIONS: usize = 32;

fn context_frames(rate: u32) -> usize {
    (rate as usize).div_ceil(200)
}
fn pulse_frames(rate: u32) -> usize {
    (rate as usize / 2_000).max(1)
}

/// One suspected event on one channel, relative to the beginning of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscontinuityEvent {
    /// One-based decoded channel index; 1/2 mean left/right for stereo.
    pub channel: u16,
    /// Event start in seconds.
    pub start_secs: f64,
    /// Candidate pulse or silent-run duration in seconds.
    pub duration_secs: f64,
}

/// Count of channel events, with a bounded list of the earliest locations.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EventSummary {
    /// Simultaneous events on two channels count twice.
    pub count: u64,
    /// Earliest 32 events, sorted by time then channel. Empty when count is zero.
    pub events: Vec<DiscontinuityEvent>,
}

impl EventSummary {
    fn record(&mut self, event: DiscontinuityEvent) {
        self.count += 1;
        // Channels finish their buffered windows separately. Keep the earliest
        // events globally, rather than filling the cap with the first channel.
        let at = self.events.partition_point(|old| {
            (old.start_secs, old.channel) <= (event.start_secs, event.channel)
        });
        if at < MAX_LOCATIONS {
            self.events.insert(at, event);
            self.events.truncate(MAX_LOCATIONS);
        }
    }
}

/// Suspected discontinuities; zero counts mean no candidates met the criteria.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiscontinuityAnalysis {
    /// Short, isolated pulse candidates, counted separately per channel.
    pub clicks: EventSummary,
    /// Abrupt digital-silence candidates, counted separately per channel.
    pub dropouts: EventSummary,
}

struct Channel {
    clicks: clicks::ClickDetector,
    dropouts: dropouts::DropoutDetector,
}

/// Bounded-memory streaming detector with about 5 ms of context on each side.
pub struct DiscontinuityDetector {
    channels: Vec<Channel>,
    result: DiscontinuityAnalysis,
    frames: u64,
    min_frames: u64,
    valid: bool,
}

impl DiscontinuityDetector {
    /// Support 8–768 kHz and 1–32 channels. Bounds limit allocations made from
    /// untrusted container headers; an unsupported stream has no reading.
    pub fn new(sample_rate: u32, channels: usize) -> Option<Self> {
        if !(8_000..=768_000).contains(&sample_rate) || !(1..=32).contains(&channels) {
            return None;
        }
        let context = context_frames(sample_rate);
        let max_pulse = pulse_frames(sample_rate);
        Some(Self {
            channels: (1..=channels)
                .map(|channel| Channel {
                    clicks: clicks::ClickDetector::new(sample_rate, channel as u16),
                    dropouts: dropouts::DropoutDetector::new(sample_rate, channel as u16),
                })
                .collect(),
            result: DiscontinuityAnalysis::default(),
            frames: 0,
            min_frames: (2 * context + max_pulse + 2) as u64,
            valid: true,
        })
    }

    /// Feed one complete frame. A malformed frame invalidates the whole
    /// reading, rather than silently presenting a count from a valid prefix.
    pub fn push_frame(&mut self, samples: &[f32]) {
        if samples.len() != self.channels.len() || samples.iter().any(|s| !s.is_finite()) {
            self.valid = false;
        }
        if !self.valid {
            return;
        }
        for (channel, &sample) in self.channels.iter_mut().zip(samples) {
            channel.clicks.push(sample as f64, &mut self.result.clicks);
            channel
                .dropouts
                .push(sample as f64, &mut self.result.dropouts);
        }
        self.frames += 1;
    }

    /// Flush candidates with full real context; never pad file boundaries
    /// with silence, which would manufacture apparent transitions.
    pub fn finish(mut self) -> Option<DiscontinuityAnalysis> {
        if !self.valid || self.frames < self.min_frames {
            return None;
        }
        for channel in &mut self.channels {
            channel.clicks.finish(&mut self.result.clicks);
        }
        Some(self.result)
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/analysis/discontinuities.rs"]
mod tests;
