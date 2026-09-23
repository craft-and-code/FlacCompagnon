use super::*;

fn analyze(rate: u32, channels: usize, samples: &[f32]) -> Option<DiscontinuityAnalysis> {
    let mut detector = DiscontinuityDetector::new(rate, channels).unwrap();
    for frame in samples.chunks(channels) {
        detector.push_frame(frame);
    }
    detector.finish()
}

fn tone(rate: u32, seconds: f64, frequency: f64, amplitude: f64) -> Vec<f32> {
    (0..(rate as f64 * seconds) as usize)
        .map(|n| {
            (amplitude * (std::f64::consts::TAU * frequency * n as f64 / rate as f64).sin()) as f32
        })
        .collect()
}

#[test]
fn injected_pulses_have_correct_locations_widths_and_channels_at_common_rates() {
    for rate in [8_000, 11_025, 44_100, 48_000, 96_000, 192_000, 768_000] {
        for width in [1, (rate / 5_000).max(1) as usize] {
            let mut clean = tone(rate, 0.1, 317.0, 0.02);
            let start = (rate / 20) as usize + 3;
            for sample in &mut clean[start..start + width] {
                *sample += 0.6;
            }
            // The other channel's opposing polarity must not cancel evidence.
            let stereo: Vec<_> = clean.iter().flat_map(|&sample| [sample, -sample]).collect();
            let result = analyze(rate, 2, &stereo).unwrap();
            assert_eq!(result.clicks.count, 2, "{rate} Hz, width {width}");
            assert_eq!(result.dropouts.count, 0);
            for (event, channel) in result.clicks.events.iter().zip([1, 2]) {
                assert_eq!(event.channel, channel);
                assert!((event.start_secs - start as f64 / rate as f64).abs() < 1e-9);
                assert!((event.duration_secs - width as f64 / rate as f64).abs() < 1e-9);
            }
        }
    }
}

#[test]
fn pulse_across_processing_boundary_is_not_lost_or_counted_twice() {
    for start in 711..735 {
        let mut samples = vec![0.0; 2_011]; // final partial processing batch
        samples[start..start + 10].fill(-0.7);
        let result = analyze(48_000, 1, &samples).unwrap();
        assert_eq!(result.clicks.count, 1, "start {start}");
        assert_eq!(result.clicks.events[0].start_secs, start as f64 / 48_000.0);
    }
}

#[test]
fn dropout_is_a_bounded_zero_run_inside_signal_not_a_zero_crossing() {
    for rate in [8_000, 44_100, 48_000, 96_000, 192_000] {
        let mut samples = tone(rate, 0.2, 317.0, 0.4);
        let start = (rate / 20) as usize;
        let end = (rate * 8 / 100) as usize;
        samples[start..end].fill(0.0);
        let result = analyze(rate, 1, &samples).unwrap();
        assert_eq!(result.dropouts.count, 1, "{rate}");
        assert_eq!(result.clicks.count, 0);
        assert_eq!(result.dropouts.events[0].channel, 1);
        assert_eq!(
            result.dropouts.events[0].start_secs,
            start as f64 / rate as f64
        );
        assert_eq!(
            result.dropouts.events[0].duration_secs,
            (end - start) as f64 / rate as f64
        );
    }
}

#[test]
fn silence_file_boundaries_long_pauses_and_fades_are_not_dropout_candidates() {
    let rate = 48_000;
    let silence = vec![0.0; rate as usize];
    let mut boundaries = silence.clone();
    boundaries[4_800..43_200].fill(0.3);
    let mut long_pause = vec![0.3; rate as usize];
    long_pause[12_000..36_000].fill(0.0);
    let mut fade = vec![0.3; rate as usize];
    fade[18_000..24_000].fill(0.0);
    for n in 0..480 {
        fade[17_520 + n] = 0.3 * (479 - n) as f32 / 480.0;
        fade[24_000 + n] = 0.3 * n as f32 / 480.0;
    }
    for samples in [silence, boundaries, long_pause, fade] {
        assert_eq!(
            analyze(rate, 1, &samples).unwrap(),
            DiscontinuityAnalysis::default()
        );
    }
}

