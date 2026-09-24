//! Independently specified integer biases through decoding and report assembly.

use flaccompagnon_core::{analyze_file, ScanOptions};
use std::{path::Path, process::Command};

fn write_fixture(path: &Path, rate: u32, biases: &[i16]) {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: biases.len() as u16,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for n in 0..rate / 10 {
        // Balanced square wave plus exactly representable integer biases.
        let ac = if n % 2 == 0 { 8_192 } else { -8_192 };
        for &bias in biases {
            writer.write_sample(ac + bias).unwrap();
        }
    }
    writer.finalize().unwrap();
}

#[test]
fn dc_offset_survives_mono_stereo_and_multichannel_decoding() {
    let dir = tempfile::tempdir().unwrap();
    for biases in [
        vec![256],
        vec![256, -512],
        vec![256, -512, 0, 1_024, -128, 64],
    ] {
        let path = dir.path().join(format!("{}ch.wav", biases.len()));
        write_fixture(&path, 96_000, &biases);
        let result = analyze_file(&path, &ScanOptions::default());
        assert!(result.error.is_none(), "{:?}", result.error);
        let dc = result.dc_offset.unwrap();
        let expected: Vec<f64> = biases
            .iter()
            .map(|&bias| f64::from(bias) / 32_768.0)
            .collect();
        assert_eq!(dc.channel_means, expected);
        assert_eq!(
            dc.max_abs,
            expected.iter().map(|x| x.abs()).fold(0.0, f64::max)
        );
    }
}

#[test]
#[ignore = "requires ffmpeg on PATH"]
fn dc_offset_matches_ffmpeg_and_is_preserved_by_flac_encoding() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("bias.wav");
    let flac = dir.path().join("bias.flac");
    write_fixture(&wav, 48_000, &[256, -512]);
    let result = analyze_file(&wav, &ScanOptions::default());
    assert!(result.error.is_none(), "{:?}", result.error);
    let measured = result.dc_offset.unwrap();
    let output = Command::new("ffmpeg")
        .args(["-hide_banner", "-nostats", "-i"])
        .arg(&wav)
        .args(["-af", "astats=reset=0", "-f", "null", "-"])
        .output()
        .expect("install ffmpeg for this optional reference check");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    // astats prints per-channel means followed by an overall figure. Compare
    // only the channels because the overall aggregation has its own semantics.
    let reference: Vec<f64> = stderr
        .lines()
        .filter_map(|line| {
            line.split_once("DC offset:")
                .and_then(|(_, value)| value.trim().parse().ok())
        })
        .take(2)
        .collect();
    assert_eq!(reference.len(), 2, "{stderr}");
    for (actual, expected) in measured.channel_means.iter().zip(reference) {
        // FFmpeg's human-readable output rounds to six decimal places.
        assert!((actual - expected).abs() <= 0.000_000_51);
    }
    let encoded = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-n", "-i"])
        .arg(&wav)
        .args(["-c:a", "flac"])
        .arg(&flac)
        .output()
        .unwrap();
    assert!(
        encoded.status.success(),
        "{}",
        String::from_utf8_lossy(&encoded.stderr)
    );
    let decoded = analyze_file(&flac, &ScanOptions::default());
    assert!(decoded.error.is_none(), "{:?}", decoded.error);
    assert_eq!(decoded.dc_offset, Some(measured));
}
