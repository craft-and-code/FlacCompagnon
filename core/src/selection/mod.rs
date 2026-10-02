//! Requested measurements and their shared decode dependencies.

mod results;

/// CLI measurement names, in stable display order.
pub const ANALYSES: &[&str] = &[
    "authenticity",
    "bit-depth",
    "spectrum",
    "stereo",
    "phase",
    "hf-stereo",
    "clipping",
    "loudness",
    "dynamics",
    "clicks",
    "dropouts",
    "dc-offset",
    "flac-md5",
    "fingerprints",
];

/// One independently selectable measurement group.
#[derive(Debug, Clone, Copy)]
pub enum AnalysisKind {
    /// Upscaling, upsampling and lossy-source detection.
    Authenticity,
    /// Effective integer depth.
    BitDepth,
    /// Spectral cutoff.
    Spectrum,
    /// Channel relationship and balance.
    Stereo,
    /// Global and local phase.
    Phase,
    /// High-frequency stereo width.
    HfStereo,
    /// Sample and inter-sample clipping.
    Clipping,
    /// Integrated loudness, peaks and range.
    Loudness,
    /// Dynamic range.
    Dynamics,
    /// Suspected clicks.
    Clicks,
    /// Suspected dropouts.
    Dropouts,
    /// Channel DC offset.
    DcOffset,
    /// FLAC decoded-audio integrity.
    FlacMd5,
    /// Whole-file MD5 and CRC32.
    Fingerprints,
}

/// Explicit requested measurements; dependencies are derived separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisSelection(u16);

impl AnalysisSelection {
    /// Select every measurement (the desktop and legacy API default).
    pub const fn all() -> Self {
        Self((1 << ANALYSES.len()) - 1)
    }

    /// Build an explicit selection. An empty iterator means no measurements.
    pub fn from_names(names: impl IntoIterator<Item = impl AsRef<str>>) -> Result<Self, String> {
        let mut bits = 0;
        for name in names {
            let name = name.as_ref();
            let index = ANALYSES
                .iter()
                .position(|&known| known == name)
                .ok_or_else(|| format!("unknown analysis '{name}'"))?;
            bits |= 1 << index;
        }
        Ok(Self(bits))
    }

    /// Whether the caller explicitly requested this measurement.
    pub const fn enabled(self, kind: AnalysisKind) -> bool {
        self.0 & (1 << kind as u16) != 0
    }

    /// Stable names for marking partial reports and checking cache coverage.
    pub fn names(self) -> Vec<String> {
        ANALYSES
            .iter()
            .enumerate()
            .filter(|(i, _)| self.0 & (1 << i) != 0)
            .map(|(_, name)| (*name).to_string())
            .collect()
    }

    /// Whether this selection includes every measurement in another selection.
    pub const fn covers(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether every supported measurement was requested.
    pub const fn is_full(self) -> bool {
        self.0 == Self::all().0
    }

    pub(crate) const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub(crate) fn spectrum(self) -> bool {
        self.enabled(AnalysisKind::Spectrum) || self.enabled(AnalysisKind::Authenticity)
    }
    pub(crate) fn bit_depth(self) -> bool {
        self.enabled(AnalysisKind::BitDepth) || self.enabled(AnalysisKind::Authenticity)
    }
    pub(crate) fn stereo(self) -> bool {
        self.enabled(AnalysisKind::Stereo) || self.enabled(AnalysisKind::Phase)
    }
    pub(crate) fn sample_peak(self) -> bool {
        self.enabled(AnalysisKind::Clipping)
            || self.enabled(AnalysisKind::Dynamics)
            || self.enabled(AnalysisKind::Authenticity)
    }
    pub(crate) fn audio(self) -> bool {
        self.0 & ((1 << AnalysisKind::FlacMd5 as u16) - 1) != 0
    }

    pub(crate) fn dsd_pcm(self) -> Self {
        // DSD authenticity uses the spectrum only. Integer padding, codec
        // grids and PCM discontinuity heuristics do not apply after filtering.
        let unsupported = (1 << AnalysisKind::Authenticity as u16)
            | (1 << AnalysisKind::BitDepth as u16)
            | (1 << AnalysisKind::Clicks as u16)
            | (1 << AnalysisKind::Dropouts as u16);
        let spectrum = if self.enabled(AnalysisKind::Authenticity) {
            1 << AnalysisKind::Spectrum as u16
        } else {
            0
        };
        Self((self.0 & !unsupported) | spectrum)
    }
}

impl Default for AnalysisSelection {
    fn default() -> Self {
        Self::all()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/selection.rs"]
mod tests;
