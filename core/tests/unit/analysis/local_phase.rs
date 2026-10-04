use super::*;

fn tone(n: usize, bin: usize, size: usize, phase: f64) -> f32 {
    (0.2 * (std::f64::consts::TAU * bin as f64 * n as f64 / size as f64 + phase).sin()) as f32
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
}

#[test]
fn sine_phase_angles_have_their_analytical_cosine_correlation() {
    for angle in [
        0.0,
        std::f64::consts::FRAC_PI_3,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    ] {
        let mut meter = LocalPhaseMeter::new(48_000, 2).unwrap();
        let size = meter.hann.len();
        for n in 0..size * 2 {
            meter.push_frame(&[tone(n, 300, size, 0.0), 0.25 * tone(n, 300, size, angle)]);
        }
        let result = meter.finish().unwrap();
        let summary = result.broadband.unwrap();
        near(summary.correlation, angle.cos() as f32);
        near(summary.minimum_correlation, angle.cos() as f32);
        assert_eq!(summary.eligible_windows, 3);
        near(
            result.bands[1].summary.unwrap().correlation,
            angle.cos() as f32,
        );
        assert_eq!(
            summary.opposed_fraction,
            if angle.cos() <= -0.5 { 1.0 } else { 0.0 }
        );
    }
}

#[test]
fn an_inverted_treble_band_is_visible_under_a_positive_broadband_average() {
    let mut meter = LocalPhaseMeter::new(48_000, 2).unwrap();
    let size = meter.hann.len();
    for n in 0..size {
        let bass = tone(n, 32, size, 0.0);
        let treble = tone(n, 3000, size, 0.0) * 0.5;
        meter.push_frame(&[bass + treble, bass - treble]);
    }
    let result = meter.finish().unwrap();
    // Independent tones: (A² - B²)/(A² + B²), where B = A/2.
    near(result.broadband.unwrap().correlation, 0.6);
    near(result.bands[0].summary.unwrap().correlation, 1.0);
    near(result.bands[3].summary.unwrap().correlation, -1.0);
    assert!(result.bands[1].summary.is_none());
    assert!(result.bands[2].summary.is_none());
}

#[test]
fn a_short_opposed_section_keeps_its_location_even_when_the_track_agrees_overall() {
    let mut meter = LocalPhaseMeter::new(48_000, 2).unwrap();
    let size = meter.hann.len();
    for n in 0..size * 4 {
        let sample = tone(n, 300, size, 0.0);
        let sign = if (size * 2..size * 3).contains(&n) {
            -1.0
        } else {
            1.0
        };
        meter.push_frame(&[sample, sample * sign]);
    }
    let result = meter.finish().unwrap();
    let summary = result.broadband.unwrap();
    assert!(summary.correlation > 0.3);
    near(summary.minimum_correlation, -1.0);
    assert_eq!(summary.minimum_start_secs, (size * 2) as f64 / 48_000.0);
    assert_eq!(summary.eligible_windows, 7);
    assert_eq!(summary.opposed_fraction, 1.0 / 7.0);
}

#[test]
fn silence_dc_and_a_quiet_or_missing_channel_do_not_invent_phase_evidence() {
    for (dc, gain) in [(0.0, 0.0), (0.5, 0.0), (0.5, 0.0001)] {
        let mut meter = LocalPhaseMeter::new(48_000, 2).unwrap();
        let size = meter.hann.len();
        for n in 0..size {
            let sample = tone(n, 300, size, 0.0);
            meter.push_frame(&[dc + sample, dc + gain * sample]);
        }
        assert!(meter.finish().is_none());
    }
    // Removing separate DC means must retain the underlying opposition.
    let mut meter = LocalPhaseMeter::new(48_000, 2).unwrap();
    let size = meter.hann.len();
    for n in 0..size {
        let sample = tone(n, 300, size, 0.0);
        meter.push_frame(&[0.5 + sample, 0.25 - sample]);
    }
    near(meter.finish().unwrap().broadband.unwrap().correlation, -1.0);
}

#[test]
fn invalid_tail_withholds_the_valid_prefix_and_partial_windows_are_not_padded() {
    for tail in [
        vec![],
        vec![0.0],
        vec![0.0, 0.0, 0.0],
        vec![f32::NAN, 0.0],
        vec![0.0, f32::INFINITY],
    ] {
        let mut meter = LocalPhaseMeter::new(48_000, 2).unwrap();
        let size = meter.hann.len();
        for n in 0..size {
            let sample = tone(n, 300, size, 0.0);
            meter.push_frame(&[sample, sample]);
        }
        meter.push_frame(&tail);
        assert!(meter.finish().is_none());
    }
    for complete in [false, true] {
        let mut meter = LocalPhaseMeter::new(48_000, 2).unwrap();
        let size = meter.hann.len();
        let frames = if complete {
            size + size / 2 - 1
        } else {
            size - 1
        };
        for n in 0..frames {
            let sample = tone(n, 300, size, 0.0);
            meter.push_frame(&[sample, sample]);
        }
        let result = meter.finish();
        assert_eq!(result.is_some(), complete);
        if let Some(result) = result {
            assert_eq!(result.analyzed_windows, 1);
        }
    }
}

