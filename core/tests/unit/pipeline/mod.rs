use super::*;
use crate::analysis::detections::Detections;
use crate::types::ClippingInfo;

/// Hi-res-on-paper specs (96 kHz/24-bit) with everything else clean —
/// only `codec` varies between the test cases below.
fn hires_pcm(codec: Option<&str>) -> FileAnalysis {
    FileAnalysis {
        path: "/music/a.m4a".into(),
        file_name: "a.m4a".into(),
        format: "ALAC/MP4".into(),
        codec: codec.map(str::to_string),
        ext_mismatch: false,
        sample_rate: 96_000,
        channels: 2,
        declared_bits: Some(24),
        duration_secs: 200.0,
        size_bytes: 40_000_000,
        bitrate_kbps: None,
        modified_unix: None,
        detections: Detections {
            upscaling: false,
            upsampling: false,
            transcoding: false,
            detail: String::new(),
            summary: "Clean".into(),
        },
        cutoff_hz: None,
        cutoff_ratio: None,
        real_bit_depth: Some(24),
        bit_depth_evidence: None,
        lattice_score: None,
        fake_stereo: Some(false),
        phase_correlation: Some(0.8),
        phase_inverted: Some(false),
        badge: None,
        clipping: ClippingInfo::unmeasured(),
        dr_db: Some(14.0),
        integrated_lufs: Some(-23.0),
        flac_md5: None,
        file_md5: None,
        file_crc32: None,
        error: None,
    }
}

/// The bug this backstop exists for: an AAC stream sitting in an `.m4a`
/// container (extension/container implying ALAC, i.e. lossless) must
/// never earn a Hi-Res badge just because its declared specs look
/// hi-res — the codec itself says lossy, full stop.
#[test]
fn hires_badge_refuses_a_known_lossy_codec() {
    assert_eq!(hires_badge(&hires_pcm(Some("AAC")), None), None);
    assert_eq!(hires_badge(&hires_pcm(Some("MP3")), None), None);
}

/// Same specs, lossless (or unresolved — FLAC/DSD always report `codec:
/// None`) codec: the guard must not become a blanket "no badge for a
/// multi-codec container" rule.
#[test]
fn hires_badge_still_grants_lossless_codecs() {
    assert_eq!(
        hires_badge(&hires_pcm(Some("ALAC")), None),
        Some("Hi-Res".to_string())
    );
    assert_eq!(
        hires_badge(&hires_pcm(None), None),
        Some("Hi-Res".to_string())
    );
}
