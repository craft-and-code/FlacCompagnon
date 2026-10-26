//! Per-channel Fourier transform and calibrated pixel reduction.

use super::SpectrogramBuilder;
use crate::{Aggregation, Channels, Error, Window};
use rustfft::num_complex::Complex;
use std::f64::consts::TAU;

pub(super) fn window(kind: Window, size: usize) -> Vec<f64> {
    (0..size)
        .map(|i| {
            let angle = TAU * i as f64 / size as f64;
            match kind {
                Window::Hann => 0.5 - 0.5 * angle.cos(),
                // Standard four-term minimum-sidelobe coefficients; F. J. Harris,
                // "On the Use of Windows for Harmonic Analysis", Proc. IEEE, 1978.
                Window::BlackmanHarris => {
                    0.35875 - 0.48829 * angle.cos() + 0.14128 * (2.0 * angle).cos()
                        - 0.01168 * (3.0 * angle).cos()
                }
                Window::Rectangular => 1.0,
            }
        })
        .collect()
}

impl SpectrogramBuilder {
    pub(super) fn transform(&mut self, center: u64) -> Result<(), Error> {
        let size = self.result.fft_size;
        let channels = self.result.info.channels;
        let count = match self.result.options.channels {
            Channels::Single(_) => 1,
            _ => channels,
        };
        self.bins.fill(0.0);
        for channel in 0..channels {
            if matches!(self.result.options.channels, Channels::Single(selected) if channel != selected)
            {
                continue;
            }
            for (i, value) in self.buffer.iter_mut().enumerate() {
                let frame = center
                    .checked_add(i as u64)
                    .and_then(|n| n.checked_sub(size as u64 / 2));
                let sample = frame
                    .filter(|&n| n < self.received)
                    .and_then(|n| {
                        self.ring
                            .get((n % size as u64) as usize * channels + channel)
                    })
                    .copied()
                    .unwrap_or(0.0);
                *value = Complex::new(f64::from(sample) * self.window[i], 0.0);
            }
            self.fft
                .process_with_scratch(&mut self.buffer, &mut self.scratch);
            for (bin, (value, power)) in self.buffer.iter().zip(&mut self.bins).enumerate() {
                // Coherent-gain correction: a bin-centered full-scale sine is
                // 0 dBFS/bin. DC/Nyquist have no negative-frequency partner.
                let factor = if bin == 0 || bin == size / 2 {
                    1.0
                } else {
                    2.0
                };
                let measured = value.norm_sqr() * (factor / self.window_sum).powi(2);
                match self.result.options.channels {
                    Channels::MaximumPower => *power = power.max(measured),
                    _ => *power += measured / count as f64,
                }
            }
        }
        self.rows.fill(0.0);
        self.row_counts.fill(0);
        let mean = matches!(self.result.options.aggregation, Aggregation::MeanPower);
        for (&power, &row) in self.bins.iter().zip(&self.bin_rows) {
            if let Some(row) = row {
                if mean {
                    self.rows[row] += power;
                } else {
                    self.rows[row] = self.rows[row].max(power);
                }
                self.row_counts[row] += 1;
            }
        }
        let width = self.result.options.width as usize;
        let frames = u128::from(self.result.info.frames);
        let half_hop = self.result.hop_size as u64 / 2;
        let first = (u128::from(center.saturating_sub(half_hop)) * width as u128 / frames) as usize;
        let end_frame = center
            .saturating_add((self.result.hop_size as u64).div_ceil(2))
            .min(self.result.info.frames);
        let last = (u128::from(end_frame) * width as u128)
            .div_ceil(frames)
            .min(width as u128) as usize;
        for (row, (&power, &count)) in self.rows.iter().zip(&self.row_counts).enumerate() {
            // If more rows than FFT bins were requested, paint the nearest bin
            // explicitly rather than suggesting that unmeasured rows are silent.
            let power = if count == 0 {
                let fraction = (row as f64 + 0.5) / self.rows.len() as f64;
                let min = self.result.options.min_frequency;
                let hz = match self.result.options.frequency_scale {
                    crate::FrequencyScale::Linear => {
                        min + fraction * (self.result.max_frequency - min)
                    }
                    crate::FrequencyScale::Logarithmic => {
                        min * (self.result.max_frequency / min).powf(fraction)
                    }
                };
                let bin =
                    (hz * size as f64 / f64::from(self.result.info.sample_rate)).round() as usize;
                self.bins
                    .get(bin.clamp(self.valid_bins.0, self.valid_bins.1))
                    .copied()
                    .unwrap_or(0.0)
            } else if mean {
                power / f64::from(count)
            } else {
                power
            };
            for x in first..last {
                let index = row * width + x;
                if mean {
                    self.result.power[index] += power as f32;
                    self.counts[index] = self.counts[index]
                        .checked_add(1)
                        .ok_or_else(|| Error::Audio("too many observations per pixel".into()))?;
                } else {
                    self.result.power[index] = self.result.power[index].max(power as f32);
                }
            }
        }
        self.last_center = Some(center);
        Ok(())
    }
}