#[test]
fn rate_dependent_windows_and_nyquist_limits_are_explicit() {
    for (rate, channels) in [(0, 2), (7999, 2), (768001, 2), (48_000, 1), (48_000, 6)] {
        assert!(LocalPhaseMeter::new(rate, channels).is_none());
    }
    for rate in [8_000, 44_100, 96_000, 192_000, 768_000] {
        let mut meter = LocalPhaseMeter::new(rate, 2).unwrap();
        let size = meter.hann.len();
        let bin = (1000.0 * size as f64 / rate as f64).round() as usize;
        for n in 0..size {
            let sample = tone(n, bin, size, 0.0);
            meter.push_frame(&[sample, -sample]);
        }
        let result = meter.finish().unwrap();
        assert!((0.2..0.4).contains(&result.window_secs));
        assert_eq!(result.hop_secs, result.window_secs * 0.5);
        near(result.bands[1].summary.unwrap().correlation, -1.0);
        assert_eq!(result.bands[3].high_hz, 20_000.0_f64.min(rate as f64 / 2.0));
        assert!(result.bands[3].summary.is_none());
    }
}

#[test]
fn the_rms_gate_is_independent_of_transform_size() {
    for rate in [48_000, 96_000] {
        for amplitude in [0.0005_f32, 0.002] {
            let mut meter = LocalPhaseMeter::new(rate, 2).unwrap();
            let size = meter.hann.len();
            for n in 0..size {
                let sample = tone(n, 300, size, 0.0) * (amplitude / 0.2);
                meter.push_frame(&[sample, -sample]);
            }
            // A sine has RMS A/sqrt(2), independently of N and of its phase.
            assert_eq!(meter.finish().is_some(), amplitude / 2.0_f32.sqrt() > 0.001);
        }
    }
}

#[test]
fn a_large_constant_dc_cannot_invent_audible_phase_evidence() {
    for bias in [1e20_f32, 1e30, f32::MAX] {
        let mut meter = LocalPhaseMeter::new(48_000, 2).unwrap();
        for _ in 0..meter.hann.len() {
            meter.push_frame(&[bias, -bias]);
        }
        assert!(meter.finish().is_none(), "constant offset {bias}");
    }
}

#[test]
fn an_extreme_first_sample_with_zero_hann_weight_cannot_swallow_the_remaining_audio() {
    let mut meter = LocalPhaseMeter::new(32_768, 2).unwrap();
    let size = meter.hann.len();
    let extreme = 2.0_f32.powi(80);
    meter.push_frame(&[extreme, -extreme]);
    for n in 1..size {
        let sample = tone(n, 250, size, 0.0); // 1 kHz, A=.2, energy=.02.
        meter.push_frame(&[sample, -sample]);
    }
    // Hann(0)=0 exactly, so the first sample contributes nothing. The
    // remaining weighted tone must retain its independently known energy.
    assert!((meter.accumulators[0].energies[0] - 0.02).abs() < 1e-8);
    near(meter.finish().unwrap().broadband.unwrap().correlation, -1.0);
}

#[test]
fn integer_minimum_magnitude_matches_finite_float_comparisons_without_inventing_a_sample() {
    let mut cases = vec![
        vec![-0.5; 128],
        vec![-0.0, 0.0],
        vec![-0.0],
        vec![0.0],
        vec![f32::MIN_POSITIVE, f32::from_bits(1), -f32::from_bits(2)],
        vec![-f32::from_bits(1), f32::MIN_POSITIVE],
        vec![f32::MAX, -f32::MAX, -2.0_f32.powi(80)],
        vec![-0.125, 0.125, -0.25, 0.25],
    ];
    cases.push((0..8192).map(|n| tone(n, 250, 8192, 0.7)).collect());
    // Independent bit patterns span finite exponents, signs and subnormals;
    // the oracle below compares numeric absolute values, without bit ordering.
    let mut bits = 17_u32;
    let mut patterns = Vec::new();
    for _ in 0..10_000 {
        bits = bits.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let sample = f32::from_bits(bits);
        if sample.is_finite() {
            patterns.push(sample);
        }
    }
    cases.push(patterns);
    assert!(minimum_magnitude_sample(std::iter::empty()).is_none());
    for samples in cases {
        assert!(samples.iter().all(|sample| sample.is_finite()));
        let expected = samples.iter().copied().fold(samples[0], |old, sample| {
            if f64::from(sample).abs() < f64::from(old).abs() {
                sample
            } else {
                old
            }
        });
        let actual = minimum_magnitude_sample(samples.iter().copied()).unwrap();
        assert_eq!(actual.abs(), expected.abs());
        assert!(samples
            .iter()
            .any(|sample| sample.to_bits() == actual.to_bits()));
        if samples
            .iter()
            .filter(|sample| sample.abs() == expected.abs())
            .count()
            == 1
        {
            assert_eq!(actual.to_bits(), expected.to_bits());
        }
    }
}

