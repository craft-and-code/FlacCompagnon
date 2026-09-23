//! Stereo relationship: dual-mono and opposing channel polarity.
//!
//! A file can claim to be stereo while both channels carry an identical signal.
//! Two independent conditions flag it:
//! 1. Every frame had L == R (bit-exact dual mono), or
//! 2. The L-R difference energy is >= 60 dB below the total channel energy.

/// Threshold (in dB) below which the L-R difference is considered negligible.
const DIFF_FLOOR_DB: f64 = -60.0;

/// Project threshold for a likely channel-polarity inversion. A strongly
/// negative whole-stream correlation detects R = -kL even when unequal channel
/// gains prevent near-total cancellation in mono. Less negative values remain
/// a mono-compatibility warning, not a polarity finding.
const INVERTED_CORRELATION: f64 = -0.95;

/// Correlation and a conservative polarity-inversion finding for two channels.
#[derive(Debug, Clone, Copy)]
pub struct PhaseAnalysis {
    /// Whole-stream L/R correlation in -1..1; absent when either channel is silent.
    pub correlation: Option<f32>,
    /// True when the channels are strongly opposed across the whole stream.
    pub likely_inverted: bool,
}

/// Measure phase correlation from accumulated channel energies. Absolute
/// polarity cannot be known without a reference recording;
/// this only compares the two channels to each other.
pub fn analyze_phase(l_energy: f64, r_energy: f64, cross_energy: f64) -> PhaseAnalysis {
    if !l_energy.is_finite()
        || !r_energy.is_finite()
        || !cross_energy.is_finite()
        || l_energy <= f64::EPSILON
        || r_energy <= f64::EPSILON
    {
        return PhaseAnalysis {
            correlation: None,
            likely_inverted: false,
        };
    }
    let correlation = (cross_energy / (l_energy * r_energy).sqrt()).clamp(-1.0, 1.0);
    PhaseAnalysis {
        correlation: Some(correlation as f32),
        likely_inverted: correlation <= INVERTED_CORRELATION,
    }
}

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
