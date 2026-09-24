//! High-frequency stereo-width measurement.
//!
//! Intensity stereo can preserve a high-frequency energy envelope while
//! removing much of the Side signal. This meter never claims to identify the
//! codec tool that caused such a pattern: recordings may intentionally narrow
//! in the high band. It reports a conservative cue only when the upper-mid
//! reference band is demonstrably wide and the high band is persistently not.
//!
//! # Size
//!
//! This is one streaming signal path: the matched Mid/Side filters, block
//! eligibility and final decision must be read together to review what can
//! create a narrowing cue. Splitting its filter stages from its thresholds
//! would scatter that single argument across modules without creating a second
//! reusable subsystem.

use serde::{Deserialize, Serialize};

// These are experimental project heuristics, not codec/specification limits.
// -20 dB is 1% Side/Mid energy; the reference permits at least 6.3%, with
// a further 12 dB contrast. 500 ms / 70% suppress isolated changes. These
// choices still need calibration on independently labelled music corpora.
/// Lower corner of the high-frequency measurement band.
const HIGH_BAND_HZ: f64 = 6_000.0;
/// Bound the measurement to audible treble rather than DSD/hi-res ultrasound.
const HIGH_LIMIT_HZ: f64 = 20_000.0;
/// Reference band used to establish that the programme is meaningfully stereo.
const REFERENCE_LOW_HZ: f64 = 1_500.0;
const REFERENCE_HIGH_HZ: f64 = 5_000.0;
/// Each independent decision covers half a second of audio.
const BLOCK_SECS: f64 = 0.5;
/// Require enough duration to distinguish a persistent pattern from one hit.
const MIN_ELIGIBLE_BLOCKS: u32 = 3;
/// Both Mid bands must exceed this RMS floor before their Side ratio is useful.
const MID_FLOOR_DBFS: f64 = -60.0;
/// A high-band Side signal this far below Mid is effectively collapsed.
const COLLAPSED_SIDE_DB: f64 = -20.0;
/// The reference band must carry enough Side energy to show that the signal
/// was not simply narrow at every frequency.
const WIDE_REFERENCE_DB: f64 = -12.0;
/// The high-band reduction must also be substantial relative to that reference.
const NARROWING_GAP_DB: f64 = 12.0;
/// At least this share of eligible blocks must show the pattern.
const PERSISTENT_FRACTION: f64 = 0.7;

/// Measured high-frequency stereo width and the evidence used to interpret it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HighFrequencyStereo {
    /// Side-to-Mid energy ratio over eligible complete blocks in the nominal
    /// 6–20 kHz band (limited by Nyquist), floored at -120 dB.
    pub side_to_mid_db: f32,
    /// Equivalent ratio in the 1.5–5 kHz reference band.
    pub reference_side_to_mid_db: f32,
    /// Fraction of eligible half-second blocks that met the narrowing rule.
    pub narrowed_block_fraction: f32,
    /// A persistent high-band narrowing cue, not a codec or provenance verdict.
    pub narrowed: bool,
}

/// Direct-form biquad used for the Butterworth low-pass/high-pass sections.
#[derive(Clone, Copy)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    fn new(cutoff_hz: f64, q: f64, sample_rate: u32, high_pass: bool) -> Self {
        let omega = std::f64::consts::TAU * cutoff_hz / sample_rate as f64;
        let cosine = omega.cos();
        let alpha = omega.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        // W3C Audio EQ Cookbook, LPF/HPF equations:
        // https://www.w3.org/TR/audio-eq-cookbook/
        let sign = if high_pass { -1.0 } else { 1.0 };
        let b0 = (1.0 - sign * cosine) / (2.0 * a0);
        Self {
            b0,
            b1: sign * 2.0 * b0,
            b2: b0,
            a1: -2.0 * cosine / a0,
            a2: (1.0 - alpha) / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn push(&mut self, input: f64) -> f64 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }
}

/// Eighth-order Butterworth cascade. Matching Mid/Side filters preserve
/// their ratio; steep skirts reduce leakage from neighbouring bands.
struct Butterworth {
    stages: [Biquad; 4],
}

