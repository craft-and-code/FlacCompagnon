use super::*;
use crate::analysis::{analyzer::StreamAnalyzer, loudness::LoudnessMeter};

fn tone(meter: &mut LoudnessMeter, rate: u32, seconds: f64, level: f64, channels: usize) {
    let amplitude = 10f64.powf(level / 20.0);
    for n in 0..(seconds * rate as f64).round() as u64 {
        let sample =
            (amplitude * (std::f64::consts::TAU * 1000.0 * n as f64 / rate as f64).sin()) as f32;
        meter.push_frame(&[sample, sample][..channels]);
    }
}

fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 0.1,
        "{actual} vs {expected} LUFS"
    );
}

#[test]
fn ebu_3341_file_maxima_cases_10_and_13_are_invariant_to_tone_start() {
    // Table 1 defines 20 shifted files for each timescale; expected Max S/M
    // is -23 LUFS +/-0.1. These offsets expose under-reading on a 100 ms grid.
    for (duration, shift_step) in [(3.0, 0.15), (0.4, 0.02)] {
        for index in 0..20 {
            let mut meter = LoudnessMeter::new(48_000, 2).unwrap();
            let silence = index as f64 * shift_step;
            for _ in 0..(silence * 48_000.0).round() as usize {
                meter.push_frame(&[0.0, 0.0]);
            }
            tone(&mut meter, 48_000, duration, -23.0, 2);
            for _ in 0..48_000 {
                meter.push_frame(&[0.0, 0.0]);
            }
            let maxima = meter.peaks().unwrap();
            let peak = if duration == 3.0 {
                maxima.short_term.unwrap()
            } else {
                maxima.momentary.unwrap()
            };
            near(peak.lufs, -23.0);
            assert!(
                (peak.start_secs - silence).abs() < 0.005,
                "{index}: {peak:?}"
            );
        }
    }
}

#[test]
fn ebu_reference_levels_mono_gain_and_below_gate_audio_remain_measurable() {
    for rate in [44_100, 48_000, 96_000] {
        for channels in [1, 2] {
            for level in [-23.0, -80.0] {
                let mut meter = LoudnessMeter::new(rate, channels).unwrap();
                tone(&mut meter, rate, 3.2, level, channels);
                let peaks = meter.peaks().unwrap();
                let expected = level as f32
                    - if channels == 1 {
                        10.0 * 2.0_f32.log10()
                    } else {
                        0.0
                    };
                near(peaks.momentary.unwrap().lufs, expected);
                near(peaks.short_term.unwrap().lufs, expected);
                if level < -70.0 {
                    assert!(meter.integrated_lufs().is_none());
                }
            }
        }
    }
}

#[test]
fn exact_window_lengths_and_silence_distinguish_missing_from_a_reading() {
    let mut meter = LoudnessMeter::new(48_000, 1).unwrap();
    for n in 0..144_000 {
        let sample = (0.1 * (std::f64::consts::TAU * 1000.0 * n as f64 / 48_000.0).sin()) as f32;
        meter.push_frame(&[sample]);
        if n == 19_198 {
            assert!(meter.peaks().is_none());
        }
        if n == 19_199 {
            assert!(meter.peaks().unwrap().momentary.is_some());
        }
        if n == 143_998 {
            assert!(meter.peaks().unwrap().short_term.is_none());
        }
    }
    assert!(meter.peaks().unwrap().short_term.is_some());
    let mut silent = LoudnessMeter::new(48_000, 2).unwrap();
    for _ in 0..200_000 {
        silent.push_frame(&[0.0, 0.0]);
    }
    assert!(silent.peaks().is_none());
    for (rate, channels) in [(7999, 2), (768001, 2), (48000, 0), (48000, 6)] {
        assert!(LoudnessMeter::new(rate, channels).is_none());
    }
}

#[test]
fn malformed_tail_invalidates_both_maxima_including_an_empty_frame() {
    for bad in [
        vec![],
        vec![0.0],
        vec![0.0, 0.0, 0.0],
        vec![f32::NAN, 0.0],
        vec![f32::INFINITY, 0.0],
    ] {
        let mut meter = LoudnessMeter::new(48_000, 2).unwrap();
        tone(&mut meter, 48_000, 3.1, -23.0, 2);
        assert!(meter.peaks().unwrap().short_term.is_some());
        meter.push_frame(&bad);
        assert!(meter.peaks().is_none());
    }
    let mut analyzer = StreamAnalyzer::new(2, 48_000);
    for n in 0..48_000 {
        let sample = (0.1 * (std::f64::consts::TAU * 1000.0 * n as f64 / 48_000.0).sin()) as f32;
        analyzer.push_frame(&[sample, sample], None);
    }
    analyzer.push_frame(&[], None);
    assert!(analyzer.finish(48_000, None).loudness_peaks.is_none());
}

#[test]
fn a_short_real_file_has_no_short_term_reading() {
    let mut analyzer = StreamAnalyzer::new(2, 48_000);
    for n in 0..120_000 {
        let sample = (0.1 * (std::f64::consts::TAU * 1000.0 * n as f64 / 48_000.0).sin()) as f32;
        analyzer.push_frame(&[sample, sample], None);
    }
    let peaks = analyzer.finish(48_000, None).loudness_peaks.unwrap();
    assert!(peaks.momentary.is_some());
    assert!(peaks.short_term.is_none());
}

#[test]
fn lra_synthetic_tail_cannot_raise_the_exported_loudness_maxima() {
    let mut analyzer = StreamAnalyzer::new(2, 48_000);
    let mut real = LoudnessMeter::new(48_000, 2).unwrap();
    for n in 0..148_800 {
        let sample = if n == 148_799 { 0.5 } else { 0.0 };
        analyzer.push_frame(&[sample, sample], None);
        real.push_frame(&[sample, sample]);
    }
    let before = real.peaks().unwrap();
    // The last real sample excites the K filter. Feeding the LRA-only silence
    // produces a measurable filter tail that is outside the real programme.
    for _ in 0..72_000 {
        real.push_frame(&[0.0, 0.0]);
    }
    assert!(real.peaks().unwrap().momentary.unwrap().lufs > before.momentary.unwrap().lufs);
    assert_eq!(analyzer.finish(48_000, None).loudness_peaks, Some(before));
}

#[test]
fn off_grid_last_window_is_included_without_synthetic_tail_frames() {
    let mut meter = LoudnessPeaksMeter::new(10, 4, 30);
    meter.push(1000.0, 1000.0, 3); // incomplete despite a large sum
    assert!(meter.result(true, true).is_none());
    meter.push(4.0, 4.0, 4);
    meter.push(8.0, 8.0, 5); // the final complete window starts at 0.1 s
    meter.push(8.0, 8.0, 6); // an exact tie retains the first position
    let peak = meter.result(true, true).unwrap().momentary.unwrap();
    near(peak.lufs, -0.691 + 10.0 * 2.0_f32.log10());
    assert_eq!(peak.start_secs, 0.1);
}

#[test]
fn compact_short_term_ring_overflow_withholds_only_the_unrepresentable_reading() {
    let mut meter = LoudnessMeter::new(48_000, 2).unwrap();
    meter.push_frame(&[f32::MAX, -f32::MAX]);
    for _ in 0..150_000 {
        meter.push_frame(&[0.0, 0.0]);
    }
    let peaks = meter.peaks().unwrap();
    assert!(peaks.momentary.unwrap().lufs.is_finite());
    assert!(peaks.short_term.is_none());
}
