//! Independent sample-domain defects injected into a WAV, through the same
//! decode, analysis and report assembly path as the desktop application.

use flaccompagnon_core::{analyze_file, ScanOptions};

#[test]
fn pcm_click_and_dropout_survive_decode_and_are_reported_on_the_correct_channel() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("injected.wav");
    let rate = 48_000;
    let mut writer = hound::WavWriter::create(
        &path,
        hound::WavSpec {
            channels: 2,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for n in 0..rate {
        let clean =
            (3_000.0 * (std::f64::consts::TAU * 317.0 * n as f64 / rate as f64).sin()) as i16;
        let left = if n == 12_000 { clean + 20_000 } else { clean };
        // At 317 Hz both 0.55 s and 0.58 s have nonzero boundary amplitude.
        let right = if (26_400..27_840).contains(&n) {
            0
        } else {
            clean
        };
        writer.write_sample(left).unwrap();
        writer.write_sample(right).unwrap();
    }
    writer.finalize().unwrap();
    let result = analyze_file(&path, &ScanOptions::default());
    assert!(result.error.is_none(), "{:?}", result.error);
    let measured = result.discontinuities.expect("PCM is measurable");
    assert_eq!(measured.clicks.count, 1);
    assert_eq!(measured.clicks.events[0].channel, 1);
    assert_eq!(measured.clicks.events[0].start_secs, 0.25);
    assert_eq!(measured.dropouts.count, 1);
    assert_eq!(measured.dropouts.events[0].channel, 2);
    assert_eq!(measured.dropouts.events[0].start_secs, 0.55);
    assert_eq!(measured.dropouts.events[0].duration_secs, 0.03);
}
