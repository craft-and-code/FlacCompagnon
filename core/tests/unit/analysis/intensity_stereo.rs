use super::*;

const RATE: u32 = 48_000;

fn analyse<F>(seconds: u32, mut frame: F) -> Option<HighFrequencyStereo>
where
    F: FnMut(usize) -> [f32; 2],
{
    let mut meter = HighFrequencyStereoMeter::new(RATE, 2).expect("48 kHz stereo is supported");
    for n in 0..seconds as usize * RATE as usize {
        meter.push_frame(&frame(n));
    }
    meter.finish()
}

fn sine(hz: f64, sample: usize) -> f64 {
    (std::f64::consts::TAU * hz * sample as f64 / RATE as f64).sin()
}

#[test]
fn persistent_high_band_side_collapse_is_reported_against_a_wide_reference() {
    // Ground truth is constructed directly in M/S form: the 3 kHz reference
    // has equal Mid and Side energy, while the 9 kHz component's Side energy
    // is 40 dB below Mid for four seconds.
    let measured = analyse(4, |n| {
        let reference_mid = 0.10 * sine(3_000.0, n);
        let reference_side = 0.10 * sine(3_000.0, n);
        let high_mid = 0.10 * sine(9_000.0, n);
        let high_side = 0.001 * sine(9_000.0, n);
        [
            (reference_mid + reference_side + high_mid + high_side) as f32,
            (reference_mid - reference_side + high_mid - high_side) as f32,
        ]
    })
    .expect("four active seconds provide enough blocks");

    assert!(measured.narrowed, "{measured:?}");
    assert!(measured.side_to_mid_db < -25.0);
    assert!(measured.reference_side_to_mid_db > -8.0);
    assert!(measured.narrowed_block_fraction > 0.9);
}

#[test]
fn wide_high_band_control_is_not_called_narrowed() {
    let measured = analyse(4, |n| {
        let reference_mid = 0.10 * sine(3_000.0, n);
        let reference_side = 0.10 * sine(3_000.0, n);
        let high_mid = 0.10 * sine(9_000.0, n);
        let high_side = 0.10 * sine(9_000.0, n);
        [
            (reference_mid + reference_side + high_mid + high_side) as f32,
            (reference_mid - reference_side + high_mid - high_side) as f32,
        ]
    })
    .expect("four active seconds provide enough blocks");

    assert!(!measured.narrowed);
    assert!(measured.side_to_mid_db > -8.0);
    assert!(measured.reference_side_to_mid_db > -8.0);
}

#[test]
fn signal_that_is_narrow_at_every_frequency_is_not_an_intensity_stereo_cue() {
    let measured = analyse(4, |n| {
        let reference_mid = 0.10 * sine(3_000.0, n);
        let reference_side = 0.001 * sine(3_000.0, n);
        let high_mid = 0.10 * sine(9_000.0, n);
        let high_side = 0.001 * sine(9_000.0, n);
        [
            (reference_mid + reference_side + high_mid + high_side) as f32,
            (reference_mid - reference_side + high_mid - high_side) as f32,
        ]
    })
    .expect("the Mid bands are active even though Side is narrow");

    assert!(!measured.narrowed);
    assert!(measured.reference_side_to_mid_db < -20.0);
}

#[test]
fn meter_rejects_unsupported_layouts_rates_silence_and_malformed_frames() {
    assert!(HighFrequencyStereoMeter::new(48_000, 1).is_none());
    assert!(HighFrequencyStereoMeter::new(48_000, 3).is_none());
    assert!(HighFrequencyStereoMeter::new(22_050, 2).is_none());

    let mut silence = HighFrequencyStereoMeter::new(RATE, 2).expect("supported");
    for _ in 0..RATE as usize * 4 {
        silence.push_frame(&[0.0, 0.0]);
    }
    assert!(silence.finish().is_none());

    let mut malformed = HighFrequencyStereoMeter::new(RATE, 2).expect("supported");
    malformed.push_frame(&[0.0]);
    assert!(malformed.finish().is_none());
}

#[test]
fn bass_side_energy_cannot_make_a_mono_reference_look_wide() {
    let measured = analyse(4, |n| {
        let mid = 0.02 * sine(3_000.0, n) + 0.1 * sine(9_000.0, n);
        let side = 0.5 * sine(200.0, n);
        [(mid + side) as f32, (mid - side) as f32]
    })
    .unwrap();
    assert!(
        !measured.narrowed,
        "bass leaked into the reference: {measured:?}"
    );
    assert!(measured.reference_side_to_mid_db < -20.0);
}

