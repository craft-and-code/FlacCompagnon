//! Integrated loudness for mono and stereo audio, per ITU-R BS.1770-5.
//!
//! The meter keeps 400 ms of K-weighted power and one power value per 100 ms
//! gate. It does not retain decoded samples. Channel positions are required
//! for a correct multichannel measurement, so unsupported layouts yield no
//! value rather than an incorrectly weighted LUFS number.

/// ITU-R BS.1770-5, Annex 1, Tables 1 and 2 (48 kHz reference filters).
const SHELF_48K: ([f64; 3], [f64; 3]) = (
    [
        1.535_124_859_586_97,
        -2.691_696_189_406_38,
        1.198_392_810_852_85,
    ],
    [1.0, -1.690_659_293_182_41, 0.732_480_774_215_85],
);
const HIGH_PASS_48K: ([f64; 3], [f64; 3]) = (
    [1.0, -2.0, 1.0],
    [1.0, -1.990_047_454_833_98, 0.990_072_250_366_21],
);

/// ITU-R BS.1770-5, Annex 1, equations 2 and 6.
const LOUDNESS_OFFSET: f64 = -0.691;
const ABSOLUTE_GATE_LUFS: f64 = -70.0;
const RELATIVE_GATE_LU: f64 = 10.0;

#[derive(Clone, Copy)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 3],
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    fn new(reference: ([f64; 3], [f64; 3]), sample_rate: u32) -> Self {
        // Invert the bilinear transform at the specified 48 kHz reference,
        // then apply it at the stream's rate. The resulting digital response
        // equals the published coefficients at 48 kHz and tracks the same
        // analogue frequency response at 44.1, 96, 192 kHz, etc.
        fn retune(c: [f64; 3], ratio: f64) -> [f64; 3] {
            let c0 = c[0] + c[1] + c[2];
            let c1 = c[0] - c[2];
            let c2 = c[0] - c[1] + c[2];
            [
                c0 + 2.0 * ratio * c1 + ratio * ratio * c2,
                2.0 * (c0 - ratio * ratio * c2),
                c0 - 2.0 * ratio * c1 + ratio * ratio * c2,
            ]
        }
        let ratio = sample_rate as f64 / 48_000.0;
        let b = retune(reference.0, ratio);
        let a = retune(reference.1, ratio);
        let scale = a[0];
        Self {
            b: b.map(|v| v / scale),
            a: [1.0, a[1] / scale, a[2] / scale],
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn push(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.b[1] * self.x1 + self.b[2] * self.x2
            - self.a[1] * self.y1
            - self.a[2] * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

struct ChannelFilter {
    shelf: Biquad,
    high_pass: Biquad,
}

impl ChannelFilter {
    fn new(sample_rate: u32) -> Self {
        Self {
            shelf: Biquad::new(SHELF_48K, sample_rate),
            high_pass: Biquad::new(HIGH_PASS_48K, sample_rate),
        }
    }

    fn push(&mut self, sample: f32) -> f64 {
        let weighted = self.high_pass.push(self.shelf.push(sample as f64));
        weighted * weighted
    }
}

/// Streaming EBU R 128 integrated-loudness meter for mono or stereo PCM.
pub struct LoudnessMeter {
    filters: Vec<ChannelFilter>,
    power_ring: Vec<f64>,
    ring_at: usize,
    window_power: f64,
    frames: u64,
    step_frames: u64,
    block_powers: Vec<f64>,
}

impl LoudnessMeter {
    /// Create a meter for a stream. The channel count is limited to one or two
    /// until decode paths can supply reliable speaker positions and LFE roles.
    pub fn new(sample_rate: u32, channels: usize) -> Option<Self> {
        // 768 kHz bounds the 400 ms ring to 307,200 f64 values (~2.5 MB) even
        // for a hostile header; below 8 kHz K-weighting loses useful bandwidth.
        if !(1..=2).contains(&channels) || !(8_000..=768_000).contains(&sample_rate) {
            return None;
        }
        // ITU-R BS.1770-5 Annex 1: 400 ms gates with 75% overlap, rounded to
        // whole samples. At common rates both lengths are exact integers.
        let block_frames = (sample_rate as f64 * 0.4).round() as usize;
        let step_frames = (sample_rate as f64 * 0.1).round() as u64;
        Some(Self {
            filters: (0..channels)
                .map(|_| ChannelFilter::new(sample_rate))
                .collect(),
            power_ring: vec![0.0; block_frames],
            ring_at: 0,
            window_power: 0.0,
            frames: 0,
            step_frames,
            block_powers: Vec::new(),
        })
    }

    /// Push a complete decoded frame, with one normalized sample per channel.
    pub fn push_frame(&mut self, samples: &[f32]) {
        if samples.len() != self.filters.len() {
            return;
        }
        let power: f64 = self
            .filters
            .iter_mut()
            .zip(samples)
            .map(|(filter, &sample)| filter.push(sample))
            .sum();
        self.window_power += power - self.power_ring[self.ring_at];
        self.power_ring[self.ring_at] = power;
        self.ring_at = (self.ring_at + 1) % self.power_ring.len();
        self.frames += 1;

        let block_frames = self.power_ring.len() as u64;
        if self.frames >= block_frames
            && (self.frames - block_frames).is_multiple_of(self.step_frames)
        {
            self.block_powers
                .push((self.window_power / block_frames as f64).max(0.0));
        }
    }

    /// Return the absolute- and relative-gated programme loudness in LUFS.
    /// Silence and files shorter than one 400 ms gate have no reading.
    pub fn integrated_lufs(&self) -> Option<f32> {
        let absolute_power = 10f64.powf((ABSOLUTE_GATE_LUFS - LOUDNESS_OFFSET) / 10.0);
        let (sum, count) = self
            .block_powers
            .iter()
            .filter(|&&power| power > absolute_power)
            .fold((0.0, 0usize), |(sum, count), &power| {
                (sum + power, count + 1)
            });
        if count == 0 {
            return None;
        }
        let relative_power = (sum / count as f64) / 10f64.powf(RELATIVE_GATE_LU / 10.0);
        let (sum, count) = self
            .block_powers
            .iter()
            .filter(|&&power| power > absolute_power && power > relative_power)
            .fold((0.0, 0usize), |(sum, count), &power| {
                (sum + power, count + 1)
            });
        if count == 0 {
            return None;
        }
        Some((LOUDNESS_OFFSET + 10.0 * (sum / count as f64).log10()) as f32)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/analysis/loudness.rs"]
mod tests;
