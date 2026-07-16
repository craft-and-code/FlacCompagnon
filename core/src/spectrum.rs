//! Spectral helpers: averaging accumulated power into a dB spectrum, detecting
//! the content cutoff frequency, and measuring how sharp that cutoff is.
//!
//! The cutoff is found with an **absolute** threshold relative to the spectral
//! peak: any frequency whose averaged level rises above `CONTENT_FLOOR_DB` is
//! considered to carry real content. Genuine lossless audio keeps (often faint)
//! content up to ~20-22 kHz, so its cutoff lands near Nyquist; a lossy source
//! stops abruptly at the encoder's low-pass, leaving a dead zone above it.
//!
//! A cutoff below Nyquist is only strong evidence of transcoding when it is a
//! *cliff* — a large drop into a low, flat dead zone. Gradual roll-off (common
//! in genuine recordings) is reported as merely "suspicious" by the caller.

/// Level, in dB relative to the spectral peak, above which a bin is treated as
/// carrying real content rather than noise / encoder dead-zone.
pub const CONTENT_FLOOR_DB: f32 = -90.0;

/// Convert accumulated per-bin power into an averaged magnitude spectrum in dB,
/// normalized so the strongest bin sits at 0 dB.
pub fn average_to_db(power_acc: &[f64], window_count: u64) -> Vec<f32> {
    if window_count == 0 || power_acc.is_empty() {
        return vec![f32::NEG_INFINITY; power_acc.len()];
    }
    let n = window_count as f64;
    let mag: Vec<f64> = power_acc.iter().map(|p| (p / n).sqrt()).collect();
    let peak = mag.iter().cloned().fold(0.0f64, f64::max).max(1e-30);
    mag.iter()
        .map(|m| (20.0 * (m / peak).max(1e-12).log10()) as f32)
        .collect()
}

/// Detect the content cutoff frequency.
///
/// Returns `(cutoff_hz, cutoff_ratio)` where `cutoff_ratio = cutoff / Nyquist`.
pub fn detect_cutoff(spectrum_db: &[f32], sample_rate: u32, fft_size: usize) -> (f64, f64) {
    let nyquist = sample_rate as f64 / 2.0;
    let n_bins = spectrum_db.len();
    if n_bins < 8 || sample_rate == 0 {
        return (nyquist, 1.0);
    }
    let bin_hz = sample_rate as f64 / fft_size as f64;
    let smoothed = moving_average(spectrum_db, 5);

    // Highest frequency bin still carrying content.
    let mut cutoff_bin = 0usize;
    for bin in (0..n_bins).rev() {
        if smoothed[bin] > CONTENT_FLOOR_DB {
            cutoff_bin = bin;
            break;
        }
    }

    let cutoff_hz = (cutoff_bin as f64 * bin_hz).min(nyquist);
    let cutoff_ratio = if nyquist > 0.0 { cutoff_hz / nyquist } else { 0.0 };
    (cutoff_hz, cutoff_ratio)
}

/// Measure the sharpness of the cutoff.
///
/// Returns `(cliff_db, above_db)`:
/// * `above_db` — mean level in the ~3 kHz band just above the cutoff (the
///   suspected dead zone). `NEG_INFINITY` if the cutoff is at Nyquist.
/// * `cliff_db` — how far the level drops from just below the cutoff to just
///   above it (positive == a drop). `0.0` when there is no band above.
pub fn cutoff_context(
    spectrum_db: &[f32],
    sample_rate: u32,
    fft_size: usize,
    cutoff_hz: f64,
) -> (f32, f32) {
    let nyquist = sample_rate as f64 / 2.0;
    if spectrum_db.len() < 8 || sample_rate == 0 {
        return (0.0, f32::NEG_INFINITY);
    }
    let smoothed = moving_average(spectrum_db, 5);
    let band = 3000.0;

    let above = band_mean_db(
        &smoothed,
        sample_rate,
        fft_size,
        cutoff_hz,
        (cutoff_hz + band).min(nyquist),
    );
    let below = band_mean_db(
        &smoothed,
        sample_rate,
        fft_size,
        (cutoff_hz - band).max(0.0),
        cutoff_hz,
    );

    let cliff = if above.is_finite() && below.is_finite() {
        below - above
    } else {
        0.0
    };
    (cliff, above)
}

/// Mean of the finite dB values in the `[f_lo, f_hi]` band. `NEG_INFINITY` if
/// the band spans no bins.
fn band_mean_db(spectrum_db: &[f32], sample_rate: u32, fft_size: usize, f_lo: f64, f_hi: f64) -> f32 {
    let n = spectrum_db.len();
    let to_bin = |f: f64| ((f * fft_size as f64 / sample_rate as f64).round() as usize).min(n - 1);
    let lo = to_bin(f_lo);
    let hi = to_bin(f_hi);
    if hi <= lo {
        return f32::NEG_INFINITY;
    }
    let mut sum = 0.0f32;
    let mut cnt = 0u32;
    for &v in &spectrum_db[lo..=hi] {
        if v.is_finite() {
            sum += v;
            cnt += 1;
        }
    }
    if cnt > 0 {
        sum / cnt as f32
    } else {
        f32::NEG_INFINITY
    }
}

fn moving_average(data: &[f32], radius: usize) -> Vec<f32> {
    if radius == 0 {
        return data.to_vec();
    }
    let n = data.len();
    let mut out = vec![0.0f32; n];
    for i in 0..n {
        let lo = i.saturating_sub(radius);
        let hi = (i + radius + 1).min(n);
        let mut sum = 0.0f32;
        let mut cnt = 0u32;
        for &v in &data[lo..hi] {
            if v.is_finite() {
                sum += v;
                cnt += 1;
            }
        }
        out[i] = if cnt > 0 { sum / cnt as f32 } else { data[i] };
    }
    out
}

#[cfg(test)]
mod tests {
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
}
