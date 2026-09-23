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
#[path = "../../tests/unit/analysis/stereo.rs"]
mod tests;
