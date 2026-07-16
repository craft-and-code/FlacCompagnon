//! Fake-stereo (dual-mono) detection.
//!
//! A file can claim to be stereo while both channels carry an identical signal.
//! Two independent conditions flag it:
//! 1. Every frame had L == R (bit-exact dual mono), or
//! 2. The L-R difference energy is >= 60 dB below the total channel energy.

/// Threshold (in dB) below which the L-R difference is considered negligible.
const DIFF_FLOOR_DB: f64 = -60.0;

/// Decide whether a >= 2 channel signal is really dual-mono, from accumulated
/// energies and the count of bit-identical frames.
pub fn is_fake(
    diff_energy: f64,
    l_energy: f64,
    r_energy: f64,
    identical_frames: u64,
    total_frames: u64,
) -> bool {
    if total_frames == 0 {
        return false;
    }
    if identical_frames == total_frames {
        return true;
    }
    let sig_energy = l_energy + r_energy;
    if sig_energy <= f64::EPSILON {
        return false; // both channels silent
    }
    let ratio_db = 10.0 * (diff_energy / sig_energy).max(1e-30).log10();
    ratio_db < DIFF_FLOOR_DB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_channels_are_fake() {
        assert!(is_fake(0.0, 100.0, 100.0, 1000, 1000));
    }

    #[test]
    fn decorrelated_channels_are_real() {
        // Large difference energy relative to signal.
        assert!(!is_fake(150.0, 100.0, 100.0, 0, 1000));
    }

    #[test]
    fn tiny_difference_is_fake() {
        // Difference 70 dB down -> effectively dual mono.
        let sig = 200.0;
        let diff = sig * 10f64.powf(-70.0 / 10.0);
        assert!(is_fake(diff, 100.0, 100.0, 0, 1000));
    }
}
