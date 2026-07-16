//! Effective bit-depth estimation.
//!
//! Many "24-bit" files are really 16-bit (or less) content padded with zero low
//! bits — either because the master was 16-bit or because of a lazy conversion.
//! By OR-ing every integer sample value together, the count of trailing zero
//! bits (within the declared bit width) reveals how many low bits never carry
//! information.

/// Given the bitwise OR of every integer sample value and the container's
/// declared bit width, return the effective (used) bit depth.
///
/// The OR mask is restricted to the low `declared_bits` bits so that
/// sign-extension of negative two's-complement samples does not inflate the
/// result. `effective = declared_bits - trailing_zero_bits`.
///
/// * All-zero input (pure digital silence) is reported as 1 bit.
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

/// Is a `declared`-bit file effectively 16-bit or less?
pub fn is_fake_hires(declared: u32, or_mask: u32) -> bool {
    declared >= 24 && effective_bits(or_mask, declared) <= 16
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn silence_reports_one_bit() {
        assert_eq!(effective_bits(0, 24), 1);
    }
}
