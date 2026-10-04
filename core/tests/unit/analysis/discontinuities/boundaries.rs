use super::*;

#[test]
fn shortest_one_sample_pulse_with_full_context_has_a_reading() {
    for rate in [8_000, 11_025, 44_100, 48_000, 96_000, 768_000] {
        let context = (rate as usize).div_ceil(200);
        let mut samples = vec![0.0; 2 * context + 3];
        samples[context + 1] = 0.5;
        let result = analyze(rate, 1, &samples).expect("full real context");
        assert_eq!(result.clicks.count, 1, "{rate} Hz");
        assert_eq!(result.clicks.events[0].duration_secs, 1.0 / rate as f64);
        assert!(analyze(rate, 1, &samples[..samples.len() - 1]).is_none());
    }
}

#[test]
fn final_pulses_use_their_actual_width_to_fit_the_right_context() {
    for rate in [8_000, 11_025, 44_100, 48_000, 96_000, 768_000] {
        let context = (rate as usize).div_ceil(200);
        for width in [1, rate as usize / 2_000] {
            let mut samples = vec![0.0; rate as usize / 25 + 7];
            let start = samples.len() - context - width - 1;
            samples[start..start + width].fill(-0.5);
            let result = analyze(rate, 1, &samples).unwrap();
            assert_eq!(result.clicks.count, 1, "{rate} Hz, {width} samples");
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
}

#[test]
fn impulses_require_real_difference_context_at_both_file_boundaries() {
    let rate = 48_000;
    let context = 240;
    let length = 4_801;
    for (at, expected) in [
        (0, 0),
        (context, 0),
        (context + 1, 1),
        (length - context - 2, 1),
        (length - context - 1, 0),
        (length - 1, 0),
    ] {
        let mut samples = vec![0.0; length];
        samples[at] = 0.5;
        assert_eq!(
            analyze(rate, 1, &samples).unwrap().clicks.count,
            expected,
            "{at}"
        );
    }
}

#[test]
fn a_dropout_needs_exactly_one_complete_real_context_on_each_side() {
    let rate = 48_000;
    let context = 240;
    let width = 96;
    for (before, after, expected) in [
        (context, context, 1),
        (context - 1, context, 0),
        (context, context - 1, 0),
        (0, 2 * context, 0),
        (2 * context, 0, 0),
    ] {
        let mut samples = vec![0.125; before + width + after];
        samples[before..before + width].fill(-0.0);
        assert_eq!(analyze(rate, 1, &samples).unwrap().dropouts.count, expected);
    }
}

#[test]
fn the_last_of_32_channels_retains_its_own_event_and_timestamp() {
    let mut detector = DiscontinuityDetector::new(8_000, 32).unwrap();
    for n in 0..1_600 {
        let mut frame = [0.125; 32];
        if n == 400 {
            frame[31] += 0.5;
        }
        if (800..840).contains(&n) {
            frame[30] = 0.0;
        }
        detector.push_frame(&frame);
    }
    let result = detector.finish().unwrap();
    assert_eq!(result.clicks.count, 1);
    assert_eq!(result.clicks.events[0].channel, 32);
    assert_eq!(result.clicks.events[0].start_secs, 0.05);
    assert_eq!(result.dropouts.count, 1);
    assert_eq!(result.dropouts.events[0].channel, 31);
    assert_eq!(result.dropouts.events[0].start_secs, 0.1);
    assert_eq!(result.dropouts.events[0].duration_secs, 0.005);
}
