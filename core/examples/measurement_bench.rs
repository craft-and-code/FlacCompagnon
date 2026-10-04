//! Repeatable CPU comparison of the release-audit meters, without decoding.
//! Run `cargo run -p flaccompagnon-core --release --example measurement_bench`.
//! Fixture creation is excluded; meter construction and finalization are included.

use std::{hint::black_box, time::Instant};

use flaccompagnon_core::{analysis::analyzer::StreamAnalyzer, AnalysisSelection};

const SECONDS: u32 = 10;
const REPEATS: usize = 3;
const CASES: [(&str, &[&str]); 8] = [
    ("empty-loop", &[]),
    ("balance", &["stereo"]),
    ("local-band-phase", &["phase"]),
    ("hf-stereo", &["hf-stereo"]),
    ("dc-offset", &["dc-offset"]),
    ("impulses", &["clicks"]),
    ("dropouts", &["dropouts"]),
    ("lufs-m-s-lra", &["loudness"]),
];

fn fixture(rate: u32) -> Vec<[f32; 2]> {
    let mut samples: Vec<_> = (0..rate * SECONDS)
        .map(|frame| {
            let time = frame as f64 / rate as f64;
            let sine = |hz: f64| (std::f64::consts::TAU * hz * time).sin();
            let gain = if time < 5.0 { 1.0 } else { 0.5 };
            let mid = gain * (0.12 * sine(97.0) + 0.08 * sine(997.0) + 0.03 * sine(9_001.0));
            let side = gain * 0.04 * sine(2_351.0);
            [
                (mid + side + 0.001) as f32,
                (0.8 * mid - side - 0.001) as f32,
            ]
        })
        .collect();
    // Exercise event publication as well as ordinary controls. This fixture
    // benchmarks processing cost; it is not a ground-truth accuracy corpus.
    samples[(rate * 3) as usize][0] += 0.8;
    for frame in &mut samples[(rate * 7) as usize..(rate * 7 + rate / 100) as usize] {
        frame[0] = 0.0;
    }
    samples
}

fn measure(rate: u32, selection: AnalysisSelection, samples: &[[f32; 2]]) -> f64 {
    let start = Instant::now();
    let mut analyzer = StreamAnalyzer::new_selected(2, rate, selection);
    for frame in samples {
        analyzer.push_frame(black_box(frame), None);
    }
    black_box(analyzer.finish(rate, None));
    start.elapsed().as_secs_f64()
}

fn main() -> Result<(), String> {
    println!("rate_hz,case,audio_seconds,median_ms,realtime_factor");
    for rate in [44_100, 48_000, 96_000, 192_000] {
        let samples = fixture(rate);
        for (name, names) in CASES.into_iter().chain(std::iter::once((
            "combined",
            &[
                "stereo",
                "phase",
                "hf-stereo",
                "dc-offset",
                "clicks",
                "dropouts",
                "loudness",
            ][..],
        ))) {
            let selection = AnalysisSelection::from_names(names.iter().copied())?;
            black_box(measure(rate, selection, &samples));
            let mut elapsed =
                std::array::from_fn::<_, REPEATS, _>(|_| measure(rate, selection, &samples));
            elapsed.sort_by(f64::total_cmp);
            let seconds = elapsed[REPEATS / 2];
            println!(
                "{rate},{name},{SECONDS},{:.3},{:.6}",
                seconds * 1_000.0,
                seconds / SECONDS as f64
            );
        }
    }
    Ok(())
}
