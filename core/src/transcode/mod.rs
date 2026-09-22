//! Transcoding detection: was this "lossless" audio once through a lossy
//! codec?
//!
//! # What this looks for
//!
//! A lossy encoder scales its transform coefficients, rounds them to integers,
//! and codes the result. Decoding undoes the scaling but *cannot* undo the
//! rounding — so the decoded signal's coefficients still sit exactly on the
//! lattice the encoder put them on. Re-wrapping that signal as FLAC preserves
//! it bit for bit, lattice included.
//!
//! So the test is: reproduce the codec's own analysis chain, scale the
//! coefficients the way the encoder would have, round them, and measure how
//! much the rounding actually moved them. On genuine lossless audio the
//! coefficients fall wherever they like and the error lands near its expected
//! value for a uniform quantizer. On transcoded audio, at the right alignment
//! and the right scalefactor, the error collapses towards zero — the values
//! are *already* on the grid.
//!
//! This is statistical evidence, not proof of provenance. The uniform-error
//! model can fail on tonal or almost-silent bands. Admissibility guards avoid
//! known false positives, but their sensitivity needs broader validation.
//!
//! # Why one detector per codec
//!
//! The principle above is codec-independent, but the lattice only exists
//! *inside the codec's own transform*, and the two transforms have nothing in
//! common:
//!
//! * **AAC** ([`aac`]) takes an MDCT straight off the PCM — 2048-sample long
//!   windows (1024 coefficients), 256-sample short ones.
//! * **MP3** takes a hybrid filterbank: a 32-band polyphase filter, *then* an
//!   18-point MDCT per band, *then* aliasing-reduction butterflies.
//!
//! An MP3 lattice viewed through AAC's transform is not a lattice, it is
//! noise. No amount of threshold tuning bridges that, which is why there are
//! two detectors and not one with a parameter.
//!
//! # Provenance
//!
//! The method is Olivier Derrien's, published as:
//!
//! * O. Derrien, "Detection of Genuine Lossless Audio Files: Application to
//!   the MPEG-AAC Codec", *J. Audio Eng. Soc.*, vol. 67, no. 3, pp. 116–123
//!   (2019 March). [DOI 10.17743/jaes.2019.0002](https://doi.org/10.17743/jaes.2019.0002)
//!
//! **This is an independent implementation written from the equations in that
//! paper**, not a translation of the author's MATLAB reference. That
//! distinction is deliberate and it matters: the reference distribution is
//! marked "personal use or research purpose, commercial use prohibited", which
//! is incompatible with this project's MIT licence. An algorithm published in
//! a journal may be implemented freely; a particular implementation of it may
//! not be copied. Every constant below is traceable either to a numbered
//! equation in the paper or to the MPEG standard it comes from, and says so.
//!
//! Where the paper is ambiguous, the choice made here is stated at the point
//! it is made rather than silently resolved.

pub mod aac;
pub mod aac_tables;
pub mod criterion;
pub mod frames;
pub mod mdct;
pub mod mp3;
pub mod mp3_tables;
pub mod pqmf;

pub use aac::AacParams;
pub use mp3::Mp3Params;

/// One scalefactor band: a half-open range of coefficient indices.
///
/// Shared by both codecs. The *values* differ per standard and per sample
/// rate, but the shape of the idea does not, and the criterion that consumes
/// them treats a band as a band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Band {
    /// First coefficient index, inclusive.
    pub start: usize,
    /// One past the last coefficient index.
    pub end: usize,
}

impl Band {
    /// Number of coefficients in the band.
    pub fn width(&self) -> usize {
        self.end - self.start
    }
}

/// What a codec detector concluded about one file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TranscodeEvidence {
    /// The detection criterion `L` from the paper's §1.4: the fraction of
    /// (subband, scalefactor) trials whose rounding error fell below its
    /// threshold. The best shape's counts are pooled across frames, then
    /// maximized over alignments and selected channel signals. In `0..=1`,
    /// but NOT a calibrated probability of having a lossy source.
    pub likelihood: f64,
    /// The sample alignment that produced [`Self::likelihood`]. Meaningful
    /// as a diagnostic only: even genuine lossless audio has a winning
    /// offset, so this value alone does not establish synchronization.
    pub offset: usize,
    /// Whether [`Self::likelihood`] cleared the codec's significance
    /// threshold — the actual verdict.
    pub detected: bool,
}

/// Why the lattice search produced no verdict.
///
/// The detectors return `Option`, and for a long time so did the pipeline —
/// which meant every reason a file went unmeasured collapsed into the same
/// silent `None`, indistinguishable from each other and, in the table, from a
/// clean result. A dash in the Lattice column then said "something went
/// wrong somewhere" and nothing more, which is not enough to act on.
///
/// So the reason is carried instead of discarded. It costs one enum and it
/// turns "the detector didn't run" from a dead end into a diagnosis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LatticeSkip {
    /// The file could not be decoded a second time. Carries the decoder's own
    /// message, since that is the only thing that says *why*.
    Undecodable(String),
    /// Decoding succeeded but produced no usable samples.
    NoSamples,
    /// The sample rate has no tabulated scalefactor bands — see
    /// [`supported_rate_khz`].
    UnsupportedRate(u32),
    /// Fewer frames than one sweep needs.
    TooShort,
    /// The user cancelled the analysis.
    Cancelled,
}

impl std::fmt::Display for LatticeSkip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Undecodable(e) => write!(f, "the file could not be decoded a second time ({e})"),
            Self::NoSamples => write!(f, "the decoder returned no samples"),
            Self::UnsupportedRate(hz) => write!(
                f,
                "{:.1} kHz has no tabulated scalefactor bands (only 32, 44.1 and 48 kHz do)",
                *hz as f64 / 1000.0
            ),
            Self::TooShort => write!(f, "the file is too short for all requested analysis windows; transcoding is inconclusive"),
            Self::Cancelled => write!(f, "the analysis was cancelled"),
        }
    }
}

/// What the lattice search concluded, or why it could not conclude anything.
pub type LatticeResult = Result<TranscodeEvidence, LatticeSkip>;

/// Sample rates the detectors handle, in kHz.
///
/// The scalefactor band tables are defined per sample rate by the MPEG
/// standards; only these three are tabulated here. Other rates are outside
/// this detector's scope, not evidence against a lossy source. Resampling
/// can follow lossy decoding, so upsampling and transcoding may coexist.
pub fn supported_rate_khz(sample_rate: u32) -> Option<u32> {
    match sample_rate {
        32_000 => Some(32),
        44_100 => Some(44),
        48_000 => Some(48),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_three_tabulated_rates_are_supported() {
        assert_eq!(supported_rate_khz(44_100), Some(44));
        assert_eq!(supported_rate_khz(48_000), Some(48));
        assert_eq!(supported_rate_khz(32_000), Some(32));
        // Hi-res: a band limit here is upsampling, not a lossy source.
        assert_eq!(supported_rate_khz(96_000), None);
        assert_eq!(supported_rate_khz(88_200), None);
        // Odd rates from games or old hardware have no tabulated bands.
        assert_eq!(supported_rate_khz(22_050), None);
        assert_eq!(supported_rate_khz(0), None);
    }
}