#[test]
fn ultrasonic_side_does_not_mask_audible_high_band_narrowing() {
    let rate = 192_000;
    let mut meter = HighFrequencyStereoMeter::new(rate, 2).unwrap();
    for n in 0..rate * 4 {
        let phase = std::f64::consts::TAU * n as f64 / rate as f64;
        let mid = 0.1 * (3_000.0 * phase).sin() + 0.1 * (9_000.0 * phase).sin();
        let side = 0.1 * (3_000.0 * phase).cos() + 0.2 * (60_000.0 * phase).sin();
        meter.push_frame(&[(mid + side) as f32, (mid - side) as f32]);
    }
    let measured = meter.finish().unwrap();
    assert!(
        measured.narrowed,
        "ultrasound masked the narrowing: {measured:?}"
    );
}

#[test]
fn exact_mono_ratio_does_not_depend_on_duration_or_gain() {
    let mut readings = Vec::new();
    for (seconds, gain) in [(2, 0.1), (4, 0.2)] {
        let measured = analyse(seconds, |n| {
            let mid = (gain * (sine(3_000.0, n) + sine(9_000.0, n))) as f32;
            [mid, mid]
        })
        .unwrap();
        readings.push(measured.side_to_mid_db);
        assert!(!measured.narrowed);
    }
    assert_eq!(
        readings[0], readings[1],
        "zero Side needs a ratio floor, not an absolute energy floor"
    );
}

#[test]
fn known_ratios_remain_consistent_across_supported_sample_rates() {
    for rate in [24_000, 32_000, 44_100, 48_000, 96_000, 192_000, 768_000] {
        let mut meter = HighFrequencyStereoMeter::new(rate, 2).unwrap();
        for n in 0..rate * 2 {
            let phase = std::f64::consts::TAU * n as f64 / rate as f64;
            // Reference M/S amplitudes match. High-band Side has one tenth
            // the Mid amplitude: independently expected 20 log10(0.1)=-20 dB.
            let mid = 0.1 * (3_000.0 * phase).sin() + 0.1 * (9_000.0 * phase).sin();
            let side = 0.1 * (3_000.0 * phase).sin() + 0.01 * (9_000.0 * phase).sin();
            meter.push_frame(&[(mid + side) as f32, (mid - side) as f32]);
        }
        let measured = meter.finish().unwrap();
        assert!(
            (measured.side_to_mid_db + 20.0).abs() < 0.3,
            "{rate}: {measured:?}"
        );
        assert!(
            measured.reference_side_to_mid_db.abs() < 0.1,
            "{rate}: {measured:?}"
        );
    }
}

#[test]
fn transient_narrowing_does_not_pass_the_persistence_gate() {
    for seconds_narrow in [3, 4] {
        let measured = analyse(5, |n| {
            let mid = 0.1 * (sine(3_000.0, n) + sine(9_000.0, n));
            let reference_gain = if n < RATE as usize * seconds_narrow {
                0.1
            } else {
                0.001
            };
            let side = reference_gain * sine(3_000.0, n) + 0.001 * sine(9_000.0, n);
            [(mid + side) as f32, (mid - side) as f32]
        })
        .unwrap();
        assert!((measured.narrowed_block_fraction - seconds_narrow as f32 / 5.0).abs() < 0.01);
        assert_eq!(measured.narrowed, seconds_narrow == 4, "{measured:?}");
    }
}

#[test]
fn short_or_invalid_tail_cannot_publish_a_valid_prefix() {
    assert!(analyse(1, |n| {
        let x = (0.1 * (sine(3_000.0, n) + sine(9_000.0, n))) as f32;
        [x, x]
    })
    .is_none());
    for invalid in [
        vec![],
        vec![0.0],
        vec![f32::NAN, 0.0],
        vec![0.0, f32::INFINITY],
    ] {
        let mut meter = HighFrequencyStereoMeter::new(RATE, 2).unwrap();
        for n in 0..RATE as usize * 2 {
            let x = (0.1 * (sine(3_000.0, n) + sine(9_000.0, n))) as f32;
            meter.push_frame(&[x, x]);
        }
        meter.push_frame(&invalid);
        assert!(meter.finish().is_none());
    }
}

#[test]
fn constant_channel_offsets_do_not_excite_a_false_high_band_startup_step() {
    let mut readings = Vec::new();
    for (left_dc, right_dc) in [(0.0, 0.0), (0.5, 0.5), (0.5, -0.5)] {
        let measured = analyse(2, |n| {
            let mid = 0.002 * (sine(3000.0, n) + sine(9000.0, n));
            let side = 0.002 * sine(3000.0, n) + 0.00002 * sine(9000.0, n);
            [
                (mid + side + left_dc) as f32,
                (mid - side + right_dc) as f32,
            ]
        })
        .unwrap();
        assert!(measured.narrowed, "DC {left_dc}/{right_dc}: {measured:?}");
        readings.push(measured);
    }
    for measured in &readings[1..] {
        assert!((measured.side_to_mid_db - readings[0].side_to_mid_db).abs() < 0.05);
        assert!(
            (measured.reference_side_to_mid_db - readings[0].reference_side_to_mid_db).abs() < 0.02
        );
        assert_eq!(
            measured.narrowed_block_fraction,
            readings[0].narrowed_block_fraction
        );
    }
}

