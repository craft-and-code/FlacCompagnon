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
// unit tests; these are not confidence probabilities. Three disjoint windows
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
#[path = "../../tests/unit/analysis/bitdepth.rs"]
mod tests;
