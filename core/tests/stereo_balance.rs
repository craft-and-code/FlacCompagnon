//! Independently specified channel gains and biases through WAV decoding.

use flaccompagnon_core::{
    analysis::stereo::StereoBalance, analyze_file_selected, AnalysisSelection, ScanOptions,
};
use std::path::Path;

fn write_float(path: &Path, frames: &[[f32; 2]]) {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .unwrap();
    for frame in frames {
        for sample in frame {
            writer.write_sample(*sample).unwrap();
        }
    }
    writer.finalize().unwrap();
}

#[test]
fn decoded_short_float_audio_keeps_balance_and_correlation_at_quiet_and_above_full_scale_gains() {
    let dir = tempfile::tempdir().unwrap();
    let selection = AnalysisSelection::from_names(["stereo", "phase"]).unwrap();
    for gain in [1e-12_f32, 0.125, 2.0] {
        let path = dir.path().join(format!("gain-{gain}.wav"));
        write_float(&path, &[[gain, -2.0 * gain], [-gain, 2.0 * gain]].repeat(8));
        let result = analyze_file_selected(&path, &ScanOptions::default(), selection);
        assert!(result.error.is_none(), "{:?}", result.error);
        let Some(StereoBalance::Measured {
            right_minus_left_db,
        }) = result.stereo_balance
        else {
            panic!("both float channels contain signal");
        };
        assert!((right_minus_left_db - 6.0206).abs() < 1e-4);
        assert_eq!(result.phase_correlation, Some(-1.0));
        assert_eq!(result.phase_inverted, Some(true));
        assert!(
            result.local_phase.is_none(),
            "sixteen frames are not a complete local window"
        );
    }
}

#[test]
fn decoded_balance_preserves_raw_dc_energy_and_exact_silence_states() {
    let dir = tempfile::tempdir().unwrap();
    let selection = AnalysisSelection::from_names(["stereo"]).unwrap();
    for (name, frames, expected) in [
        (
            "dc",
            vec![[0.25, 0.75], [-0.25, 0.25]],
            Some(StereoBalance::Measured {
                right_minus_left_db: (10.0 * 5.0_f64.log10()) as f32,
            }),
        ),
        (
            "left-silent",
            vec![[0.0, 0.25], [0.0, -0.25]],
            Some(StereoBalance::LeftSilent),
        ),
        (
            "right-silent",
            vec![[0.25, 0.0], [-0.25, 0.0]],
            Some(StereoBalance::RightSilent),
        ),
        ("both-silent", vec![[0.0, 0.0]; 2], None),
    ] {
        let path = dir.path().join(format!("{name}.wav"));
        write_float(&path, &frames);
        let result = analyze_file_selected(&path, &ScanOptions::default(), selection);
        assert!(result.error.is_none(), "{:?}", result.error);
        assert_eq!(result.stereo_balance, expected);
    }
}
