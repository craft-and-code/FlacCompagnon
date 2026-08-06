//! The DSD branch of the analysis pipeline.
//!
//! Separate from the PCM path in [`super`] because almost nothing is shared:
//! the container is parsed by hand rather than probed, decoding needs an
//! external ffmpeg, only one of the three detections can even apply, and the
//! badge is granted on different grounds. The two only meet at
//! [`FileAnalysis`], which both fill in.
//!
//! `crate::dsd` (container parsing + spectral heuristics) is aliased to
//! `dsd_format` here so it is never confused with this module.

use std::path::Path;

use crate::decode;
use crate::detections::{Detections, TranscodeState};
use crate::dsd as dsd_format;
use crate::types::{FileAnalysis, ScanOptions};

/// ffmpeg decodes a 1-bit DSD stream to PCM at one eighth of its rate
/// (352.8 kHz for DSD64). The spectral analysis must use *that* rate, not the
/// 1-bit rate from the header.
const DSD_TO_PCM_DIVISOR: u32 = 8;

/// Below this boundary the brick wall came from a 44.1 kHz source, above it
/// from a 48 kHz one — the two Nyquist frequencies are 22.05 and 24 kHz, so
/// anything under 23 kHz is the former.
const CD_NYQUIST_CEILING_HZ: f64 = 23_000.0;

/// Exact DSF/DFF header verification plus, when ffmpeg is available, a content
/// check on the decoded PCM (the PCM-source brick wall).
pub(super) fn analyze(path: &Path, opts: &ScanOptions, result: &mut FileAnalysis) {
    let info = match dsd_format::parse(path) {
        Ok(i) => i,
        Err(e) => {
            result.error = Some(e.to_string());
            return;
        }
    };
    result.format = info.label();
    result.sample_rate = info.sample_rate;
    result.channels = info.channels;
    result.duration_secs = info.duration_secs();

    let mut flagged: Option<dsd_format::PcmSourceCheck> = None;
    let mut analyzed = false;

    if info.dst_compressed {
        result.detections = verdict(
            false,
            "DST-compressed DFF: header verified; content analysis is not supported for DST streams.",
            "Unknown",
        );
    } else if let Some(ffmpeg) = &opts.ffmpeg {
        let decoded_rate = (info.sample_rate / DSD_TO_PCM_DIVISOR).max(1);
        match decode::decode_and_analyze_dsd(ffmpeg, path, info.channels, decoded_rate) {
            Ok(outcome) => {
                analyzed = true;
                // The DFF header carries a byte size, not a sample count, so
                // its duration is often unknown until the audio is decoded.
                if result.duration_secs == 0.0 {
                    result.duration_secs = outcome.duration_secs;
                }
                let summary = outcome.analyzer.finish(decoded_rate, None);
                result.cutoff_hz = Some(summary.cutoff_hz);
                result.cutoff_ratio = Some(summary.cutoff_ratio);
                result.clipping = summary.clipping.clone();
                result.dr_db = summary.dr_db;
                if info.channels >= 2 {
                    result.fake_stereo = Some(summary.fake_stereo);
                }
                flagged = dsd_format::pcm_source_check(
                    &summary.spectrum_db,
                    decoded_rate,
                    summary.fft_size(),
                );
                result.detections = match flagged {
                    Some(hit) => verdict(true, &pcm_source_detail(hit), "Flagged"),
                    None => verdict(
                        false,
                        "Content blends into the sigma-delta noise shaping with no PCM brick wall — consistent with native DSD.",
                        "Clean",
                    ),
                };
            }
            Err(e) => result.error = Some(e.to_string()),
        }
    } else {
        result.detections = verdict(
            false,
            "DSD header verified. Install ffmpeg to enable the content authenticity check.",
            "Unknown",
        );
    }

    // DSD badge: header authentic and content not flagged as PCM-sourced.
    // "(unverified)" when only the header could be checked, so the badge never
    // claims more than was actually measured.
    if result.error.is_none() && flagged.is_none() {
        result.badge = Some(if analyzed {
            info.label()
        } else {
            format!("{} (unverified)", info.label())
        });
    }
}

fn pcm_source_detail(hit: dsd_format::PcmSourceCheck) -> String {
    let source = if hit.boundary_hz < CD_NYQUIST_CEILING_HZ {
        "44.1 kHz"
    } else {
        "48 kHz"
    };
    format!(
        "Upsampling: PCM-sourced DSD — digital brick wall at ~{:.2} kHz ({:.0} dB drop). \
         The 1-bit stream was converted from a {source} PCM source.",
        hit.boundary_hz / 1000.0,
        hit.drop_db,
    )
}

/// A DSD verdict. Only `upsampling` can ever fire on this path: the other two
/// detections need a PCM bit depth and an MDCT grid, neither of which a 1-bit
/// stream has — so hardcoding them false here is a statement, not a shortcut.
fn verdict(upsampling: bool, detail: &str, summary: &str) -> Detections {
    Detections {
        upscaling: false,
        upsampling,
        transcoding: TranscodeState::None,
        detail: detail.to_string(),
        summary: summary.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_the_brick_wall_to_the_right_source_rate() {
        let cd = pcm_source_detail(dsd_format::PcmSourceCheck {
            boundary_hz: 22_050.0,
            drop_db: 51.0,
        });
        assert!(cd.contains("44.1 kHz"), "{cd}");
        assert!(cd.contains("22.05 kHz"), "{cd}");

        let dvd = pcm_source_detail(dsd_format::PcmSourceCheck {
            boundary_hz: 24_000.0,
            drop_db: 48.0,
        });
        assert!(dvd.contains("48 kHz"), "{dvd}");
    }

    /// The three non-flagged verdicts must never claim a detection fired.
    #[test]
    fn informational_verdicts_flag_nothing() {
        for v in [
            verdict(false, "header only", "Unknown"),
            verdict(false, "native", "Clean"),
        ] {
            assert!(!v.upscaling);
            assert!(!v.upsampling);
            assert_eq!(v.transcoding, TranscodeState::None);
        }
    }
}
