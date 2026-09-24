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