impl Butterworth {
    fn new(cutoff_hz: f64, sample_rate: u32, high_pass: bool) -> Self {
        // Q = 1 / (2 cos((2k + 1) pi / 16)), k = 0..3.
        const Q: [f64; 4] = [0.509_795_579, 0.601_344_887, 0.899_976_224, 2.562_915_448];
        Self {
            stages: Q.map(|q| Biquad::new(cutoff_hz, q, sample_rate, high_pass)),
        }
    }

    fn push(&mut self, mut input: f64) -> f64 {
        for stage in &mut self.stages {
            input = stage.push(input);
        }
        input
    }
}

struct BandPass {
    high_pass: Butterworth,
    low_pass: Option<Butterworth>,
}

impl BandPass {
    fn new(low_hz: f64, high_hz: f64, sample_rate: u32) -> Self {
        Self {
            high_pass: Butterworth::new(low_hz, sample_rate, true),
            // A low-pass at/above Nyquist is unnecessary and degenerate.
            low_pass: (high_hz < sample_rate as f64 * 0.5)
                .then(|| Butterworth::new(high_hz, sample_rate, false)),
        }
    }

    fn push(&mut self, input: f64) -> f64 {
        let output = self.high_pass.push(input);
        self.low_pass
            .as_mut()
            .map_or(output, |filter| filter.push(output))
    }
}

/// Streaming high-frequency stereo-width meter for a valid stereo stream.
pub struct HighFrequencyStereoMeter {
    high_mid: BandPass,
    high_side: BandPass,
    reference_mid: BandPass,
    reference_side: BandPass,
    block_frames: u64,
    frames_in_block: u64,
    high_mid_energy: f64,
    high_side_energy: f64,
    reference_mid_energy: f64,
    reference_side_energy: f64,
    total_high_mid_energy: f64,
    total_high_side_energy: f64,
    total_reference_mid_energy: f64,
    total_reference_side_energy: f64,
    eligible_blocks: u32,
    narrowed_blocks: u32,
    valid: bool,
}

impl HighFrequencyStereoMeter {
    /// Start a meter for exactly two channels at a rate with a usable high band.
    pub fn new(sample_rate: u32, channels: usize) -> Option<Self> {
        // A 6 kHz high band needs enough space above it to distinguish it from
        // the reference band. Above 768 kHz the rest of the streaming analyzer
        // already declines comparable measurements for bounded memory reasons.
        if channels != 2 || !(24_000..=768_000).contains(&sample_rate) {
            return None;
        }
        let block_frames = (sample_rate as f64 * BLOCK_SECS).round() as u64;
        Some(Self {
            high_mid: BandPass::new(HIGH_BAND_HZ, HIGH_LIMIT_HZ, sample_rate),
            high_side: BandPass::new(HIGH_BAND_HZ, HIGH_LIMIT_HZ, sample_rate),
            reference_mid: BandPass::new(REFERENCE_LOW_HZ, REFERENCE_HIGH_HZ, sample_rate),
            reference_side: BandPass::new(REFERENCE_LOW_HZ, REFERENCE_HIGH_HZ, sample_rate),
            block_frames: block_frames.max(1),
            frames_in_block: 0,
            high_mid_energy: 0.0,
            high_side_energy: 0.0,
            reference_mid_energy: 0.0,
            reference_side_energy: 0.0,
            total_high_mid_energy: 0.0,
            total_high_side_energy: 0.0,
            total_reference_mid_energy: 0.0,
            total_reference_side_energy: 0.0,
            eligible_blocks: 0,
            narrowed_blocks: 0,
            valid: true,
        })
    }

