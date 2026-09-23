use super::*;

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
