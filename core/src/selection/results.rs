//! Keep unrequested measurements out of selected results and reused reports.
use super::{AnalysisKind as Kind, AnalysisSelection};
use crate::{ClippingInfo, Detections, FileAnalysis};

impl FileAnalysis {
    /// Whether a saved result covers every requested measurement. Missing
    /// coverage metadata denotes a complete report from the legacy API.
    pub fn covers_selection(&self, selection: AnalysisSelection) -> bool {
        self.analyses_run.as_ref().is_none_or(|names| {
            AnalysisSelection::from_names(names).is_ok_and(|saved| saved.covers(selection))
        })
    }

    /// Retain only requested measurements, including when reusing a richer
    /// cache. Dependencies used internally are not advertised as requested analyses.
    pub fn restrict_to(&mut self, selection: AnalysisSelection) {
        let selection = if let Some(names) = &self.analyses_run {
            selection
                .intersection(AnalysisSelection::from_names(names).unwrap_or(AnalysisSelection(0)))
        } else {
            selection
        };
        self.analyses_run = (!selection.is_full()).then(|| selection.names());
        if !selection.enabled(Kind::Authenticity) {
            self.detections = Detections {
                upscaling: false,
                upsampling: false,
                transcoding: false,
                summary: "Not analyzed".into(),
                detail: "Authenticity analysis was not requested.".into(),
            };
            self.lattice_score = None;
            self.badge = None;
        }
        if !selection.enabled(Kind::Spectrum) {
            self.cutoff_hz = None;
            self.cutoff_ratio = None;
        }
        if !selection.enabled(Kind::BitDepth) {
            self.real_bit_depth = None;
            self.bit_depth_evidence = None;
        }
        if !selection.enabled(Kind::Stereo) {
            self.fake_stereo = None;
            self.stereo_balance = None;
        }
        if !selection.enabled(Kind::Phase) {
            self.phase_correlation = None;
            self.phase_inverted = None;
            self.local_phase = None;
        }
        if !selection.enabled(Kind::HfStereo) {
            self.high_frequency_stereo = None;
        }
        if !selection.enabled(Kind::DcOffset) {
            self.dc_offset = None;
        }
        if !selection.enabled(Kind::Clipping) {
            self.clipping = ClippingInfo::unmeasured();
        }
        if !selection.enabled(Kind::Dynamics) {
            self.dr_db = None;
        }
        if !selection.enabled(Kind::Loudness) {
            self.integrated_lufs = None;
            self.loudness_peaks = None;
            self.loudness_range_lu = None;
        }
        if !selection.enabled(Kind::Clicks) && !selection.enabled(Kind::Dropouts) {
            self.discontinuities = None;
        } else if let Some(discontinuities) = &mut self.discontinuities {
            if !selection.enabled(Kind::Clicks) {
                discontinuities.clicks = Default::default();
            }
            if !selection.enabled(Kind::Dropouts) {
                discontinuities.dropouts = Default::default();
            }
        }
        if !selection.enabled(Kind::FlacMd5) {
            self.flac_md5 = None;
        }
        if !selection.enabled(Kind::Fingerprints) {
            self.file_md5 = None;
            self.file_crc32 = None;
        }
    }
}
