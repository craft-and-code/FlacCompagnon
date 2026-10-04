use super::*;

#[test]
fn invalid_tail_does_not_report_loudness_of_only_the_valid_prefix() {
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut meter = LoudnessMeter::new(48_000, 2).unwrap();
        push_tone(&mut meter, 48_000, 4.0, -23.0, 0);
        assert!(meter.integrated_lufs().is_some());
        meter.push_frame(&[invalid, 0.0]);
        assert_eq!(meter.integrated_lufs(), None);
        assert_eq!(meter.loudness_range_lu(), None);
    }
}

#[test]
fn lra_samples_at_least_ten_times_per_second_at_11025_hz() {
    let mut meter = LoudnessMeter::new(11_025, 2).unwrap();
    push_tone(&mut meter, 11_025, 4.0, -23.0, 0);
    assert!(
        meter.short_powers.len() >= 11,
        "one initial window plus ten updates"
    );
}

// EBU Tech 3341, Table 1: its test tones are a stereo 1 kHz sine with the
// stated *peak* level on each channel, driven in phase.
fn push_tone(
    meter: &mut LoudnessMeter,
    sample_rate: u32,
    seconds: f64,
    peak_dbfs: f64,
    start: u64,
) -> u64 {
    let frames = (seconds * sample_rate as f64).round() as u64;
    let amplitude = 10f64.powf(peak_dbfs / 20.0);
    for n in start..start + frames {
        let sample = (amplitude
            * (2.0 * std::f64::consts::PI * 1_000.0 * n as f64 / sample_rate as f64).sin())
            as f32;
        meter.push_frame(&[sample, sample]);
    }
    start + frames
}

#[test]
fn ebu_tech_3341_tests_1_and_2_match_reference_loudness() {
    for (peak_dbfs, expected_lufs) in [(-23.0, -23.0), (-33.0, -33.0)] {
        let mut meter = LoudnessMeter::new(48_000, 2).expect("supported layout");
        push_tone(&mut meter, 48_000, 20.0, peak_dbfs, 0);
        let measured = meter.integrated_lufs().expect("measured loudness");
        assert!(
            (measured - expected_lufs).abs() <= 0.1,
            "{peak_dbfs} dBFS yielded {measured} LUFS"
        );
    }
}

#[test]
fn ebu_tech_3341_test_3_uses_the_relative_gate() {
    let mut meter = LoudnessMeter::new(48_000, 2).expect("supported layout");
    let mut at = 0;
    for (seconds, level) in [(10.0, -36.0), (60.0, -23.0), (10.0, -36.0)] {
        at = push_tone(&mut meter, 48_000, seconds, level, at);
    }
    let measured = meter.integrated_lufs().expect("measured loudness");
    assert!((measured + 23.0).abs() <= 0.1, "{measured} LUFS");
}

#[test]
fn ebu_tech_3341_test_4_gates_the_quiet_intro_and_outro() {
    let mut meter = LoudnessMeter::new(48_000, 2).expect("supported layout");
    let mut at = 0;
    for (seconds, level) in [
        (10.0, -72.0),
        (10.0, -36.0),
        (60.0, -23.0),
        (10.0, -36.0),
        (10.0, -72.0),
    ] {
        at = push_tone(&mut meter, 48_000, seconds, level, at);
    }
    let measured = meter.integrated_lufs().expect("measured loudness");
    assert!((measured + 23.0).abs() <= 0.1, "{measured} LUFS");
}

#[test]
fn ebu_tech_3341_test_5_uses_the_relative_gate() {
    let mut meter = LoudnessMeter::new(48_000, 2).expect("supported layout");
    let mut at = 0;
    for (seconds, level) in [(20.0, -26.0), (20.1, -20.0), (20.0, -26.0)] {
        at = push_tone(&mut meter, 48_000, seconds, level, at);
    }
    let measured = meter.integrated_lufs().expect("measured loudness");
    assert!((measured + 23.0).abs() <= 0.1, "{measured} LUFS");
}

#[test]
fn other_common_sample_rates_retain_the_same_one_kilohertz_reading() {
    for sample_rate in [44_100, 96_000, 192_000, 352_800] {
        let mut meter = LoudnessMeter::new(sample_rate, 2).expect("supported rate");
        push_tone(&mut meter, sample_rate, 3.0, -23.0, 0);
        let measured = meter.integrated_lufs().expect("measured loudness");
        assert!(
            (measured + 23.0).abs() <= 0.1,
            "{sample_rate} Hz yielded {measured} LUFS"
        );
    }
}

