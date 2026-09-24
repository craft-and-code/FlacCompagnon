//! Optional independent check against the installed FFmpeg executable.
//! Run: cargo test -p flaccompagnon-core --test loudness_reference -- --ignored --nocapture
//! No third-party implementation source is used by this test.

use flaccompagnon_core::analysis::loudness::LoudnessMeter;
use std::process::Command;

#[test]
#[ignore = "requires ffmpeg on PATH"]
fn mixed_frequency_loudness_matches_ffmpeg_for_mono_and_stereo() {
    let directory = tempfile::tempdir().unwrap();
    for sample_rate in [44_100, 48_000, 96_000] {
        for channels in [1, 2] {
            let path = directory
                .path()
                .join(format!("{sample_rate}-{channels}.wav"));
            let mut writer = hound::WavWriter::create(
                &path,
                hound::WavSpec {
                    channels,
                    sample_rate,
                    bits_per_sample: 32,
                    sample_format: hound::SampleFormat::Float,
                },
            )
            .unwrap();
            let mut meter = LoudnessMeter::new(sample_rate, channels as usize).unwrap();
            for frame in 0..sample_rate * 60 {
                let time = frame as f64 / sample_rate as f64;
                let level = [-20.0, -26.0, -34.0, -22.0][(time / 15.0) as usize];
                let gain = 10f64.powf(level / 20.0);
                let mut samples = [0.0; 2];
                for (channel, sample) in samples.iter_mut().enumerate().take(channels as usize) {
                    // Cover both K-weighting transitions, unequal channel
                    // power and time-varying levels, not only 1 kHz tones.
                    let sine = |hz: f64| (std::f64::consts::TAU * hz * time).sin();
                    *sample = (gain
                        * (0.5 * sine(80.0)
                            + 0.3 * sine(997.0 + channel as f64 * 103.0)
                            + 0.2 * sine(7_000.0))
                        * if channel == 0 { 1.0 } else { 0.7 })
                        as f32;
                    writer.write_sample(*sample).unwrap();
                }
                meter.push_frame(&samples[..channels as usize]);
            }
            writer.finalize().unwrap();
            let output = Command::new("ffmpeg")
                .args(["-hide_banner", "-nostats", "-i"])
                .arg(&path)
                .args(["-af", "ebur128", "-f", "null", "-"])
                .output()
                .expect("ffmpeg must be installed for this optional test");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(output.status.success(), "{stderr}");
            let summary = stderr.rsplit("Summary:").next().unwrap();
            let reading = |label: &str| -> f32 {
                summary
                    .lines()
                    .find_map(|line| {
                        line.trim()
                            .strip_prefix(label)
                            .and_then(|tail| tail.split_whitespace().next())
                            .and_then(|value| value.parse().ok())
                    })
                    .expect("FFmpeg summary field")
            };
            let integrated = meter.integrated_lufs().unwrap();
            let peaks = meter.peaks().unwrap();
            let reference_max = |label: &str| -> f32 {
                stderr
                    .lines()
                    .filter_map(|line| {
                        line.split_once(label)?
                            .1
                            .split_whitespace()
                            .next()?
                            .parse::<f32>()
                            .ok()
                    })
                    .filter(|value| value.is_finite())
                    .reduce(f32::max)
                    .expect("FFmpeg M/S updates")
            };
            let momentary = peaks.momentary.unwrap().lufs;
            let short_term = peaks.short_term.unwrap().lufs;
            let reference_m = reference_max(" M:");
            let reference_s = reference_max(" S:");
            let range = meter.loudness_range_lu().unwrap();
            let reference_integrated = reading("I:");
            let reference_range = reading("LRA:");
            println!("{sample_rate} Hz/{channels}ch: {integrated:.3} LUFS vs {reference_integrated:.1}; {range:.3} LU vs {reference_range:.1}");
            // EBU Tech 3341/3342 acceptance tolerances. Short-term window
            // alignment and histogram resolution differ between meters.
            assert!((integrated - reference_integrated).abs() <= 0.1);
            assert!((range - reference_range).abs() <= 1.0);
            // These sustained levels have many complete windows at their maxima.
            // FFmpeg logs at 10 Hz; our peak hold checks every decoded frame.
            assert!(
                (momentary - reference_m).abs() <= 0.1,
                "Max M: {momentary} vs {reference_m}"
            );
            assert!(
                (short_term - reference_s).abs() <= 0.1,
                "Max S: {short_term} vs {reference_s}"
            );
            println!("Max M {momentary:.3} vs {reference_m:.1}; Max S {short_term:.3} vs {reference_s:.1} LUFS");
        }
    }
}
