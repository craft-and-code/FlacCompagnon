use super::*;

/// Quarter-rate sine at phase π/4: samples sit at ±0.693 but the true crest
/// is 0.98. NumPy ground truth for this filter: 0.97135.
#[test]
fn recovers_inter_sample_peak() {
    let fs = 48_000usize;
    let mut tp = TruePeak::new(1);
    let mut sample_peak = 0.0f32;
    for n in 0..fs {
        // A quarter-rate sine is exactly periodic every 4 samples, so its
        // phase can be computed from n % 4 instead of the raw n * rate
        // formula. That matters here: for a full second at 48 kHz the raw
        // formula's angle argument grows past 75 000 radians, and f32
        // loses enough precision at that magnitude to visibly perturb
        // sin() (this was the actual cause of a prior test flake, not the
        // filter under test). Wrapping keeps the angle small and exact.
        let phase = std::f32::consts::FRAC_PI_2 * (n % 4) as f32 + std::f32::consts::FRAC_PI_4;
        let x = 0.98 * phase.sin();
        sample_peak = sample_peak.max(x.abs());
        tp.push_frame(&[x]);
    }
    assert!(sample_peak < 0.70, "sample peak {sample_peak}");
    let peak = tp.peak();
    assert!(
        (peak - 0.97135).abs() < 0.002,
        "true peak {peak}, expected ≈0.97135"
    );
}

/// Hard-clipped sine: reconstruction overshoots full scale (true peak > 1),
/// NumPy ground truth 1.01199.
#[test]
fn clipped_material_reads_over() {
    let fs = 48_000usize;
    let mut tp = TruePeak::new(1);
    for n in 0..fs {
        let x = (1.4 * (2.0 * std::f32::consts::PI * 997.0 * n as f32 / fs as f32).sin())
            .clamp(-1.0, 1.0);
        tp.push_frame(&[x]);
    }
    let peak = tp.peak();
    assert!(
        (peak - 1.01199).abs() < 0.002,
        "true peak {peak}, expected ≈1.01199"
    );
    assert!(
        tp.peak_dbtp() > 0.0,
        "dBTP {} should be positive",
        tp.peak_dbtp()
    );
}

/// Benign smooth material: true peak stays close to the sample peak
/// (no phantom overshoot from the filter itself).
#[test]
fn benign_material_matches_sample_peak() {
    let fs = 48_000usize;
    let mut tp = TruePeak::new(2);
    let mut sample_peak = 0.0f32;
    for n in 0..fs {
        // 100 Hz sine, samples land densely on the crest.
        let x = 0.5 * (2.0 * std::f32::consts::PI * 100.0 * n as f32 / fs as f32).sin();
        sample_peak = sample_peak.max(x.abs());
        tp.push_frame(&[x, x]);
    }
    let ratio = tp.peak() / sample_peak;
    assert!((0.98..=1.05).contains(&ratio), "ratio {ratio}");
}

#[test]
fn final_samples_receive_the_full_reconstruction_filter() {
    // Independent zero-stuffing/convolution reference: derive the Blackman
    // sinc kernel, rather than reusing the production polyphase tap table.
    let mut taps: Vec<f64> = (0..48)
        .map(|k| {
            let x = (k as f64 - 23.5) / 4.0;
            let sinc = (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x);
            let phase = std::f64::consts::TAU * k as f64 / 47.0;
            sinc * (0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos())
        })
        .collect();
    for phase in 0..4 {
        let gain: f64 = taps.iter().skip(phase).step_by(4).sum();
        for tap in taps.iter_mut().skip(phase).step_by(4) {
            *tap /= gain;
        }
    }
    let samples = [0.9f64, 0.9];
    let mut reconstructed = vec![0.0f64; 4 * samples.len() + taps.len()];
    for (frame, sample) in samples.iter().enumerate() {
        for (tap, weight) in taps.iter().enumerate() {
            reconstructed[frame * 4 + tap] += sample * weight;
        }
    }
    let reference = reconstructed.iter().map(|s| s.abs()).fold(0.0f64, f64::max);
    assert!(reference > 1.09);

    let mut meter = TruePeak::new(1);
    for sample in samples {
        meter.push_frame(&[sample as f32]);
    }
    meter.flush_tail();
    assert!((f64::from(meter.peak()) - reference).abs() < 0.0002);
}

#[test]
fn true_peak_never_underreports_a_stored_impulse() {
    let mut meter = TruePeak::new(1);
    meter.push_frame(&[1.0]);
    meter.flush_tail();
    assert!(meter.peak() >= 1.0);
}