#[test]
fn published_k_weighting_changes_low_and_high_frequency_loudness() {
    // Analytic transfer magnitudes from ITU-R BS.1770-5 Annex 1 Tables 1/2:
    // H(100 Hz) = -1.1335 dB, H(10 kHz) = +4.0419 dB. Applying equation 2
    // to a -23 dBFS peak in-phase stereo sine gives these LUFS values.
    for (frequency, expected_lufs) in [(100.0, -24.8245), (10_000.0, -19.6491)] {
        let mut meter = LoudnessMeter::new(48_000, 2).expect("supported layout");
        let amplitude = 10f64.powf(-23.0 / 20.0);
        for n in 0..(48_000 * 3) {
            let sample = (amplitude
                * (2.0 * std::f64::consts::PI * frequency * n as f64 / 48_000.0).sin())
                as f32;
            meter.push_frame(&[sample, sample]);
        }
        let measured = meter.integrated_lufs().expect("measured loudness");
        assert!(
            (measured as f64 - expected_lufs).abs() < 0.1,
            "{frequency} Hz yielded {measured} LUFS"
        );
    }
}

#[test]
fn a_single_channel_has_three_lu_less_power_than_identical_stereo() {
    let mut meter = LoudnessMeter::new(48_000, 1).expect("mono supported");
    let amplitude = 10f64.powf(-23.0 / 20.0);
    for n in 0..(48_000 * 3) {
        let sample =
            (amplitude * (2.0 * std::f64::consts::PI * 1_000.0 * n as f64 / 48_000.0).sin()) as f32;
        meter.push_frame(&[sample]);
    }
    let measured = meter.integrated_lufs().expect("measured loudness");
    assert!((measured + 26.01).abs() < 0.1, "{measured} LUFS");
}

#[test]
fn silence_short_files_and_unmapped_multichannel_audio_have_no_reading() {
    let mut meter = LoudnessMeter::new(48_000, 1).expect("mono supported");
    for _ in 0..48_000 {
        meter.push_frame(&[0.0]);
    }
    assert_eq!(meter.integrated_lufs(), None);
    let mut short = LoudnessMeter::new(48_000, 2).expect("stereo supported");
    for _ in 0..10_000 {
        short.push_frame(&[0.1, 0.1]);
    }
    assert_eq!(short.integrated_lufs(), None);
    assert!(LoudnessMeter::new(48_000, 6).is_none());
}

#[test]
fn ebu_tech_3342_reference_two_level_ranges() {
    // EBU Tech 3342 (2023), Table 1, test cases 1-3.
    for (first, second, expected) in [
        (-20.0, -30.0, 10.0),
        (-20.0, -15.0, 5.0),
        (-40.0, -20.0, 20.0),
    ] {
        let mut meter = LoudnessMeter::new(48_000, 2).expect("stereo supported");
        let at = push_tone(&mut meter, 48_000, 20.0, first, 0);
        push_tone(&mut meter, 48_000, 20.0, second, at);
        let range = meter.loudness_range_lu().expect("range measured");
        assert!(
            (range - expected).abs() <= 1.0,
            "{first}/{second} dBFS yielded {range} LU"
        );
    }
}

#[test]
fn ebu_tech_3342_reference_five_level_range_gates_low_background() {
    // EBU Tech 3342 (2023), Table 1, test case 4: the -50 dBFS parts fall
    // below the relative gate, leaving a 15 LU range.
    let mut meter = LoudnessMeter::new(48_000, 2).expect("stereo supported");
    let mut at = 0;
    for level in [-50.0, -35.0, -20.0, -35.0, -50.0] {
        at = push_tone(&mut meter, 48_000, 20.0, level, at);
    }
    let range = meter.loudness_range_lu().expect("range measured");
    assert!((range - 15.0).abs() <= 1.0, "{range} LU");
}

#[test]
fn lra_needs_three_seconds_and_a_non_silent_signal() {
    let mut short = LoudnessMeter::new(48_000, 2).expect("stereo supported");
    push_tone(&mut short, 48_000, 2.0, -23.0, 0);
    assert_eq!(short.loudness_range_lu(), None);
    let mut silent = LoudnessMeter::new(48_000, 2).expect("stereo supported");
    for _ in 0..(48_000 * 10) {
        silent.push_frame(&[0.0, 0.0]);
    }
    assert_eq!(silent.loudness_range_lu(), None);
}

#[test]
fn extreme_float_samples_do_not_produce_a_nonfinite_range() {
    let mut meter = LoudnessMeter::new(48_000, 2).expect("stereo supported");
    meter.push_frame(&[f32::MAX, f32::MAX]);
    for _ in 0..(48_000 * 4) {
        meter.push_frame(&[0.0, 0.0]);
    }
    assert_eq!(meter.loudness_range_lu(), None);
}

