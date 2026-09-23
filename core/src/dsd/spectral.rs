//! Calibrated spectral heuristics around DSD, in both directions.
//!
//! * [`pcm_source_check`] — a *DSD file* whose content came from PCM. Real DSD
//!   blends smoothly into the sigma-delta noise shaping that rises above
//!   ~25 kHz; PCM-sourced DSD instead shows a digital brick wall at the
//!   source's Nyquist (22.05 or 24 kHz). Measured on synthetic ground truth (a
//!   2nd-order delta-sigma modulator fed native-band vs 44.1k-band-limited
//!   content): the drop across the boundary is ≈ 3 dB for native DSD and
//!   ≈ 50 dB for PCM-sourced.
//! * [`dsd_heritage_check`] — the mirror case: *hi-res PCM* (FLAC/WAV ≥ 96 kHz)
//!   converted from a DSD master, which carries the sigma-delta noise ramp
//!   into the PCM file.
//!
//! Both work from the averaged spectrum the analyzer already produced, so
//! neither reads a file or knows anything about container layout.

/// Result of the PCM-source check on decoded DSD content.
#[derive(Debug, Clone, Copy)]
pub struct PcmSourceCheck {
    /// Boundary where the brick wall sits (22 050 or 24 000 Hz).
    pub boundary_hz: f64,
    /// Level drop across the boundary in dB.
    pub drop_db: f32,
}

/// Minimum drop across a PCM-Nyquist boundary to call it a brick wall.
/// Calibrated: native ≈ 3 dB, PCM-sourced ≈ 50 dB.
pub const PCM_CLIFF_DB: f32 = 30.0;

/// Minimum ultrasonic rise for [`dsd_heritage_check`]. Measured on a real
/// DSD-sourced 192 kHz FLAC: valley ≈ −85 dB, 36–70 kHz ≈ −55 dB → +30 dB.
const DSD_RAMP_DB: f32 = 15.0;

/// Mean level over `[lo, hi]` Hz of an averaged dB spectrum, or `None` when
/// the band falls outside the spectrum, spans `min_span` bins or fewer, or
/// holds no finite value.
///
/// `min_span` is why this takes a parameter instead of being one shared
/// constant: the brick-wall check reads narrow bands right next to each other
/// and only needs them non-empty, while the ramp check averages wide bands
/// and would be noise-driven if it accepted a two-bin one.
fn mean_band(spectrum_db: &[f32], bin_hz: f64, lo: f64, hi: f64, min_span: usize) -> Option<f32> {
    if bin_hz <= 0.0 || spectrum_db.is_empty() {
        return None;
    }
    let a = (lo / bin_hz) as usize;
    let b = ((hi / bin_hz) as usize).min(spectrum_db.len() - 1);
    if b <= a + min_span {
        return None;
    }
    let (sum, n) = spectrum_db[a..=b]
        .iter()
        .filter(|v| v.is_finite())
        .fold((0.0f32, 0usize), |(s, n), v| (s + v, n + 1));
    (n > 0).then(|| sum / n as f32)
}

/// Inspect the averaged spectrum (from the analyzer, computed on the decoded
/// PCM at `decoded_rate`) for a digital brick wall at a PCM source's Nyquist.
pub fn pcm_source_check(
    spectrum_db: &[f32],
    decoded_rate: u32,
    fft_size: usize,
) -> Option<PcmSourceCheck> {
    if spectrum_db.len() < 16 || decoded_rate == 0 || fft_size == 0 {
        return None;
    }
    let bin_hz = decoded_rate as f64 / fft_size as f64;

    let mut best: Option<PcmSourceCheck> = None;
    for boundary in [22_050.0f64, 24_000.0] {
        let below = mean_band(spectrum_db, bin_hz, boundary - 2_000.0, boundary - 200.0, 0);
        let above = mean_band(spectrum_db, bin_hz, boundary + 300.0, boundary + 2_000.0, 0);
        let (Some(lo), Some(hi)) = (below, above) else {
            continue;
        };
        // Require actual content below the boundary (not silence).
        if lo <= -80.0 {
            continue;
        }
        let drop = lo - hi;
        if drop >= PCM_CLIFF_DB && best.is_none_or(|b| drop > b.drop_db) {
            best = Some(PcmSourceCheck {
                boundary_hz: boundary,
                drop_db: drop,
            });
        }
    }
    best
}

/// Detect the sigma-delta heritage of a DSD master inside hi-res *PCM*
/// (FLAC/WAV at ≥ 96 kHz): genuine PCM recordings decay monotonically into the
/// ultrasonic range, while DSD-converted PCM shows a valley around 22–30 kHz
/// followed by a strong **rising** noise ramp. Returns the rise in dB.
pub fn dsd_heritage_check(spectrum_db: &[f32], sample_rate: u32, fft_size: usize) -> Option<f32> {
    if sample_rate < 96_000 || spectrum_db.len() < 16 || fft_size == 0 {
        return None;
    }
    let nyq = sample_rate as f64 / 2.0;
    let bin_hz = sample_rate as f64 / fft_size as f64;

    let valley = mean_band(spectrum_db, bin_hz, 22_000.0, 30_000.0, 3)?;
    let ramp = mean_band(
        spectrum_db,
        bin_hz,
        36_000.0,
        (0.92 * nyq).min(75_000.0),
        3,
    )?;
    let rise = ramp - valley;
    (rise >= DSD_RAMP_DB && ramp > -75.0).then_some(rise)
}

#[cfg(test)]
#[path = "../../tests/unit/dsd/spectral.rs"]
mod tests;
