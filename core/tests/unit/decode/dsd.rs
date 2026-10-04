use super::*;
use std::io::Cursor;

fn selection() -> crate::AnalysisSelection {
    crate::AnalysisSelection::from_names(["dc-offset"]).expect("known analysis")
}

#[test]
fn pcm_frames_split_across_pipe_reads_keep_channel_alignment() {
    // 65536 bytes per read cannot hold a whole number of 3-channel frames.
    let samples = [0.25f32, -0.5, 0.125].repeat(6000);
    let bytes: Vec<_> = samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    let (analyzer, count) = analyze_pcm_output(&mut Cursor::new(bytes), 3, 352_800, selection())
        .expect("complete float frames");
    assert_eq!(count, 6000);
    assert_eq!(
        analyzer
            .finish(352_800, None)
            .dc_offset
            .expect("channel means")
            .channel_means,
        vec![0.25, -0.5, 0.125]
    );
}

#[test]
fn successful_ffmpeg_exit_cannot_hide_a_partial_pcm_frame() {
    let mut bytes = 0.25f32.to_le_bytes().to_vec();
    bytes.push(0);
    assert!(analyze_pcm_output(&mut Cursor::new(bytes), 1, 352_800, selection()).is_err());
}

#[test]
fn non_finite_ffmpeg_samples_are_rejected() {
    for sample in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(analyze_pcm_output(
            &mut Cursor::new(sample.to_le_bytes()),
            1,
            352_800,
            selection(),
        )
        .is_err());
    }
}

#[test]
fn invalid_dsd_parameters_are_rejected_before_spawning_ffmpeg() {
    for (channels, rate) in [(0, 352_800), (usize::MAX, 352_800), (2, 0)] {
        assert!(decode_and_analyze_dsd(
            "nonexistent-ffmpeg",
            Path::new("missing.dsf"),
            channels,
            rate,
        )
        .is_err());
    }
}
