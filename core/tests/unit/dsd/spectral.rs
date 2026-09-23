use super::*;

#[test]
fn dsd_heritage_in_hires_pcm_is_detected() {
    // 192 kHz PCM, FFT 8192: mimic the measured DSD-sourced profile —
    // content valley ~-85 dB at 22-30 kHz, noise ramp ~-55 dB at 36-70 kHz.
    let fft = 8192usize;
    let rate = 192_000u32;
    let bin_hz = rate as f64 / fft as f64;
    let mut spec = vec![-85.0f32; fft / 2 + 1];
    for (i, v) in spec.iter_mut().enumerate() {
        let f = i as f64 * bin_hz;
        if f < 20_000.0 {
            *v = -50.0;
        } else if f > 34_000.0 {
            *v = -55.0;
        }
    }
    let rise = dsd_heritage_check(&spec, rate, fft).expect("detected");
    assert!(rise > 15.0, "rise {rise}");
    // Genuine hi-res PCM: monotonic decay, no ultrasonic rise.
    let genuine: Vec<f32> = (0..=fft / 2)
        .map(|i| -40.0 - 60.0 * (i as f32 / (fft / 2) as f32))
        .collect();
    assert!(dsd_heritage_check(&genuine, rate, fft).is_none());
}

#[test]
fn pcm_cliff_is_flagged_and_native_is_not() {
    // Decoded rate 352.8 kHz, FFT 8192 -> Nyquist 176.4 kHz over 4097 bins.
    let fft = 8192usize;
    let rate = 352_800u32;
    let nbins = fft / 2 + 1;
    let bin_hz = rate as f64 / fft as f64;
    // PCM-sourced: content 0 dB up to 22.05 kHz, -55 dB above, noise ramp later.
    let mut fake = vec![-55.0f32; nbins];
    for (i, v) in fake.iter_mut().enumerate() {
        if (i as f64 * bin_hz) < 22_050.0 {
            *v = 0.0;
        }
    }
    let hit = pcm_source_check(&fake, rate, fft).expect("flagged");
    assert!((hit.boundary_hz - 22_050.0).abs() < 1.0);
    assert!(hit.drop_db > 30.0);
    // Native-like: gentle 3 dB step into the noise shaping.
    let mut native = vec![-3.0f32; nbins];
    for (i, v) in native.iter_mut().enumerate() {
        if (i as f64 * bin_hz) < 22_050.0 {
            *v = 0.0;
        }
    }
    assert!(pcm_source_check(&native, rate, fft).is_none());
}

/// Degenerate inputs reach these from real files (a zero-length spectrum
/// when a track decoded to nothing, `fft_size` 0 from an uninitialised
/// analyzer). Both must answer "no finding", not divide by zero or index
/// out of bounds.
#[test]
fn degenerate_inputs_are_rejected_without_panicking() {
    assert!(pcm_source_check(&[], 352_800, 8192).is_none());
    assert!(pcm_source_check(&[0.0; 4097], 352_800, 0).is_none());
    assert!(pcm_source_check(&[0.0; 4097], 0, 8192).is_none());
    assert!(dsd_heritage_check(&[], 192_000, 8192).is_none());
    assert!(dsd_heritage_check(&[0.0; 4097], 192_000, 0).is_none());
    // Spectrum far too short for the bands these look at.
    assert!(pcm_source_check(&[0.0; 20], 352_800, 8192).is_none());
    assert!(dsd_heritage_check(&[0.0; 20], 192_000, 8192).is_none());
}

/// A spectrum of all-NaN (possible when a silent track produces log(0))
/// must not be reported as a finding.
#[test]
fn non_finite_spectrum_yields_no_finding() {
    let nan = vec![f32::NAN; 4097];
    assert!(pcm_source_check(&nan, 352_800, 8192).is_none());
    assert!(dsd_heritage_check(&nan, 192_000, 8192).is_none());
}
