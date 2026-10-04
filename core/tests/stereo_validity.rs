//! A malformed trailing frame must not publish the valid stereo prefix.

use flaccompagnon_core::{analysis::analyzer::StreamAnalyzer, AnalysisSelection};

#[test]
fn stereo_measurements_withhold_the_valid_prefix_after_an_invalid_frame() {
    let selection = AnalysisSelection::from_names(["stereo", "phase"]).unwrap();
    for frame in [
        vec![],
        vec![0.0],
        vec![0.0, 0.0, 0.0],
        vec![f32::NAN, 0.0],
        vec![0.0, f32::INFINITY],
    ] {
        for right in [0.125_f32, -0.25] {
            let mut analyzer = StreamAnalyzer::new_selected(2, 48_000, selection);
            for n in 0..64 {
                let sign = if n % 2 == 0 { 1.0 } else { -1.0 };
                analyzer.push_frame(&[sign * 0.125, sign * right], None);
            }
            analyzer.push_frame(&frame, None);
            // Subsequent valid frames cannot make the invalid stream valid again.
            analyzer.push_frame(&[0.125, right], None);
            let result = analyzer.finish(48_000, None);
            assert!(result.stereo_balance.is_none(), "{frame:?}");
            assert!(result.phase_correlation.is_none(), "{frame:?}");
            assert!(!result.phase_inverted, "{frame:?}");
            assert!(!result.fake_stereo, "{frame:?}");
            assert!(result.local_phase.is_none(), "{frame:?}");
        }
    }
}

#[test]
fn finite_stereo_frames_keep_the_analytical_balance_and_opposition() {
    let selection = AnalysisSelection::from_names(["stereo", "phase"]).unwrap();
    let mut analyzer = StreamAnalyzer::new_selected(2, 48_000, selection);
    for n in 0..64 {
        let left = if n % 2 == 0 { 0.125 } else { -0.125 };
        analyzer.push_frame(&[left, -2.0 * left], None);
    }
    let result = analyzer.finish(48_000, None);
    assert_eq!(result.phase_correlation, Some(-1.0));
    assert!(result.phase_inverted);
    let Some(flaccompagnon_core::analysis::stereo::StereoBalance::Measured {
        right_minus_left_db,
    }) = result.stereo_balance
    else {
        panic!("finite two-channel energy");
    };
    assert!((right_minus_left_db - 6.0206).abs() < 1e-4);
}

#[test]
fn quiet_unequal_channels_do_not_become_dual_mono() {
    let selection = AnalysisSelection::from_names(["stereo"]).unwrap();
    // R=2L has a difference/signal power ratio of 1/5, independently of gain.
    // R=1.0005L has a ratio near 1.25e-7, below the existing -60 dB floor.
    for gain in [1e-20_f32, 1e-10, 1.0, 1e20] {
        for (right_gain, expected) in [(2.0, false), (1.0005, true), (1.0, true)] {
            let mut analyzer = StreamAnalyzer::new_selected(2, 48_000, selection);
            for n in 0..64 {
                let left = if n % 2 == 0 { gain } else { -gain };
                analyzer.push_frame(&[left, right_gain * left], None);
            }
            assert_eq!(
                analyzer.finish(48_000, None).fake_stereo,
                expected,
                "gain={gain}, R/L={right_gain}"
            );
        }
    }
}