#[test]
fn hann_parseval_energies_match_independent_tones_in_every_band() {
    let mut meter = LocalPhaseMeter::new(32_768, 2).unwrap();
    let size = meter.hann.len();
    let bins = [25, 250, 1000, 2250]; // Exactly 100, 1000, 4000 and 9000 Hz.
    let gains = [0.25_f64, 0.5, 0.75, 1.25];
    let phases = [
        0.0,
        std::f64::consts::FRAC_PI_3,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    ];
    for n in 0..size {
        let mut frame = [0.0_f64; 2];
        for i in 0..4 {
            let angle = std::f64::consts::TAU * bins[i] as f64 * n as f64 / size as f64;
            frame[0] += 0.2 * angle.sin();
            frame[1] += 0.2 * gains[i] * (angle + phases[i]).sin();
        }
        meter.push_frame(&[frame[0] as f32, frame[1] as f32]);
    }
    for i in 0..4 {
        // Periodic Hann's three non-zero tone bins have relative powers
        // 1:4:1; Parseval/window-gain correction restores A^2/2 per tone.
        let expected = [
            0.02,
            0.02 * gains[i].powi(2),
            0.02 * gains[i] * phases[i].cos(),
        ];
        for (actual, expected) in meter.accumulators[i + 1].energies.iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 1e-8,
                "band {i}: {actual} != {expected}"
            );
        }
    }
    for component in 0..3 {
        let summed_bands: f64 = meter
            .accumulators
            .iter()
            .skip(1)
            .map(|a| a.energies[component])
            .sum();
        assert!((meter.accumulators[0].energies[component] - summed_bands).abs() < 1e-12);
    }
}

#[test]
fn tones_on_band_edges_share_hann_leakage_without_double_counting_bins() {
    for (hz, lower_band) in [(200, 0), (2000, 1), (6000, 2)] {
        let mut meter = LocalPhaseMeter::new(32_768, 2).unwrap();
        let size = meter.hann.len();
        let bin = hz / 4;
        for n in 0..size {
            let x = tone(n, bin, size, 0.0);
            meter.push_frame(&[x, -x]);
        }
        // The center bin and upper neighbour lie in the upper band (5/6
        // energy); the lower Hann neighbour lies in the lower band (1/6).
        let lower = meter.accumulators[lower_band + 1].energies[0];
        let upper = meter.accumulators[lower_band + 2].energies[0];
        assert!((lower - 0.02 / 6.0).abs() < 1e-8, "edge {hz}: {lower}");
        assert!(
            (upper - 0.02 * 5.0 / 6.0).abs() < 1e-8,
            "edge {hz}: {upper}"
        );
        assert!((meter.accumulators[0].energies[0] - lower - upper).abs() < 1e-12);
    }
}

#[test]
fn audible_upper_limit_excludes_the_20khz_center_but_preserves_its_lower_hann_bin() {
    let mut meter = LocalPhaseMeter::new(65_536, 2).unwrap();
    let size = meter.hann.len();
    for n in 0..size {
        let x = tone(n, 5000, size, 0.0); // 20 kHz exactly; bin spacing = 4 Hz.
        meter.push_frame(&[x, -x]);
    }
    assert!((meter.accumulators[0].energies[0] - 0.02 / 6.0).abs() < 1e-8);
    near(
        meter.finish().unwrap().bands[3]
            .summary
            .unwrap()
            .correlation,
        -1.0,
    );
}

#[test]
fn aggregation_uses_summed_energies_instead_of_averaging_window_coefficients() {
    let mut accumulator = Accumulator::default();
    accumulator.push([1.0, 1.0, 1.0], 0.0);
    accumulator.push([100.0, 100.0, -100.0], 1.0);
    let summary = accumulator.summary.unwrap();
    near(summary.correlation, -99.0 / 101.0);
    assert_eq!(summary.minimum_start_secs, 1.0);
    assert_eq!(summary.opposed_fraction, 0.5);
}