#[test]
fn silence_and_below_gate_audio_do_not_grow_programme_histories() {
    for level in [None, Some(-80.0)] {
        let mut meter = LoudnessMeter::new(48_000, 2).unwrap();
        if let Some(level) = level {
            push_tone(&mut meter, 48_000, 4.0, level, 0);
            // Gate filtering must not also gate the ungated M/S maxima.
            assert!(meter.peaks().unwrap().short_term.is_some());
        } else {
            for _ in 0..192_000 {
                meter.push_frame(&[0.0, 0.0]);
            }
        }
        assert!(meter.block_powers.is_empty());
        assert!(meter.short_powers.is_empty());
        assert_eq!(meter.integrated_lufs(), None);
        assert_eq!(meter.loudness_range_lu(), None);
    }
}

// These tests inject a distribution of independently chosen linear powers.
// Zero filter/ring history makes the LRA-only tail silent, so they isolate
// the specified gates and rounded-index quantiles from the K-filter response.
fn range_of_powers(powers: Vec<f64>) -> Option<f32> {
    let mut meter = LoudnessMeter::new(48_000, 1).unwrap();
    meter.frames = meter.short_ring.len() as u64;
    meter.short_powers = powers;
    meter.loudness_range_lu()
}

#[test]
fn lra_selects_quantiles_without_including_the_bottom_ten_or_top_five_percent() {
    // Exactly 100 observations: rank 10 (zero-based) is 4 and rank 94 is 16.
    // Noise below the absolute gate must not change their ranks or weights.
    let mut powers = vec![1.0; 10];
    powers.extend([4.0; 75]);
    powers.extend([16.0; 10]);
    powers.extend([64.0; 5]);
    for repetitions in [1, 3] {
        let mut repeated = powers.repeat(repetitions);
        repeated.extend([0.0; 200]);
        repeated.extend([1e-10; 100]);
        repeated.reverse();
        let actual = range_of_powers(repeated).unwrap();
        let expected = (10.0 * (16.0_f64 / 4.0).log10()) as f32;
        assert!((actual - expected).abs() < 1e-5, "{actual} vs {expected}");
    }
}

#[test]
fn lra_relative_gate_uses_power_mean_and_twenty_lu_not_ten() {
    // Absolute-gated mean = (80*.01 + 10*1 + 10*100)/100 = 10.108.
    // Its -20 LU power threshold is .10108: .01 is removed, 1 is retained.
    // An arithmetic mean in dB or a -10 LU gate gives a different result.
    let mut powers = vec![0.01; 80];
    powers.extend([1.0; 10]);
    powers.extend([100.0; 10]);
    assert!((range_of_powers(powers).unwrap() - 20.0).abs() < 1e-5);
}

#[test]
fn gate_equality_is_excluded_for_integrated_but_included_for_lra() {
    let mut meter = LoudnessMeter::new(48_000, 1).unwrap();
    let at_gate = meter.absolute_gate_power;
    meter.block_powers = vec![at_gate; 20];
    assert_eq!(meter.integrated_lufs(), None);
    assert_eq!(range_of_powers(vec![at_gate; 20]), Some(0.0));
}

#[test]
fn lra_rounded_rank_selection_handles_one_value_and_halfway_rounding() {
    assert_eq!(range_of_powers(vec![4.0]), Some(0.0));
    let powers: Vec<_> = (1..=11).rev().map(f64::from).collect();
    // (11-1)*.1 = 1, (11-1)*.95 = 9.5, rounded upward to rank 10.
    let expected = (10.0 * (11.0_f64 / 2.0).log10()) as f32;
    assert!((range_of_powers(powers).unwrap() - expected).abs() < 1e-5);
}

#[test]
fn the_published_48_kilohertz_coefficients_are_preserved_exactly() {
    let filters = ChannelFilter::new(48_000);
    assert_eq!(filters.shelf.b, SHELF_48K.0);
    assert_eq!(filters.shelf.a, SHELF_48K.1);
    assert_eq!(filters.high_pass.b, HIGH_PASS_48K.0);
    assert_eq!(filters.high_pass.a, HIGH_PASS_48K.1);
}

fn squared_transfer(filter: &Biquad, frequency: f64, rate: u32) -> f64 {
    let angle = std::f64::consts::TAU * frequency / f64::from(rate);
    let squared_polynomial = |c: [f64; 3]| {
        let real = c[0] + c[1] * angle.cos() + c[2] * (2.0 * angle).cos();
        let imaginary = c[1] * angle.sin() + c[2] * (2.0 * angle).sin();
        real * real + imaginary * imaginary
    };
    squared_polynomial(filter.b) / squared_polynomial(filter.a)
}

