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
