use super::*;

#[test]
fn pulse_edges_must_reach_eight_times_the_exact_context_difference_rms() {
    for (injected, expected) in [(0.09375, 0), (0.109375, 1), (0.125, 1)] {
        let mut samples = [0.0078125, -0.0078125].repeat(2_400);
        // The clean differences are exactly 1/64. At this positive sample,
        // the injected edges are (injected + 1/64) and its opposite.
        samples[2_400] += injected;
        assert_eq!(analyze(48_000, 1, &samples).unwrap().clicks.count, expected);
    }
}

#[test]
fn opposing_edge_mismatch_accepts_25_percent_but_rejects_more() {
    for (resumed, expected) in [(0.125, 1), (0.140625, 0)] {
        let mut samples = vec![0.0; 4_800];
        samples[2_400] = 0.5;
        samples[2_401..].fill(resumed);
        assert_eq!(analyze(48_000, 1, &samples).unwrap().clicks.count, expected);
    }
}

#[test]
fn absolute_jump_floors_prevent_gain_invariance_for_quiet_events() {
    for (height, expected) in [(0.03125, 0), (0.0625, 1)] {
        let mut samples = vec![0.0; 4_800];
        samples[2_400] = height;
        assert_eq!(analyze(48_000, 1, &samples).unwrap().clicks.count, expected);
    }
    for (level, expected) in [(0.015625, 0), (0.03125, 1)] {
        let mut samples = vec![level; 4_800];
        samples[2_400..2_496].fill(0.0);
        assert_eq!(
            analyze(48_000, 1, &samples).unwrap().dropouts.count,
            expected
        );
    }
}

#[test]
fn dropout_duration_limits_are_inclusive_at_integer_sample_rates() {
    for rate in [8_000, 11_025, 44_100, 48_000, 96_000, 768_000] {
        let context = (rate as usize).div_ceil(200);
        let minimum = (rate as usize).div_ceil(500);
        let maximum = rate as usize / 4;
        for (width, expected) in [
            (minimum - 1, 0),
            (minimum, 1),
            (maximum, 1),
            (maximum + 1, 0),
        ] {
            let mut samples = vec![0.125; 2 * context + width];
            samples[context..context + width].fill(0.0);
            let result = analyze(rate, 1, &samples).unwrap();
            assert_eq!(
                result.dropouts.count, expected,
                "{rate} Hz, {width} samples"
            );
        }
    }
}

#[test]
fn pulse_width_limit_uses_integer_frames_without_rounding_up() {
    for rate in [8_000, 11_025, 44_100, 48_000, 96_000, 768_000] {
        let maximum = rate as usize / 2_000;
        for (width, expected) in [(maximum, 1), (maximum + 1, 0)] {
            let mut samples = vec![0.0; rate as usize / 10];
            let start = rate as usize / 20;
            samples[start..start + width].fill(0.5);
            assert_eq!(analyze(rate, 1, &samples).unwrap().clicks.count, expected);
        }
    }
}

#[test]
fn resumed_power_ratio_limits_accept_the_boundary_and_reject_large_level_changes() {
    for (resumed, expected) in [(0.03125, 1), (0.5, 1), (0.015625, 0), (1.0, 0)] {
        let mut samples = vec![0.125; 4_800];
        samples[2_400..2_496].fill(0.0);
        samples[2_496..].fill(resumed);
        assert_eq!(
            analyze(48_000, 1, &samples).unwrap().dropouts.count,
            expected
        );
    }
}

#[test]
fn resumed_context_must_be_at_least_90_percent_nonzero() {
    for (zeros, expected) in [(24, 1), (25, 0)] {
        let mut samples = vec![0.125; 4_800];
        samples[2_400..2_496].fill(0.0);
        for offset in (9..240).step_by(10).take(zeros) {
            samples[2_496 + offset] = 0.0;
        }
        if zeros == 25 {
            samples[2_496 + 1] = 0.0;
        }
        assert_eq!(
            analyze(48_000, 1, &samples).unwrap().dropouts.count,
            expected
        );
    }
}

#[test]
fn noisy_or_dc_shifted_mutes_are_not_exact_zero_dropouts() {
    for filling in [f32::MIN_POSITIVE, 0.000001, 0.125] {
        let mut samples = vec![0.5; 4_800];
        samples[2_400..2_496].fill(filling);
        assert_eq!(analyze(48_000, 1, &samples).unwrap().dropouts.count, 0);
    }
}

#[test]
fn a_deliberate_fast_fade_can_still_meet_the_dropout_shape() {
    let mut samples = vec![0.125; 4_800];
    samples[2_400..2_496].fill(0.0);
    // Four-sample linear ramps have 1/32 edges, above the 0.02 floor.
    // There is no information in these samples that can identify intention.
    samples[2_397..2_400].copy_from_slice(&[0.09375, 0.0625, 0.03125]);
    samples[2_496..2_499].copy_from_slice(&[0.03125, 0.0625, 0.09375]);
    assert_eq!(analyze(48_000, 1, &samples).unwrap().dropouts.count, 1);
}