#[test]
fn clean_tones_square_waves_noise_and_damped_musical_attacks_are_not_clicks() {
    let rate = 48_000;
    let low = tone(rate, 0.4, 20.0, 0.7);
    let high = tone(rate, 0.4, 10_000.0, 0.7);
    let square: Vec<_> = tone(rate, 0.4, 440.0, 0.7)
        .iter()
        .map(|s| if *s >= 0.0 { 0.7 } else { -0.7 })
        .collect();
    let mut seed = 123u32;
    let noise: Vec<_> = (0..19_200)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed as f64 / u32::MAX as f64 * 0.6 - 0.3) as f32
        })
        .collect();
    let attack: Vec<_> = (0..19_200)
        .map(|n| {
            let t = (n as f64 / rate as f64 - 0.05).max(0.0);
            (0.8 * (std::f64::consts::TAU * 997.0 * t).sin() * (-t / 0.03).exp()) as f32
        })
        .collect();
    for (name, samples) in [
        ("low", low),
        ("high", high),
        ("square", square),
        ("noise", noise),
        ("attack", attack),
    ] {
        let result = analyze(rate, 1, &samples).unwrap();
        assert_eq!(result.clicks.count, 0, "{name}");
        assert_eq!(result.dropouts.count, 0, "{name}");
    }
}

#[test]
fn event_storage_is_bounded_but_counts_and_earliest_channel_order_are_preserved() {
    let mut samples = vec![0.0; 48_000];
    for n in (960..47_000).step_by(960) {
        samples[n] = 0.8;
    }
    let stereo: Vec<_> = samples.iter().flat_map(|&s| [s, s]).collect();
    let result = analyze(48_000, 2, &stereo).unwrap();
    assert_eq!(result.clicks.count, 96);
    assert_eq!(result.clicks.events.len(), MAX_LOCATIONS);
    assert_eq!(result.clicks.events[0].channel, 1);
    assert_eq!(result.clicks.events[1].channel, 2);
    assert_eq!(result.clicks.events.last().unwrap().start_secs, 0.32);
}

#[test]
fn malformed_short_or_unsupported_streams_have_no_reading() {
    assert!(DiscontinuityDetector::new(0, 2).is_none());
    assert!(DiscontinuityDetector::new(u32::MAX, 2).is_none());
    assert!(DiscontinuityDetector::new(48_000, 0).is_none());
    assert!(DiscontinuityDetector::new(48_000, 33).is_none());
    assert!(analyze(48_000, 1, &[0.0; 10]).is_none());
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut samples = vec![0.1; 4_800];
        samples.push(invalid);
        assert!(analyze(48_000, 1, &samples).is_none());
    }
    let mut detector = DiscontinuityDetector::new(48_000, 2).unwrap();
    detector.push_frame(&[0.2]);
    assert!(detector.finish().is_none());
}

#[test]
fn a_click_burst_crossing_a_batch_boundary_keeps_its_single_count() {
    let mut samples = vec![0.0; 2_000];
    samples[719] = 0.8;
    samples[740] = -0.8;
    assert_eq!(analyze(48_000, 1, &samples).unwrap().clicks.count, 1);
}

#[test]
fn a_near_end_click_is_processed_when_real_context_is_available() {
    let mut samples = vec![0.0; 2_011];
    samples[1_700] = 0.8;
    let result = analyze(48_000, 1, &samples).unwrap();
    assert_eq!(result.clicks.count, 1);
    assert_eq!(result.clicks.events[0].start_secs, 1_700.0 / 48_000.0);
}

#[test]
fn subthreshold_impulses_and_gaps_outside_duration_bounds_are_not_flagged() {
    let mut small = vec![0.0; 4_800];
    small[2_400] = 0.01;
    assert_eq!(analyze(48_000, 1, &small).unwrap().clicks.count, 0);
    for width in [48, 12_001] {
        let mut samples = vec![0.3; 24_000];
        samples[2_400..2_400 + width].fill(0.0);
        assert_eq!(
            analyze(48_000, 1, &samples).unwrap(),
            DiscontinuityAnalysis::default()
        );
    }
}
