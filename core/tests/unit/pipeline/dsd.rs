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

/// Header-only checks retain their explanation without a third verdict.
#[test]
fn informational_verdicts_flag_nothing() {
    for v in [verdict(false, "header only"), verdict(false, "native")] {
        assert!(!v.upscaling);
        assert!(!v.upsampling);
        assert!(!v.transcoding);
        assert_eq!(v.summary, "Clean");
    }
}
