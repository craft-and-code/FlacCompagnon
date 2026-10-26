use super::*;
use crate::{Channels, FrequencyScale, Window};

fn options() -> AnalysisOptions {
    AnalysisOptions {
        width: 80,
        height: 257,
        fft_size: Some(512),
        hop_size: Some(64),
        window: Window::Hann,
        ..Default::default()
    }
}

fn tone(channels: usize, inverted: bool) -> Vec<f32> {
    (0..4096)
        .flat_map(|frame| {
            // Analytic bin 32: 3000 Hz at 48 kHz, amplitude exactly one quarter.
            let value = (0.25 * (std::f64::consts::TAU * 32.0 * frame as f64 / 512.0).sin()) as f32;
            (0..channels).map(move |channel| {
                if inverted && channel == 1 {
                    -value
                } else {
                    value
                }
            })
        })
        .collect()
}

fn compute(samples: &[f32], channels: usize, settings: AnalysisOptions) -> Spectrogram {
    let mut builder = SpectrogramBuilder::new(
        SignalInfo {
            sample_rate: 48000,
            channels,
            frames: (samples.len() / channels) as u64,
        },
        settings,
    )
    .unwrap();
    builder.push_interleaved(samples).unwrap();
    builder.finish().unwrap()
}

#[test]
fn quarter_scale_sine_is_minus_twelve_dbfs_at_the_correct_frequency() {
    let data = compute(&tone(1, false), 1, options());
    let strongest = data.power.iter().copied().fold(0.0_f32, f32::max);
    assert!((10.0 * f64::from(strongest).log10() + 12.041199826559248).abs() < 0.001);
    assert!((data.level_db(40, 32).unwrap() + 12.041199826559248).abs() < 0.001);
    assert!(data.level_db(40, 120).unwrap() < -100.0);
}

#[test]
fn opposed_stereo_retains_the_same_spectrum_as_mono() {
    let mono = compute(&tone(1, false), 1, options());
    let stereo = compute(&tone(2, true), 2, options());
    assert_eq!(mono.power, stereo.power);
}

#[test]
fn packet_boundaries_do_not_change_the_measurement() {
    let samples = tone(2, false);
    let whole = compute(&samples, 2, options());
    let mut builder = SpectrogramBuilder::new(whole.signal(), options()).unwrap();
    for packet in samples.chunks(34) {
        builder.push_interleaved(packet).unwrap();
    }
    assert_eq!(whole.power, builder.finish().unwrap().power);
}

#[test]
fn short_audio_has_no_artificial_silent_time_columns() {
    let data = compute(
        &tone(1, false)[..1200],
        1,
        AnalysisOptions {
            width: 1200,
            ..options()
        },
    );
    for column in 0..1200 {
        assert!(data.level_db(column, 32).unwrap() > -20.0);
    }
}

#[test]
fn dc_and_nyquist_are_not_doubled_like_mirrored_bins() {
    for nyquist in [false, true] {
        let samples: Vec<_> = (0..4096)
            .map(|n| if nyquist && n % 2 == 1 { -0.25 } else { 0.25 })
            .collect();
        let data = compute(&samples, 1, options());
        let row = if nyquist { 256 } else { 0 };
        assert!((data.level_db(40, row).unwrap() + 12.041199826559248).abs() < 0.001);
    }
}

#[test]
fn silence_is_negative_infinity_and_outside_pixels_are_absent() {
    let data = compute(&vec![0.0; 4096], 1, options());
    assert_eq!(data.level_db(1, 1), Some(f64::NEG_INFINITY));
    assert_eq!(data.level_db(80, 1), None);
}

#[test]
fn invalid_samples_and_changed_lengths_are_rejected() {
    let info = SignalInfo {
        sample_rate: 48000,
        channels: 2,
        frames: 10,
    };
    let mut builder = SpectrogramBuilder::new(info, options()).unwrap();
    assert!(builder.push_interleaved(&[0.0]).is_err());
    assert!(builder.push_interleaved(&[f32::NAN, 0.0]).is_err());
    assert!(builder.push_interleaved(&[f32::INFINITY, 0.0]).is_err());
    assert!(builder.push_interleaved(&[0.0; 22]).is_err());
    assert!(builder.finish().is_err());
}

#[test]
fn invalid_dimensions_frequencies_and_channels_are_rejected() {
    let info = SignalInfo {
        sample_rate: 48000,
        channels: 2,
        frames: 10,
    };
    for settings in [
        AnalysisOptions {
            width: 0,
            ..options()
        },
        AnalysisOptions {
            width: u32::MAX,
            height: u32::MAX,
            ..options()
        },
        AnalysisOptions {
            fft_size: Some(7),
            ..options()
        },
        AnalysisOptions {
            hop_size: Some(0),
            ..options()
        },
        AnalysisOptions {
            channels: Channels::Single(2),
            ..options()
        },
        AnalysisOptions {
            frequency_scale: FrequencyScale::Logarithmic,
            ..options()
        },
        AnalysisOptions {
            min_frequency: f64::NAN,
            ..options()
        },
        AnalysisOptions {
            max_frequency: Some(24001.0),
            ..options()
        },
    ] {
        assert!(SpectrogramBuilder::new(info, settings).is_err());
    }
}

#[test]
fn a_frequency_range_without_any_measured_bin_is_rejected() {
    // 48 kHz / 512 = 93.75 Hz per bin: neither 100 nor 110 Hz is a measured
    // frequency. Painting the nearest bin would import content outside the view.
    let info = SignalInfo {
        sample_rate: 48000,
        channels: 1,
        frames: 4096,
    };
    let settings = AnalysisOptions {
        min_frequency: 100.0,
        max_frequency: Some(110.0),
        ..options()
    };
    assert!(SpectrogramBuilder::new(info, settings).is_err());
}

#[test]
fn cancellation_is_observed_inside_a_large_packet_and_at_finish() {
    let samples = tone(1, false);
    let info = SignalInfo {
        sample_rate: 48000,
        channels: 1,
        frames: 4096,
    };
    let mut builder = SpectrogramBuilder::new(info, options()).unwrap();
    assert!(matches!(
        builder.push_interleaved_cancellable(&samples, &|| true),
        Err(Error::Cancelled)
    ));
    let mut builder = SpectrogramBuilder::new(info, options()).unwrap();
    builder.push_interleaved(&samples).unwrap();
    assert!(matches!(
        builder.finish_cancellable(&|| true),
        Err(Error::Cancelled)
    ));
}

#[test]
fn logarithmic_frequency_axis_places_a_known_tone_at_its_physical_position() {
    let data = compute(
        &tone(1, false),
        1,
        AnalysisOptions {
            frequency_scale: FrequencyScale::Logarithmic,
            min_frequency: 100.0,
            max_frequency: Some(24000.0),
            ..options()
        },
    );
    let row = ((3000.0_f64 / 100.0).ln() / 240.0_f64.ln() * 257.0) as u32;
    assert!((data.level_db(40, row).unwrap() + 12.041199826559248).abs() < 0.001);
}

#[test]
fn mean_power_pooling_keeps_the_reference_level_on_an_exact_bin() {
    let data = compute(
        &tone(1, false),
        1,
        AnalysisOptions {
            aggregation: Aggregation::MeanPower,
            ..options()
        },
    );
    assert!((data.level_db(40, 32).unwrap() + 12.041199826559248).abs() < 0.001);
}
