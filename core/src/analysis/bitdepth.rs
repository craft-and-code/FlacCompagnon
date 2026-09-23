//! Effective bit-depth estimation.
//!
//! Many "24-bit" files are really 16-bit (or less) content padded with zero low
//! bits — either because the master was 16-bit or because of a lazy conversion.
//! By OR-ing every integer sample value together, the count of trailing zero
//! bits (within the declared bit width) reveals how many low bits never carry
//! information.

use serde::{Deserialize, Serialize};

/// How the effective integer depth was obtained. Neither method reconstructs
/// a recording's history; `NarrowGrid` is an estimate rather than exact padding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BitDepthMethod {
    /// Count low bits that are zero in every decoded integer sample.
    Stored,
    /// A lower-depth grid survives beneath a small residual on every channel.
    NarrowGrid,
}

/// Evidence accompanying the effective depth displayed in `Real bits`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitDepthEvidence {
    /// Exact occupied depth, including any noise added during export.
    pub stored_bits: u32,
    /// Distinguishes exact unused bits from an estimated quantization grid.
    pub method: BitDepthMethod,
}

// Conservative engineering guards, exercised by the ground-truth and adversarial
// tests below; these are not confidence probabilities. Three disjoint windows
// must each span 64 grid steps and at least 32 different codes modulo 64.
const WINDOW_SAMPLES: u32 = 4096;
const MIN_RICH_WINDOWS: u32 = 3;
const MIN_CODE_SPAN: i64 = 64;
const MIN_CODE_BINS: u32 = 32;
// At most 1/16 of a grid step on either side. For 16 -> 24, this is +/-16
// destination LSB: the supplied Audacity export reaches +/-10 across 16,558,080
// samples. A smaller +/-8 limit misses 142 samples. See docs/upscaling.md.
// A gap of at least six bits keeps the accepted region narrow enough to be
// useful; dither obscuring more closely spaced grids is intentionally not inferred.
const MIN_GRID_SHIFT: u32 = 6;
const MAX_GRID_SHIFT: u32 = 24; // source >= 8 bits in a <= 32-bit container

struct Grid {
    shift: u32,
    count: u32,
    min: i64,
    max: i64,
    bins: u64,
    rich_windows: u32,
}

impl Grid {
    fn new(shift: u32) -> Self {
        Self {
            shift,
            count: 0,
            min: i64::MAX,
            max: i64::MIN,
            bins: 0,
            rich_windows: 0,
        }
    }

    fn push(&mut self, sample: i32) -> bool {
        // i64 also makes the rounding safe at both endpoints of signed 32-bit PCM.
        let step = 1i64 << self.shift;
        let code = (i64::from(sample) + step / 2) >> self.shift;
        if (i64::from(sample) - code * step).unsigned_abs() > (step / 16) as u64 {
            return false;
        }
        self.count += 1;
        self.min = self.min.min(code);
        self.max = self.max.max(code);
        self.bins |= 1u64 << (code & 63);
        if self.count == WINDOW_SAMPLES {
            if self.max - self.min >= MIN_CODE_SPAN && self.bins.count_ones() >= MIN_CODE_BINS {
                self.rich_windows = self.rich_windows.saturating_add(1);
            }
            self.count = 0;
            self.min = i64::MAX;
            self.max = i64::MIN;
            self.bins = 0;
        }
        true
    }
}

struct Channel {
    or_mask: u32,
    grids: Vec<Grid>,
}

impl Channel {
    fn new() -> Self {
        Self {
            or_mask: 0,
            grids: (MIN_GRID_SHIFT..=MAX_GRID_SHIFT).map(Grid::new).collect(),
        }
    }

    fn push(&mut self, sample: i32) {
        self.or_mask |= sample as u32;
        // Leading zero samples satisfy every grid without contributing any
        // evidence. Avoid running all candidates over an entirely silent file.
        if self.or_mask == 0 {
            return;
        }
        // One outlier vetoes the candidate for the rest of the file. A padded
        // intro, silent channel, or long silence must not hide native precision.
        self.grids.retain_mut(|grid| grid.push(sample));
    }

