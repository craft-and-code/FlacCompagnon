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
    2.8770394e-19,
    -8.462079e-5,
    -3.6092138e-4,
    -3.6337903e-4,
    7.056867e-4,
    2.9338012e-3,
    4.6865623e-3,
    2.9440138e-3,
    -4.295475e-3,
    -1.4678888e-2,
    -2.0272631e-2,
    -1.1363294e-2,
    1.5119077e-2,
    4.7897023e-2,
    6.2173748e-2,
    3.317538e-2,
    -4.2571243e-2,
    -1.3200353e-1,
    -1.7082947e-1,
    -9.321175e-2,
    1.271849e-1,
    4.4935167e-1,
    7.712779e-1,
    9.725855e-1,
    9.725855e-1,
    7.712779e-1,
    4.4935167e-1,
    1.271849e-1,
    -9.321175e-2,
    -1.7082947e-1,
    -1.3200353e-1,
    -4.2571243e-2,
    3.317538e-2,
    6.2173748e-2,
    4.7897023e-2,
    1.5119077e-2,
    -1.1363294e-2,
    -2.0272631e-2,
    -1.4678888e-2,
    -4.295475e-3,
    2.9440138e-3,
    4.6865623e-3,
    2.9338012e-3,
    7.056867e-4,
    -3.6337903e-4,
    -3.6092138e-4,
    -8.462079e-5,
    2.8770394e-19,
];

/// One channel's delay line.
struct Channel {
    /// Last `PHASE_LEN` input samples, newest first.
    delay: [f32; PHASE_LEN],
}

/// Streaming multi-channel true-peak meter.
///
/// Feed it frames as they are decoded, then read [`TruePeak::peak_dbtp`].
/// A positive value is an inter-sample "over": the waveform a DAC reconstructs
/// exceeds full scale even though no stored sample does.
///
/// ```
/// use flaccompagnon_core::analysis::truepeak::TruePeak;
///
/// // A quarter-rate sine sampled at its zero-crossing offset: every stored
/// // sample sits at ±0.693, but the real crest between them is 0.98.
/// let mut tp = TruePeak::new(1);
/// let mut sample_peak: f32 = 0.0;
/// for n in 0..4096 {
///     let phase = std::f32::consts::FRAC_PI_2 * (n % 4) as f32
///         + std::f32::consts::FRAC_PI_4;
///     let x = 0.98 * phase.sin();
///     sample_peak = sample_peak.max(x.abs());
///     tp.push_frame(&[x]);
/// }
///
/// assert!(sample_peak < 0.70);          // what a naive peak meter reports
/// assert!(tp.peak() > 0.95);            // what the signal actually reaches
/// assert!(tp.peak_dbtp() < 0.0);        // still below full scale here
/// ```
pub struct TruePeak {
    channels: Vec<Channel>,
    peak: f32,
}

impl TruePeak {
    /// Start tracking oversampled peak for a stream with `channels` audio
    /// channels (at least one channel is always allocated).
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
#[path = "../../tests/unit/analysis/truepeak.rs"]
mod tests;
