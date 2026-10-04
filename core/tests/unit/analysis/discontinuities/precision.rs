use super::*;

#[test]
fn a_large_float_leaving_the_power_window_does_not_hide_a_later_dropout() {
    for rate in [8_000, 48_000, 768_000] {
        let mut samples = vec![0.125; rate as usize / 10];
        samples[0] = 2f32.powi(40);
        let start = rate as usize / 20;
        let width = (rate as usize).div_ceil(500);
        samples[start..start + width].fill(0.0);
        let result = analyze(rate, 1, &samples).unwrap();
        // Both local powers are exactly (1/8)^2, irrespective of the old
        // sample. The finite injected gap is the only zero run in the file.
        assert_eq!(result.dropouts.count, 1, "{rate} Hz");
        assert_eq!(
            result.dropouts.events[0].start_secs,
            start as f64 / rate as f64
        );
        assert_eq!(
            result.dropouts.events[0].duration_secs,
            width as f64 / rate as f64
        );
    }
}

#[test]
fn a_large_early_float_does_not_turn_a_clean_square_wave_into_clicks() {
    let mut samples = [0.125, -0.125].repeat(2_400);
    samples[0] = 2f32.powi(40);
    // Every contextual first difference is 1/4; every edge is also 1/4,
    // so no edge can be eight times its context RMS.
    assert_eq!(analyze(48_000, 1, &samples).unwrap().clicks.count, 0);
}

#[test]
fn constant_offsets_and_polarity_do_not_change_isolated_pulse_edges() {
    for bias in [0.0, 0.125, -2.0] {
        for gain in [1.0, -1.0, 4.0] {
            let mut samples = vec![bias; 4_800];
            samples[2_400] += 0.5 * gain;
            let result = analyze(48_000, 1, &samples).unwrap();
            assert_eq!(result.clicks.count, 1);
            assert_eq!(result.dropouts.count, 0);
        }
    }
}

#[test]
fn above_full_scale_active_audio_is_not_clamped_before_dropout_detection() {
    for amplitude in [-2.0, 2.0] {
        let mut samples = vec![amplitude; 4_800];
        samples[2_400..2_496].fill(0.0);
        let result = analyze(48_000, 1, &samples).unwrap();
        assert_eq!(result.dropouts.count, 1);
        assert_eq!(result.clicks.count, 0);
    }
}

#[test]
fn a_late_first_candidate_has_complete_left_and_right_energy_contexts() {
    for rate in [8_000, 48_000, 192_000, 768_000] {
        let context = (rate as usize).div_ceil(200);
        let hop = (rate as usize).div_ceil(100);
        let width = rate as usize / 2_000;
        let start = context + hop;
        let mut samples: Vec<_> = (0..2 * context + hop + width + 1)
            .map(|n| {
                if n.is_multiple_of(2) {
                    1.0 / 128.0
                } else {
                    -1.0 / 128.0
                }
            })
            .collect();
        // Background derivatives are exactly 1/64, below the absolute floor.
        // This first eligible entry is the last candidate of the full batch;
        // its widest allowed exit still needs the final context sample.
        for sample in &mut samples[start..start + width] {
            *sample += 0.5;
        }
        let result = analyze(rate, 1, &samples).unwrap();
        assert_eq!(result.clicks.count, 1, "{rate} Hz");
        assert_eq!(
            result.clicks.events[0].start_secs,
            start as f64 / rate as f64
        );
        assert_eq!(
            result.clicks.events[0].duration_secs,
            width as f64 / rate as f64
        );
    }
}
