//! Effective bit-depth estimation.
//!
//! Many "24-bit" files are really 16-bit content padded with zero low bits or
//! exported with a tiny amount of dither. Exact padding is recovered from the
//! bitwise OR of every sample. Dithered padding is recognized separately from
//! the sample distribution around the 16-bit quantization grid.

/// Maximum distance from the nearest 16-bit grid point in a 24-bit sample.
/// Audacity's shaped dither on the regression file stays within ±10; ±16
/// leaves headroom without accepting the 128-step spread of real 24-bit data.
const DITHER_GRID_TOLERANCE: u32 = 16;
/// A short or nearly silent signal cannot establish a quantization lattice.
const DITHER_GRID_MIN_SAMPLES: u64 = 4_096;
const DITHER_GRID_MIN_PEAK: u32 = 4_096;

/// Streaming evidence for a 16-bit quantization grid inside 24-bit PCM.
#[derive(Debug, Default)]
pub struct Grid16Evidence {
    samples: u64,
    near_grid: u64,
    peak: u32,
}

impl Grid16Evidence {
    /// Add one integer sample in its declared native width.
    pub fn push(&mut self, sample: i32) {
        self.samples = self.samples.saturating_add(1);
        self.peak = self.peak.max(sample.unsigned_abs());
        let residue = sample.rem_euclid(256) as u32;
        let distance = residue.min(256 - residue);
        if distance <= DITHER_GRID_TOLERANCE {
            self.near_grid = self.near_grid.saturating_add(1);
        }
    }

    /// Whether a 24-bit stream follows a 16-bit grid hidden by low-level dither.
    pub fn detected(&self, declared_bits: u32) -> bool {
        declared_bits == 24
            && self.samples >= DITHER_GRID_MIN_SAMPLES
            && self.peak >= DITHER_GRID_MIN_PEAK
            && self.near_grid == self.samples
    }
}

/// Given the bitwise OR of every integer sample value and the container's
/// declared bit width, return the effective (used) bit depth.
///
/// The OR mask is restricted to the low `declared_bits` bits so that
/// sign-extension of negative two's-complement samples does not inflate the
/// result. `effective = declared_bits - trailing_zero_bits`.
///
/// * All-zero input returns the arithmetic minimum, 1. This is not evidence
///   of a one-bit recording: callers must treat silence as unmeasurable.
pub fn effective_bits(or_mask: u32, declared_bits: u32) -> u32 {
    let width = declared_bits.clamp(1, 32);
    let mask = if width >= 32 {
        u32::MAX
    } else {
        (1u32 << width) - 1
    };
    let low = or_mask & mask;
    if low == 0 {
        return 1;
    }
    let trailing = low.trailing_zeros();
    width.saturating_sub(trailing).max(1)
}

/// Does non-silent integer content fit in 16 bits within a wider container?
pub fn is_fake_hires(declared: u32, or_mask: u32) -> bool {
    (24..=32).contains(&declared) && or_mask != 0 && effective_bits(or_mask, declared) <= 16
}

#[cfg(test)]
mod tests {
    use super::*;

    // The underscore in each literal below sits at the padding boundary, not
    // every four digits: `0x1234_00` reads as "the 16-bit sample 0x1234,
    // shifted into a 24-bit field, low byte zero", which is the whole point of
    // this test. Clippy's suggested regrouping (`0x0012_3400`) is uniform but
    // says nothing, so the lint is turned off here rather than obeyed.
    #[allow(clippy::unusual_byte_groupings)]
    #[test]
    fn sixteen_bit_padded_to_24_is_flagged() {
        // 16-bit samples left-shifted into a 24-bit field => low 8 bits zero.
        let mut mask = 0u32;
        for v in [0x1234_00u32, 0x5600_00, 0x00AB_00, 0x7F00_00] {
            mask |= v;
        }
        assert_eq!(effective_bits(mask, 24), 16);
        assert!(is_fake_hires(24, mask));
    }

    #[test]
    fn genuine_24bit_is_not_flagged() {
        let mut mask = 0u32;
        for v in [0x123456u32, 0x000001, 0xABCDEF, 0x000003] {
            mask |= v;
        }
        assert!(effective_bits(mask, 24) > 16);
        assert!(!is_fake_hires(24, mask));
    }

    #[test]
    fn sign_extended_negatives_do_not_inflate() {
        // -256 (0xFFFFFF00) has 8 trailing zero bits; within a 24-bit width the
        // effective depth is 16, not 32.
        let mask = (-256i32) as u32;
        assert_eq!(effective_bits(mask, 24), 16);
    }

    #[test]
    fn silence_is_not_evidence_of_fake_hires() {
        assert!(!is_fake_hires(24, 0));
        assert!(!is_fake_hires(32, 0));
    }

    #[test]
    fn silence_reports_one_bit() {
        assert_eq!(effective_bits(0, 24), 1);
    }

    #[test]
    fn audacity_style_dither_still_reveals_the_16bit_grid() {
        let mut evidence = Grid16Evidence::default();
        for n in 0..DITHER_GRID_MIN_SAMPLES {
            let base = (((n as i32 % 20_001) - 10_000) << 8).clamp(-8_388_608, 8_388_607);
            let dither = (n as i32 % 21) - 10;
            evidence.push(base + dither);
        }
        assert!(evidence.detected(24));
    }

    #[test]
    fn genuine_24bit_residues_do_not_form_a_16bit_grid() {
        let mut evidence = Grid16Evidence::default();
        for n in 0..DITHER_GRID_MIN_SAMPLES {
            evidence.push((n as i32 * 65_537) & 0x7f_ffff);
        }
        assert!(!evidence.detected(24));
    }

    #[test]
    fn silence_with_low_level_noise_is_not_enough_evidence() {
        let mut evidence = Grid16Evidence::default();
        for n in 0..DITHER_GRID_MIN_SAMPLES {
            evidence.push((n as i32 % 21) - 10);
        }
        assert!(!evidence.detected(24));
    }
}
