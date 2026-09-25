//! Independent PCM controls and an actual encoder exercise the production sweep.
use flaccompagnon_core::{
    decode::decode_to_pcm,
    transcode::{aac, mp3, AacParams, Mp3Params},
};
use flaccompagnon_services::convert::{convert_file, ConvertFormat, ConvertSettings};

fn noise(n: usize, mut state: u32) -> Vec<f64> {
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let sample = state as f64 / u32::MAX as f64 * 0.8 - 0.4;
            (sample * 8_388_608.0).round() / 8_388_608.0
        })
        .collect()
}

#[test]
fn one_lsb_stereo_difference_is_not_mp3_evidence() {
    let left = noise(66 * 1024, 0x2545_F491);
    let difference = noise(left.len(), 0xABCD_EF12);
    let right = left
        .iter()
        .zip(difference)
        .map(|(l, d)| {
            l + if d > 0.0 {
                1.0 / 8_388_608.0
            } else {
                -1.0 / 8_388_608.0
            }
        })
        .collect();
    // This PCM was synthesized directly, never passed through a lossy codec.
    // Before rejecting all-zero quantization trials its Side scored 1.0.
    let evidence = mp3::detect(&[left, right], 44_100, &Mp3Params::default(), &|| false).unwrap();
    assert!(!evidence.detected, "{evidence:?}");
}

#[test]
fn one_aac_frame_cannot_use_the_sixty_four_frame_threshold() {
    let signal = noise(3 * 1024, 0x2545_F491);
    assert!(aac::detect(&[signal], 44_100, &AacParams::default(), &|| false).is_none());
}

#[test]
fn an_actual_mp3_encoder_still_leaves_detectable_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("control.wav");
    let encoded = dir.path().join("encoded.mp3");
    let signal = noise(66 * 1024, 0x2545_F491);
    write_pcm(&source, &signal);
    let params = Mp3Params::default();
    assert!(
        !mp3::detect(&[signal], 44_100, &params, &|| false)
            .unwrap()
            .detected
    );
    convert_file(
        &source,
        &encoded,
        &ConvertSettings {
            format: ConvertFormat::Mp3,
            bitrate_kbps: Some(128),
            flac_effort: Default::default(),
            preserve_modtime: false,
        },
        &|| false,
    )
    .unwrap();
    let pcm = decode_to_pcm(&encoded).unwrap();
    assert_eq!(pcm.channels, 1);
    let decoded = pcm.samples.iter().map(|v| f64::from(*v)).collect();
    let evidence = mp3::detect(&[decoded], pcm.sample_rate, &params, &|| false).unwrap();
    assert!(evidence.detected, "{evidence:?}");
}

fn write_pcm(path: &std::path::Path, signal: &[f64]) {
    write_pcm_at_rate(path, signal, 44_100);
}

fn write_pcm_at_rate(path: &std::path::Path, signal: &[f64], sample_rate: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 24,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for value in signal {
        writer
            .write_sample((value * 8_388_608.0).round() as i32)
            .unwrap();
    }
    writer.finalize().unwrap();
}

#[test]
fn an_incomplete_codec_search_keeps_its_reason_without_a_third_verdict() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("short.wav");
    // Enough for all MP3 frames, but not the AAC population.
    write_pcm(&source, &noise(8 * 1024, 0x2545_F491));
    let result = flaccompagnon_core::analyze_file(&source, &Default::default());
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(result.detections.summary, "Clean");
    assert!(result.lattice_score.is_none());
    assert!(result.detections.detail.contains("too short"));
}

#[test]
fn native_high_rate_pcm_stays_clean_when_codec_checks_do_not_apply() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("native-high-rate.wav");
    // Native, full-band 24-bit integer PCM: no padding or bandwidth signature.
    let signal = noise(16 * 1024, 0x2545_F491);
    for rate in [88_200, 96_000, 192_000] {
        write_pcm_at_rate(&source, &signal, rate);
        let result = flaccompagnon_core::analyze_file(&source, &Default::default());
        assert!(result.error.is_none(), "{:?}", result.error);
        assert_eq!(result.real_bit_depth, Some(24));
        assert_eq!(result.detections.summary, "Clean", "rate {rate}");
        assert!(
            !result.detections.upscaling
                && !result.detections.upsampling
                && !result.detections.transcoding
        );
        assert!(result.lattice_score.is_none());
        assert!(result.detections.detail.contains("not applicable"));
    }
}
