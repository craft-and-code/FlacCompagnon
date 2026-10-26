//! Bounded, streaming STFT: every source frame is consumed at its native rate.

mod transform;

use crate::{Aggregation, AnalysisOptions, Error, FrequencyScale, SignalInfo};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

/// Measured linear powers and the exact axes used to produce them.
#[derive(Debug)]
pub struct Spectrogram {
    pub(crate) info: SignalInfo,
    pub(crate) options: AnalysisOptions,
    pub(crate) fft_size: usize,
    pub(crate) hop_size: usize,
    pub(crate) max_frequency: f64,
    pub(crate) power: Vec<f32>,
}

impl Spectrogram {
    /// Native decoded source properties.
    pub fn signal(&self) -> SignalInfo {
        self.info
    }
    /// Actual transform length, including an automatically selected length.
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }
    /// Actual distance between FFT centers in source frames.
    pub fn hop_size(&self) -> usize {
        self.hop_size
    }
    /// Plotted dimensions, excluding labels and margins.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.options.width, self.options.height)
    }
    /// Duration calculated from decoded complete frames.
    pub fn duration(&self) -> f64 {
        self.info.frames as f64 / f64::from(self.info.sample_rate)
    }
    /// Peak-amplitude-referenced dBFS/bin at a pixel, with row zero at the bottom.
    /// Silence is negative infinity; coordinates outside the plot return `None`.
    pub fn level_db(&self, column: u32, row: u32) -> Option<f64> {
        if column >= self.options.width || row >= self.options.height {
            return None;
        }
        self.power
            .get(row as usize * self.options.width as usize + column as usize)
            .map(|&value| 10.0 * f64::from(value).log10())
    }
    pub(crate) fn fraction(&self, frequency: f64) -> f64 {
        let min = self.options.min_frequency;
        match self.options.frequency_scale {
            FrequencyScale::Linear => (frequency - min) / (self.max_frequency - min),
            FrequencyScale::Logarithmic => (frequency / min).ln() / (self.max_frequency / min).ln(),
        }
    }
}

/// Stream interleaved PCM into fixed-size spectral buckets, retaining one window.
pub struct SpectrogramBuilder {
    result: Spectrogram,
    received: u64,
    next_center: u64,
    last_center: Option<u64>,
    ring: Vec<f32>,
    window: Vec<f64>,
    window_sum: f64,
    fft: Arc<dyn Fft<f64>>,
    buffer: Vec<Complex<f64>>,
    scratch: Vec<Complex<f64>>,
    bins: Vec<f64>,
    rows: Vec<f64>,
    row_counts: Vec<u32>,
    bin_rows: Vec<Option<usize>>,
    valid_bins: (usize, usize),
    counts: Vec<u32>,
}

impl SpectrogramBuilder {
    /// Validate all dimensions before allocating; source length must be exact.
    pub fn new(info: SignalInfo, options: AnalysisOptions) -> Result<Self, Error> {
        let (fft_size, hop_size, max_frequency) = options.validate(info)?;
        let height = options.height as usize;
        let pixels = options.width as usize * height;
        let mean = matches!(options.aggregation, Aggregation::MeanPower);
        let mut planner = FftPlanner::<f64>::new();
        let fft = planner.plan_fft_forward(fft_size);
        let window = transform::window(options.window, fft_size);
        let window_sum = window.iter().sum();
        let result = Spectrogram {
            info,
            options,
            fft_size,
            hop_size,
            max_frequency,
            power: zeroed(pixels)?,
        };
        let bin_rows: Vec<Option<usize>> = (0..=fft_size / 2)
            .map(|bin| {
                let hz = bin as f64 * f64::from(info.sample_rate) / fft_size as f64;
                if hz < result.options.min_frequency || hz > max_frequency {
                    None
                } else {
                    Some(((result.fraction(hz) * height as f64) as usize).min(height - 1))
                }
            })
            .collect();
        let first_bin = bin_rows.iter().position(Option::is_some).ok_or_else(|| {
            Error::Configuration(
                "frequency range contains no FFT bin; use a longer transform".into(),
            )
        })?;
        let last_bin = bin_rows
            .iter()
            .rposition(Option::is_some)
            .unwrap_or(first_bin);
        Ok(Self {
            ring: zeroed(fft_size * info.channels)?,
            scratch: zeroed(fft.get_inplace_scratch_len())?,
            buffer: zeroed(fft_size)?,
            bins: zeroed(fft_size / 2 + 1)?,
            rows: zeroed(height)?,
            row_counts: zeroed(height)?,
            counts: zeroed(if mean { pixels } else { 0 })?,
            window,
            window_sum,
            fft,
            result,
            bin_rows,
            valid_bins: (first_bin, last_bin),
            received: 0,
            next_center: 0,
            last_center: None,
        })
    }

    /// Consume complete, finite interleaved frames without retaining the packet.
    pub fn push_interleaved(&mut self, samples: &[f32]) -> Result<(), Error> {
        self.push_interleaved_cancellable(samples, &|| false)
    }

    /// Consume a packet, checking cancellation between transform windows.
    pub fn push_interleaved_cancellable(
        &mut self,
        samples: &[f32],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), Error> {
        let channels = self.result.info.channels;
        if !samples.len().is_multiple_of(channels) || samples.iter().any(|v| !v.is_finite()) {
            return Err(Error::Audio(
                "non-finite sample or incomplete interleaved frame".into(),
            ));
        }
        let frames = (samples.len() / channels) as u64;
        if frames > self.result.info.frames.saturating_sub(self.received) {
            return Err(Error::Audio(
                "stream exceeds the declared frame count".into(),
            ));
        }
        for frame in samples.chunks_exact(channels) {
            let start = (self.received % self.result.fft_size as u64) as usize * channels;
            self.ring[start..start + channels].copy_from_slice(frame);
            self.received += 1;
            if self.received
                >= self
                    .next_center
                    .saturating_add(self.result.fft_size as u64 / 2)
            {
                if cancelled() {
                    return Err(Error::Cancelled);
                }
                self.transform(self.next_center)?;
                self.next_center = self.next_center.saturating_add(self.result.hop_size as u64);
            }
        }
        Ok(())
    }

    /// Complete right-boundary windows and reject a missing audio tail.
    pub fn finish(self) -> Result<Spectrogram, Error> {
        self.finish_cancellable(&|| false)
    }

    /// Complete the spectrogram, allowing cancellation during boundary windows.
    pub fn finish_cancellable(
        mut self,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Spectrogram, Error> {
        if self.received != self.result.info.frames {
            return Err(Error::Audio(
                "incomplete stream: decoded frame count changed".into(),
            ));
        }
        while self.next_center < self.received {
            if cancelled() {
                return Err(Error::Cancelled);
            }
            self.transform(self.next_center)?;
            self.next_center = self.next_center.saturating_add(self.result.hop_size as u64);
        }
        if self.last_center != Some(self.received - 1) {
            if cancelled() {
                return Err(Error::Cancelled);
            }
            self.transform(self.received - 1)?;
        }
        if matches!(self.result.options.aggregation, Aggregation::MeanPower) {
            for (power, count) in self.result.power.iter_mut().zip(&self.counts) {
                if *count > 0 {
                    *power /= *count as f32;
                }
            }
        }
        Ok(self.result)
    }
}

fn zeroed<T: Default + Clone>(length: usize) -> Result<Vec<T>, Error> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(length)
        .map_err(|e| Error::Configuration(format!("allocation failed: {e}")))?;
    result.resize(length, T::default());
    Ok(result)
}

#[cfg(test)]
#[path = "../../tests/unit/analysis.rs"]
mod tests;
