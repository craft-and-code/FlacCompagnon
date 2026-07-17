//! True-peak (inter-sample peak) measurement, ITU-R BS.1770-style.
//!
//! Digital samples are points on a continuous waveform: between two samples the
//! reconstructed analog signal can swing *above* both of them. A track whose
//! sample peak reads −0.2 dBFS can therefore clip a DAC's reconstruction filter
//! at +1 dBTP. BS.1770-4 measures this by upsampling ×4 with a 48-tap lowpass
//! FIR and taking the peak of the oversampled stream — exactly what this module
//! does, as a streaming polyphase filter (12 multiplies per phase, 4 phases per
//! input sample, per channel).
//!
//! The filter is a windowed-sinc (Blackman) lowpass with cutoff at the input
//! Nyquist, unity gain per phase. Validated against an independent NumPy
//! implementation: a 0.98 full-scale quarter-rate sine sampled at its
//! zero-crossing offset reads 0.693 sample peak but 0.9714 true peak
//! (recovering the hidden crest), and hard-clipped material reads > 1.0
//! (an inter-sample "over").

/// 4× oversampling factor (BS.1770-4 recommends ≥ 4x for ≤ 96 kHz rates).
pub const TP_FACTOR: usize = 4;
/// Taps per polyphase branch (48 total / 4 phases).
const PHASE_LEN: usize = 12;

/// 48-tap windowed-sinc lowpass, generated and validated in NumPy
/// (`sinc(n/4) * blackman(48)`, normalized to unity DC gain per phase).
const TP_TAPS: [f32; 48] = [
    2.8770393805e-19, -8.4620792140e-05, -3.6092137667e-04, -3.6337902617e-04,
    7.0568670835e-04, 2.9338012272e-03, 4.6865622501e-03, 2.9440137922e-03,
    -4.2954750165e-03, -1.4678888264e-02, -2.0272630732e-02, -1.1363293682e-02,
    1.5119076534e-02, 4.7897022893e-02, 6.2173748295e-02, 3.3175379005e-02,
    -4.2571243044e-02, -1.3200353510e-01, -1.7082947420e-01, -9.3211746888e-02,
    1.2718490344e-01, 4.4935166808e-01, 7.7127787555e-01, 9.7258547035e-01,
    9.7258547035e-01, 7.7127787555e-01, 4.4935166808e-01, 1.2718490344e-01,
    -9.3211746888e-02, -1.7082947420e-01, -1.3200353510e-01, -4.2571243044e-02,
    3.3175379005e-02, 6.2173748295e-02, 4.7897022893e-02, 1.5119076534e-02,
    -1.1363293682e-02, -2.0272630732e-02, -1.4678888264e-02, -4.2954750165e-03,
    2.9440137922e-03, 4.6865622501e-03, 2.9338012272e-03, 7.0568670835e-04,
    -3.6337902617e-04, -3.6092137667e-04, -8.4620792140e-05, 2.8770393805e-19,
];

/// One channel's delay line.
struct Channel {
    /// Last `PHASE_LEN` input samples, newest first.
    delay: [f32; PHASE_LEN],
}

/// Streaming multi-channel true-peak meter.
pub struct TruePeak {
    channels: Vec<Channel>,
    peak: f32,
}

impl TruePeak {
    pub fn new(channels: usize) -> Self {
        Self {
            channels: (0..channels.max(1))
                .map(|_| Channel {
                    delay: [0.0; PHASE_LEN],
                })
                .collect(),
            peak: 0.0,
        }
    }

    /// Feed one frame (one sample per channel).
    pub fn push_frame(&mut self, samples: &[f32]) {
        for (ch, &s) in self.channels.iter_mut().zip(samples.iter()) {
            // Shift the delay line (newest sample at index 0).
            ch.delay.copy_within(0..PHASE_LEN - 1, 1);
            ch.delay[0] = s;
            // Four oversampled outputs: phase p uses taps p, p+4, p+8, ...
            // With the newest sample first, tap index k*4+p multiplies delay[k].
            for p in 0..TP_FACTOR {
                let mut acc = 0.0f32;
                for k in 0..PHASE_LEN {
                    acc += TP_TAPS[k * TP_FACTOR + p] * ch.delay[k];
                }
                let a = acc.abs();
                if a > self.peak {
                    self.peak = a;
                }
            }
        }
    }

    /// Highest oversampled magnitude seen so far (linear, can exceed 1.0).
    pub fn peak(&self) -> f32 {
        self.peak
    }

    /// True peak in dBTP (0.0 == full scale; positive == inter-sample over).
    pub fn peak_dbtp(&self) -> f32 {
        if self.peak > 0.0 {
            20.0 * self.peak.log10()
        } else {
            f32::NEG_INFINITY
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Quarter-rate sine at phase π/4: samples sit at ±0.693 but the true crest
    /// is 0.98. NumPy ground truth for this filter: 0.97135.
    #[test]
    fn recovers_inter_sample_peak() {
        let fs = 48_000usize;
        let mut tp = TruePeak::new(1);
        let mut sample_peak = 0.0f32;
        for n in 0..fs {
            // A quarter-rate sine is exactly periodic every 4 samples, so its
            // phase can be computed from n % 4 instead of the raw n * rate
            // formula. That matters here: for a full second at 48 kHz the raw
            // formula's angle argument grows past 75 000 radians, and f32
            // loses enough precision at that magnitude to visibly perturb
            // sin() (this was the actual cause of a prior test flake, not the
            // filter under test). Wrapping keeps the angle small and exact.
            let phase = std::f32::consts::FRAC_PI_2 * (n % 4) as f32 + std::f32::consts::FRAC_PI_4;
            let x = 0.98 * phase.sin();
            sample_peak = sample_peak.max(x.abs());
            tp.push_frame(&[x]);
        }
        assert!(sample_peak < 0.70, "sample peak {sample_peak}");
        let peak = tp.peak();
        assert!(
            (peak - 0.97135).abs() < 0.002,
            "true peak {peak}, expected ≈0.97135"
        );
    }

    /// Hard-clipped sine: reconstruction overshoots full scale (true peak > 1),
    /// NumPy ground truth 1.01199.
    #[test]
    fn clipped_material_reads_over() {
        let fs = 48_000usize;
        let mut tp = TruePeak::new(1);
        for n in 0..fs {
            let x = (1.4 * (2.0 * std::f32::consts::PI * 997.0 * n as f32 / fs as f32).sin())
                .clamp(-1.0, 1.0);
            tp.push_frame(&[x]);
        }
        let peak = tp.peak();
        assert!(
            (peak - 1.01199).abs() < 0.002,
            "true peak {peak}, expected ≈1.01199"
        );
        assert!(tp.peak_dbtp() > 0.0, "dBTP {} should be positive", tp.peak_dbtp());
    }

    /// Benign smooth material: true peak stays close to the sample peak
    /// (no phantom overshoot from the filter itself).
    #[test]
    fn benign_material_matches_sample_peak() {
        let fs = 48_000usize;
        let mut tp = TruePeak::new(2);
        let mut sample_peak = 0.0f32;
        for n in 0..fs {
            // 100 Hz sine, samples land densely on the crest.
            let x = 0.5 * (2.0 * std::f32::consts::PI * 100.0 * n as f32 / fs as f32).sin();
            sample_peak = sample_peak.max(x.abs());
            tp.push_frame(&[x, x]);
        }
        let ratio = tp.peak() / sample_peak;
        assert!((0.98..=1.05).contains(&ratio), "ratio {ratio}");
    }
}
