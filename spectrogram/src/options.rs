//! Measurement choices, independent of image styling.

use crate::Error;

/// Maximum plotted pixels, bounding allocations independently of file headers.
pub const MAX_PIXELS: usize = 16_777_216;

/// Plot presets shared by hosts; custom dimensions remain available separately.
#[derive(Debug, Clone, Copy, Default)]
pub enum PlotSize {
    /// 1200×627, rounded from the original 900:470 aspect ratio.
    #[default]
    Standard,
    /// Original detailed plot, 1800×940.
    Large,
}

impl PlotSize {
    /// Plotted width and height, excluding the surrounding frame.
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Standard => (1200, 627),
            Self::Large => (1800, 940),
        }
    }
}

/// The actual decoded PCM format and exact number of complete source frames.
#[derive(Debug, Clone, Copy)]
pub struct SignalInfo {
    /// Frames per second, without resampling.
    pub sample_rate: u32,
    /// Interleaved samples per frame, from 1 through 32.
    pub channels: usize,
    /// Exact source frame count; a mismatched stream is rejected.
    pub frames: u64,
}

/// Frequency mapping of plotted rows.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum FrequencyScale {
    /// Equal Hz intervals occupy equal vertical distances.
    #[default]
    Linear,
    /// Equal frequency ratios occupy equal distances; minimum must be positive.
    Logarithmic,
}

/// Window applied before each Fourier transform.
#[derive(Debug, Clone, Copy, Default)]
pub enum Window {
    /// Periodic Hann; the general-purpose default.
    #[default]
    Hann,
    /// Four-term minimum-sidelobe Blackman-Harris; broader main lobe.
    BlackmanHarris,
    /// No taper; suitable for exact bin-centered synthetic references.
    Rectangular,
}

/// Combine spectra after transforming channels, avoiding mono cancellation.
#[derive(Debug, Clone, Copy, Default)]
pub enum Channels {
    /// Average channel powers, preserving identical mono/stereo levels.
    #[default]
    MeanPower,
    /// Retain the strongest channel at each frequency.
    MaximumPower,
    /// Analyze a single zero-based channel.
    Single(usize),
}

/// Reduce spectral observations into a plotted pixel.
#[derive(Debug, Clone, Copy, Default)]
pub enum Aggregation {
    /// Preserve the strongest contributing bin/window, including brief events.
    #[default]
    Peak,
    /// Average linear power over contributing bins/windows, before decibels.
    MeanPower,
}

/// Spectral computation parameters. Dimensions describe the plot, not margins.
#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    /// Plotted width; default 1200 pixels.
    pub width: u32,
    /// Plotted height; default round(1200 × 470 / 900) = 627 pixels.
    pub height: u32,
    /// FFT length; `None` keeps approximately the 8192/44.1 kHz duration.
    pub fft_size: Option<usize>,
    /// Center-to-center hop; `None` uses one eighth of the FFT length.
    pub hop_size: Option<usize>,
    /// Taper before the transform.
    pub window: Window,
    /// Channel selection/combination.
    pub channels: Channels,
    /// Pixel reduction convention.
    pub aggregation: Aggregation,
    /// Vertical axis mapping.
    pub frequency_scale: FrequencyScale,
    /// Lower plotted frequency in Hz.
    pub min_frequency: f64,
    /// Upper plotted frequency; `None` uses decoded Nyquist.
    pub max_frequency: Option<f64>,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        let (width, height) = PlotSize::Standard.dimensions();
        Self {
            width,
            height,
            fft_size: None,
            hop_size: None,
            window: Window::Hann,
            channels: Channels::MeanPower,
            aggregation: Aggregation::Peak,
            frequency_scale: FrequencyScale::Linear,
            min_frequency: 0.0,
            max_frequency: None,
        }
    }
}

impl AnalysisOptions {
    pub(crate) fn validate(&self, signal: SignalInfo) -> Result<(usize, usize, f64), Error> {
        let invalid = |message: &str| Error::Configuration(message.into());
        if signal.sample_rate == 0 || !(1..=32).contains(&signal.channels) || signal.frames == 0 {
            return Err(invalid(
                "nonempty PCM with a positive rate and 1–32 channels required",
            ));
        }
        let pixels = u64::from(self.width) * u64::from(self.height);
        if self.width == 0 || self.height == 0 || pixels > MAX_PIXELS as u64 {
            return Err(invalid(
                "plot dimensions exceed the pixel limit or are zero",
            ));
        }
        let exponent = (signal.sample_rate as f64 * 8192.0 / 44100.0)
            .log2()
            .round() as u32;
        let automatic = (1_usize << exponent.min(16)).clamp(256, 65536);
        let fft = self.fft_size.unwrap_or(automatic);
        let hop = self.hop_size.unwrap_or(fft / 8);
        if !(16..=65536).contains(&fft) || !fft.is_power_of_two() || hop == 0 || hop > fft {
            return Err(invalid(
                "FFT must be a power of two from 16 to 65536; hop must be 1..=FFT",
            ));
        }
        if matches!(self.channels, Channels::Single(channel) if channel >= signal.channels) {
            return Err(invalid("selected channel is absent"));
        }
        let max = self
            .max_frequency
            .unwrap_or(f64::from(signal.sample_rate) / 2.0);
        if !self.min_frequency.is_finite()
            || !max.is_finite()
            || self.min_frequency < 0.0
            || max <= self.min_frequency
            || max > f64::from(signal.sample_rate) / 2.0
            || (self.frequency_scale == FrequencyScale::Logarithmic && self.min_frequency <= 0.0)
        {
            return Err(invalid(
                "frequency bounds must be ordered and within decoded Nyquist",
            ));
        }
        Ok((fft, hop, max))
    }
}
