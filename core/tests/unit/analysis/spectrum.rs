use super::*;

#[test]
fn cutoff_of_full_band_noise_is_near_nyquist() {
    let spec = vec![0.0f32; 4097];
    let (hz, ratio) = detect_cutoff(&spec, 44100, 8192);
    assert!(ratio > 0.9, "ratio was {ratio} (hz {hz})");
}

#[test]
fn cutoff_of_band_limited_signal_is_detected() {
    // Content up to bin 2900 (~15.6 kHz at 44.1k/8192), dead above.
    let mut spec = vec![-140.0f32; 4097];
    for s in spec.iter_mut().take(2900) {
        *s = 0.0;
    }
    let (hz, ratio) = detect_cutoff(&spec, 44100, 8192);
    assert!((hz - 15600.0).abs() < 400.0, "hz was {hz}");
    assert!(ratio < 0.75, "ratio was {ratio}");
}

#[test]
fn faint_hf_content_still_counts() {
    // Content that decays to -80 dB near Nyquist is genuine, not a cutoff.
    let mut spec = vec![0.0f32; 4097];
    for (i, s) in spec.iter_mut().enumerate() {
        *s = -80.0 * (i as f32 / 4096.0); // 0 dB at DC down to -80 dB at Nyquist
    }
    let (_, ratio) = detect_cutoff(&spec, 44100, 8192);
    assert!(ratio > 0.9, "ratio was {ratio}");
}

#[test]
fn cliff_is_measured() {
    // 0 dB up to ~15.6 kHz then a drop to -120 dB.
    let mut spec = vec![-120.0f32; 4097];
    for s in spec.iter_mut().take(2900) {
        *s = 0.0;
    }
    let (cliff, above) = cutoff_context(&spec, 44100, 8192, 15600.0);
    assert!(cliff > 80.0, "cliff was {cliff}");
    assert!(above < -100.0, "above was {above}");
}