#[test]
fn common_gain_channel_swap_and_common_polarity_keep_nonzero_side_ratios() {
    let mut baseline: Option<HighFrequencyStereo> = None;
    for (gain, swap, polarity) in [(0.01, false, 1.0), (0.1, true, 1.0), (0.5, false, -1.0)] {
        let measured = analyse(2, |n| {
            let mid = gain * (sine(3000.0, n) + sine(9000.0, n));
            let side = gain * (sine(3000.0, n) + 0.1 * sine(9000.0, n));
            let frame = [
                (polarity * (mid + side)) as f32,
                (polarity * (mid - side)) as f32,
            ];
            if swap {
                [frame[1], frame[0]]
            } else {
                frame
            }
        })
        .unwrap();
        if let Some(base) = baseline {
            assert!((measured.side_to_mid_db - base.side_to_mid_db).abs() < 0.005);
            assert!(
                (measured.reference_side_to_mid_db - base.reference_side_to_mid_db).abs() < 0.005
            );
            assert_eq!(measured.narrowed, base.narrowed);
        } else {
            baseline = Some(measured);
        }
    }
}

#[test]
fn a_panned_common_signal_has_side_energy_without_frequency_dependent_narrowing() {
    for right_gain in [0.0, 0.25, 0.5, 2.0] {
        let measured = analyse(2, |n| {
            let left = 0.1 * (sine(3000.0, n) + sine(9000.0, n));
            [left as f32, (right_gain * left) as f32]
        })
        .unwrap();
        let expected = 20.0 * ((1.0_f64 - right_gain) / (1.0 + right_gain)).abs().log10();
        assert!((f64::from(measured.side_to_mid_db) - expected).abs() < 1e-4);
        assert!((f64::from(measured.reference_side_to_mid_db) - expected).abs() < 1e-4);
        assert!(!measured.narrowed);
    }
    assert!(
        analyse(2, |n| {
            let x = (0.1 * (sine(3000.0, n) + sine(9000.0, n))) as f32;
            [x, -x]
        })
        .is_none(),
        "pure Side has no eligible Mid"
    );
}

#[test]
fn partial_third_block_cannot_satisfy_the_three_complete_block_minimum() {
    for missing_last_frame in [true, false] {
        let mut meter = HighFrequencyStereoMeter::new(RATE, 2).unwrap();
        let frames = RATE as usize * 3 / 2 - usize::from(missing_last_frame);
        for n in 0..frames {
            let x = (0.1 * (sine(3000.0, n) + sine(9000.0, n))) as f32;
            meter.push_frame(&[x, x]);
        }
        assert_eq!(meter.finish().is_some(), !missing_last_frame);
    }
}

#[test]
fn an_extreme_first_sample_does_not_swallow_normal_audio_in_later_blocks() {
    let rate = 48_000;
    let extreme = 2.0_f32.powi(80);
    for first in [[extreme, extreme], [extreme, -extreme]] {
        let mut meter = HighFrequencyStereoMeter::new(rate, 2).unwrap();
        meter.push_frame(&first);
        for n in 1..rate * 5 / 2 {
            let time = f64::from(n) / f64::from(rate);
            let reference = 0.02 * (std::f64::consts::TAU * 3_000.0 * time).sin();
            let high = 0.02 * (std::f64::consts::TAU * 9_000.0 * time).sin();
            let mid = reference + high;
            let side = reference + high * 0.01;
            meter.push_frame(&[(mid + side) as f32, (mid - side) as f32]);
        }
        // The extreme initial step may dominate the first block. Subsequent
        // complete blocks contain the analytical -40 dB high / 0 dB reference
        // M/S pair after the real filter transient has decayed.
        assert_eq!(meter.eligible_blocks, 5, "first frame {first:?}");
        assert!(meter.narrowed_blocks >= 4, "first frame {first:?}");
    }
}

#[test]
fn primed_high_pass_rejects_even_extreme_constant_mid_or_side_exactly() {
    for level in [2.0_f32.powi(80), f32::MAX] {
        for right in [level, -level] {
            let mut meter = HighFrequencyStereoMeter::new(24_000, 2).unwrap();
            for _ in 0..36_000 {
                meter.push_frame(&[level, right]);
            }
            assert_eq!(meter.eligible_blocks, 0);
            assert!(meter.finish().is_none());
        }
    }
}
