//! Abrupt digital silence bounded by signal; not a general silence detector.

use super::{context_frames, DiscontinuityEvent, EventSummary};

// Project thresholds: both boundaries must jump by >=0.02 full scale (~−34
// dBFS). Context must exceed −60 dBFS RMS, and the resumed signal must stay
// within 12 dB of the preceding RMS. These conservative checks exclude fades,
// leading/trailing silence, and most unrelated starts/stops.
const MIN_EDGE: f64 = 0.02;
const MIN_POWER: f64 = 1e-6;
const CONTEXT_POWER_RATIO: f64 = 16.0;

struct Gap {
    start: u64,
    before_power: f64,
}

struct Resumption {
    gap: Gap,
    duration: u64,
    power: f64,
    nonzero: usize,
    frames: usize,
}

pub(super) struct DropoutDetector {
    history: Vec<f64>,
    at: usize,
    power: f64,
    previous: f64,
    frames: u64,
    gap: Option<Gap>,
    resumption: Option<Resumption>,
    rate: u32,
    channel: u16,
}

impl DropoutDetector {
    pub(super) fn new(rate: u32, channel: u16) -> Self {
        Self {
            history: vec![0.0; context_frames(rate)],
            at: 0,
            power: 0.0,
            previous: 0.0,
            frames: 0,
            gap: None,
            resumption: None,
            rate,
            channel,
        }
    }

    pub(super) fn push(&mut self, sample: f64, output: &mut EventSummary) {
        let context = self.history.len();
        if sample == 0.0 && self.previous != 0.0 {
            let before_power = self.power.max(0.0) / context as f64;
            if self.frames >= context as u64
                && self.previous.abs() >= MIN_EDGE
                && before_power >= MIN_POWER
            {
                self.gap = Some(Gap {
                    start: self.frames,
                    before_power,
                });
            }
        } else if sample != 0.0 {
            if let Some(gap) = self.gap.take() {
                let duration = self.frames - gap.start;
                // 2–250 ms excludes zero crossings and long musical pauses.
                if duration >= (self.rate as u64).div_ceil(500)
                    && duration <= self.rate as u64 / 4
                    && sample.abs() >= MIN_EDGE
                {
                    self.resumption = Some(Resumption {
                        gap,
                        duration,
                        power: 0.0,
                        nonzero: 0,
                        frames: 0,
                    });
                }
            }
        }
        if let Some(after) = &mut self.resumption {
            after.power += sample * sample;
            after.nonzero += usize::from(sample != 0.0);
            after.frames += 1;
            if after.frames == context {
                let power = after.power / context as f64;
                if power >= MIN_POWER
                    && after.nonzero * 10 >= context * 9
                    && power >= after.gap.before_power / CONTEXT_POWER_RATIO
                    && power <= after.gap.before_power * CONTEXT_POWER_RATIO
                {
                    output.record(DiscontinuityEvent {
                        channel: self.channel,
                        start_secs: after.gap.start as f64 / self.rate as f64,
                        duration_secs: after.duration as f64 / self.rate as f64,
                    });
                }
                self.resumption = None;
            }
        }
        self.power += sample * sample - self.history[self.at];
        self.history[self.at] = sample * sample;
        self.at = (self.at + 1) % context;
        self.previous = sample;
        self.frames += 1;
    }
}