    /// Feed one left/right frame. Invalid input withholds the whole result;
    /// publishing only its earlier prefix would misrepresent a file-wide cue.
    pub fn push_frame(&mut self, samples: &[f32]) {
        if samples.len() != 2 || samples.iter().any(|sample| !sample.is_finite()) {
            self.valid = false;
            return;
        }
        if !self.valid {
            return;
        }

        let mid = (samples[0] as f64 + samples[1] as f64) * 0.5;
        let side = (samples[0] as f64 - samples[1] as f64) * 0.5;
        let high_mid = self.high_mid.push(mid);
        let high_side = self.high_side.push(side);
        let reference_mid = self.reference_mid.push(mid);
        let reference_side = self.reference_side.push(side);
        self.high_mid_energy += high_mid * high_mid;
        self.high_side_energy += high_side * high_side;
        self.reference_mid_energy += reference_mid * reference_mid;
        self.reference_side_energy += reference_side * reference_side;
        self.frames_in_block += 1;

        if self.frames_in_block == self.block_frames {
            self.finish_block();
        }
    }

    fn finish_block(&mut self) {
        if self.frames_in_block == 0 {
            return;
        }
        let frames = self.frames_in_block as f64;
        let high_mid_rms = (self.high_mid_energy / frames).sqrt();
        let reference_mid_rms = (self.reference_mid_energy / frames).sqrt();
        let min_mid_rms = 10f64.powf(MID_FLOOR_DBFS / 20.0);
        if high_mid_rms > min_mid_rms && reference_mid_rms > min_mid_rms {
            let high_ratio = ratio_db(self.high_side_energy, self.high_mid_energy);
            let reference_ratio = ratio_db(self.reference_side_energy, self.reference_mid_energy);
            if high_ratio.is_finite() && reference_ratio.is_finite() {
                self.eligible_blocks += 1;
                self.total_high_mid_energy += self.high_mid_energy;
                self.total_high_side_energy += self.high_side_energy;
                self.total_reference_mid_energy += self.reference_mid_energy;
                self.total_reference_side_energy += self.reference_side_energy;
                if is_narrowed(high_ratio, reference_ratio) {
                    self.narrowed_blocks += 1;
                }
            }
        }
        self.frames_in_block = 0;
        self.high_mid_energy = 0.0;
        self.high_side_energy = 0.0;
        self.reference_mid_energy = 0.0;
        self.reference_side_energy = 0.0;
    }

    /// Return the measurement aggregated over eligible complete blocks. A trailing partial block is not
    /// used: a half-filled excerpt can otherwise turn a short fade into an
    /// apparent persistent change of stereo width.
    pub fn finish(self) -> Option<HighFrequencyStereo> {
        if !self.valid || self.eligible_blocks < MIN_ELIGIBLE_BLOCKS {
            return None;
        }
        let side_to_mid_db = ratio_db(self.total_high_side_energy, self.total_high_mid_energy);
        let reference_side_to_mid_db = ratio_db(
            self.total_reference_side_energy,
            self.total_reference_mid_energy,
        );
        if !side_to_mid_db.is_finite() || !reference_side_to_mid_db.is_finite() {
            return None;
        }
        let narrowed_block_fraction = self.narrowed_blocks as f64 / self.eligible_blocks as f64;
        Some(HighFrequencyStereo {
            side_to_mid_db: side_to_mid_db as f32,
            reference_side_to_mid_db: reference_side_to_mid_db as f32,
            narrowed_block_fraction: narrowed_block_fraction as f32,
            narrowed: narrowed_block_fraction >= PERSISTENT_FRACTION
                && is_narrowed(side_to_mid_db, reference_side_to_mid_db),
        })
    }
}

fn ratio_db(numerator: f64, denominator: f64) -> f64 {
    // A relative -120 dB reporting floor keeps zero Side finite and invariant
    // under gain/duration changes; it is not a measured noise floor.
    10.0 * (numerator / denominator).max(1e-12).log10()
}

fn is_narrowed(high_ratio_db: f64, reference_ratio_db: f64) -> bool {
    high_ratio_db <= COLLAPSED_SIDE_DB
        && reference_ratio_db >= WIDE_REFERENCE_DB
        && reference_ratio_db - high_ratio_db >= NARROWING_GAP_DB
}

#[cfg(test)]
#[path = "../../tests/unit/analysis/intensity_stereo.rs"]
mod tests;