    fn depth(&self, declared: u32) -> u32 {
        let exact = effective_bits(self.or_mask, declared);
        self.grids
            .iter()
            .filter_map(|grid| {
                let bits = declared.checked_sub(grid.shift)?;
                (bits >= 8 && bits < exact && grid.rich_windows >= MIN_RICH_WINDOWS).then_some(bits)
            })
            .min()
            .unwrap_or(exact)
    }
}

/// Streaming, channel-preserving integer precision analysis.
pub(super) struct BitDepthAnalyzer {
    channels: Vec<Channel>,
    saw_integer: bool,
    complete: bool,
}

impl BitDepthAnalyzer {
    pub(super) fn new(channels: usize) -> Self {
        Self {
            channels: (0..channels).map(|_| Channel::new()).collect(),
            saw_integer: false,
            complete: true,
        }
    }

    pub(super) fn push_frame(&mut self, samples: Option<&[i32]>) {
        let Some(samples) = samples.filter(|s| s.len() == self.channels.len()) else {
            self.complete = false;
            return;
        };
        self.saw_integer = true;
        for (channel, &sample) in self.channels.iter_mut().zip(samples) {
            channel.push(sample);
        }
    }

    pub(super) fn finish(&self, declared: Option<u32>) -> Option<(u32, BitDepthEvidence)> {
        let declared = declared.filter(|b| (1..=32).contains(b))?;
        if !self.saw_integer || !self.complete {
            return None;
        }
        let mask = self.channels.iter().fold(0, |mask, ch| mask | ch.or_mask);
        let stored_bits = effective_bits(mask, declared);
        // Taking the maximum prevents one low-depth channel from reducing the
        // verdict for a different channel. An all-zero channel contributes 1,
        // preserving the historical digital-silence convention.
        let bits = self.channels.iter().map(|ch| ch.depth(declared)).max()?;
        let method = if bits < stored_bits {
            BitDepthMethod::NarrowGrid
        } else {
            BitDepthMethod::Stored
        };
        Some((
            bits,
            BitDepthEvidence {
                stored_bits,
                method,
            },
        ))
    }
}

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
    fn silence_reports_one_bit() {
        assert_eq!(effective_bits(0, 24), 1);
    }

    fn measure(
        bits: u32,
        frames: impl IntoIterator<Item = Vec<i32>>,
        channels: usize,
    ) -> (u32, BitDepthEvidence) {
        let mut analyzer = BitDepthAnalyzer::new(channels);
        for frame in frames {
            analyzer.push_frame(Some(&frame));
        }
        analyzer
            .finish(Some(bits))
            .expect("complete integer fixture")
    }

    fn source(n: i32) -> i32 {
        (n * 73 % 60_001) - 30_000
    }

    #[test]
    fn exact_padding_keeps_arbitrary_widths_and_the_silence_convention() {
        for declared in [8, 16, 24, 32] {
            for original in 1..=declared {
                let sample = -1i32 << (declared - original);
                let (bits, e) = measure(declared, [vec![sample, 0]], 2);
                assert_eq!(bits, original);
                assert_eq!(e.method, BitDepthMethod::Stored);
            }
            assert_eq!(measure(declared, [vec![0, 0]], 2).0, 1);
        }
    }

    #[test]
    fn low_level_residual_does_not_hide_a_sustained_lower_depth_grid() {
        // Construct an integer source independently, then add a bounded residue
        // after widening. This includes signed samples on both sides of zero.
        for (original, declared) in [(8, 16), (12, 24), (16, 24), (18, 24), (20, 32), (24, 32)] {
            let shift = declared - original;
            let frames = (0..16_384).map(|n| {
                let range = (1i32 << (original - 1)).min(30_000);
                let s = (n * 73 % (2 * range - 1)) - range;
                let residual = n % 5 - 2;
                vec![(s << shift) + residual]
            });
            let (bits, evidence) = measure(declared, frames, 1);
            assert_eq!(bits, original, "{original} -> {declared}");
            assert_eq!(evidence.stored_bits, declared);
            assert_eq!(evidence.method, BitDepthMethod::NarrowGrid);
        }
    }

    #[test]
    fn a_single_outlier_after_a_long_padded_intro_vetoes_the_grid() {
        let frames = (0..20_000)
            .map(|n| vec![(source(n) << 8) + if n == 19_999 { 127 } else { n % 21 - 10 }]);
        let (bits, e) = measure(24, frames, 1);
        assert_eq!((bits, e.method), (24, BitDepthMethod::Stored));
    }

    #[test]
    fn every_channel_must_support_the_lower_depth() {
        for reverse in [false, true] {
            let frames = (0..16_384).map(|n| {
                let mut f = vec![(source(n) << 8) + n % 21 - 10, source(n) * 253 + 1];
                if reverse {
                    f.reverse();
                }
                f
            });
            assert_eq!(measure(24, frames, 2).0, 24);
        }
        let frames = (0..16_384).map(|n| vec![0, (source(n) << 8) + n % 21 - 10]);
        assert_eq!(measure(24, frames, 2).0, 16);
    }

    #[test]
    fn different_channel_grids_use_the_highest_required_depth() {
        let frames = (0..16_384).map(|n| vec![(source(n) << 8) + n % 21 - 10, source(n) << 4]);
        let (bits, evidence) = measure(24, frames, 2);
        assert_eq!(bits, 20);
        assert_eq!(evidence.method, BitDepthMethod::NarrowGrid);
    }

    #[test]
    fn anti_phase_channels_cannot_cancel_the_bit_depth_evidence() {
        let frames = (0..16_384).map(|n| {
            let s = (source(n) << 8) + n % 21 - 10;
            vec![s, -s]
        });
        assert_eq!(measure(24, frames, 2).0, 16);
    }

    #[test]
    fn silent_noisy_dc_and_few_level_signals_do_not_establish_a_grid() {
        for kind in 0..4 {
            let frames = (0..16_384).map(|n| {
                let coarse = match kind {
                    0 => 0,
                    1 => 10_000,
                    2 => {
                        if n % 2 == 0 {
                            20_000
                        } else {
                            -20_000
                        }
                    }
                    _ => {
                        if n % 4096 == 0 {
                            source(n)
                        } else {
                            0
                        }
                    }
                };
                vec![(coarse << 8) + n % 21 - 10]
            });
            assert_eq!(measure(24, frames, 1).0, 24, "kind {kind}");
        }
    }

    #[test]
    fn too_little_varying_audio_cannot_be_an_estimated_grid() {
        let frames = (0..20_000).map(|n| {
            vec![if n < 8192 {
                (source(n) << 8) + n % 21 - 10
            } else {
                0
            }]
        });
        assert_eq!(measure(24, frames, 1).0, 24);
    }

    #[test]
    fn native_integer_noise_and_low_level_tones_are_not_lower_depth() {
        for amplitude in [10, 1_000, 30_000, 8_000_000] {
            let mut rng = 0xa17b_924du32;
            let noise = (0..16_384).map(|_| {
                rng ^= rng << 13;
                rng ^= rng >> 17;
                rng ^= rng << 5;
                vec![(rng % (2 * amplitude + 1)) as i32 - amplitude as i32]
            });
            assert_eq!(measure(24, noise, 1).0, 24);
            let tone = (0..16_384)
                .map(|n| vec![(amplitude as f64 * (n as f64 * 0.14247586).sin()).round() as i32]);
            assert_eq!(measure(24, tone, 1).0, 24);
        }
    }

    #[test]
    fn empty_float_or_partly_missing_integer_frames_do_not_invent_a_depth() {
        let mut a = BitDepthAnalyzer::new(2);
        assert!(a.finish(Some(24)).is_none());
        a.push_frame(Some(&[256, -256]));
        a.push_frame(None);
        assert!(a.finish(Some(24)).is_none());
        let mut a = BitDepthAnalyzer::new(2);
        a.push_frame(Some(&[256]));
        assert!(a.finish(Some(24)).is_none());
        assert!(a.finish(Some(0)).is_none());
        assert!(a.finish(Some(33)).is_none());
    }
}