#[test]
fn low_rate_k_weighting_matches_independently_measured_ffmpeg_metadata() {
    // FFmpeg 9.0.2 ebur128 metadata, last M/S of a four-second stereo sine
    // with -23 dBFS peak per channel; frequencies 30, 100, 1000, 3000 Hz.
    // Generated independently as sin(2*pi*f*n/Fs), f32le, no resampling.
    // These fixed reference readings include both K-filter transitions.
    // FFmpeg metadata rounds to .001 LU; the analytic steady-state response
    // should agree within that precision, without fitting the 1 kHz level.
    for (rate, references) in [
        (8_000, [-31.780, -24.609, -22.980, -19.461]),
        (11_025, [-31.851, -24.680, -22.953, -19.611]),
        (48_000, [-31.995, -24.824, -22.993, -19.883]),
        (96_000, [-32.016, -24.846, -23.011, -19.911]),
    ] {
        let filters = ChannelFilter::new(rate);
        for (frequency, reference) in [30.0, 100.0, 1_000.0, 3_000.0].into_iter().zip(references) {
            let transfer = squared_transfer(&filters.shelf, frequency, rate)
                * squared_transfer(&filters.high_pass, frequency, rate);
            // Two equal sine channels cancel the sinusoidal mean's 1/2.
            let lufs = LOUDNESS_OFFSET - 23.0 + 10.0 * transfer.log10();
            assert!(
                (lufs - reference).abs() <= 0.001,
                "{rate} Hz, {frequency} Hz sine: {lufs} vs {reference}"
            );
        }
    }
}

#[test]
fn retuned_filter_poles_are_stable_across_the_supported_rate_range() {
    for rate in [
        8_000, 11_025, 16_000, 32_000, 44_100, 48_000, 96_000, 192_000, 768_000,
    ] {
        let filters = ChannelFilter::new(rate);
        for filter in [filters.shelf, filters.high_pass] {
            // Second-order Jury stability conditions, independent of the
            // coefficient-retuning calculation: every pole is inside |z|<1.
            let [_, a1, a2] = filter.a;
            assert!(a2.abs() < 1.0);
            assert!(1.0 + a1 + a2 > 0.0);
            assert!(1.0 - a1 + a2 > 0.0);
            assert!(filter.b.iter().chain(&filter.a).all(|v| v.is_finite()));
            for fraction in [0.001, 0.01, 0.1, 0.4] {
                let power = squared_transfer(&filter, f64::from(rate) * fraction, rate);
                assert!(power.is_finite() && power > 0.0);
            }
        }
    }
}

#[test]
fn odd_sample_rate_lra_padding_is_not_shorter_than_one_point_five_seconds() {
    let mut meter = LoudnessMeter::new(11_025, 1).unwrap();
    meter.frames = 33_075;
    meter.pad_lra_tail();
    let added = meter.frames - 33_075;
    // At least 1.5 seconds, rounded upward by less than one sample.
    assert!(2 * added >= 3 * 11_025);
    assert!(2 * (added - 1) < 3 * 11_025);
}

#[test]
fn quiet_loudness_windows_recover_after_a_large_finite_float_passage() {
    let mut meter = LoudnessMeter::new(48_000, 1).unwrap();
    for frame in 0..384_000 {
        if frame == 336_000 {
            // Peak-hold reset isolates complete windows after the loud
            // passage has left both histories; the K filter has settled.
            meter.peaks = LoudnessPeaksMeter::new(48_000, 19_200, 144_000);
        }
        let amplitude = if frame < 144_000 {
            2f64.powi(20)
        } else {
            0.125
        };
        let sample = (amplitude
            * (std::f64::consts::TAU * 1_000.0 * f64::from(frame) / 48_000.0).sin())
            as f32;
        meter.push_frame(&[sample]);
    }
    // Analytic BS.1770 Tables 1/2 response: mono sine power is A^2 |H|^2/2.
    // This is the same quiet passage irrespective of the preceding gain.
    let expected = -21.0654;
    let current_m = power_to_lufs(meter.window_power.total() / 19_200.0).unwrap();
    let current_s = power_to_lufs(meter.short_window_power.total() / 144_000.0).unwrap();
    let last_i = power_to_lufs(*meter.block_powers.last().unwrap()).unwrap();
    let last_lra = power_to_lufs(*meter.short_powers.last().unwrap()).unwrap();
    let peaks = meter.peaks().unwrap();
    for measured in [
        current_m,
        current_s,
        last_i,
        last_lra,
        peaks.momentary.unwrap().lufs,
        peaks.short_term.unwrap().lufs,
    ] {
        assert!(
            (measured - expected).abs() < 0.005,
            "{measured} vs {expected}"
        );
    }
}
