use super::*;

/// A file that never decoded must not look like it peaked at full scale.
#[test]
fn unmeasured_clipping_has_no_peak() {
    let c = ClippingInfo::unmeasured();
    assert!(!c.clipped);
    assert_eq!(c.clipped_samples, 0);
    assert_eq!(c.clip_events, 0);
    assert!(c.peak_dbfs.is_infinite() && c.peak_dbfs.is_sign_negative());
    assert!(c.true_peak_dbtp.is_infinite() && c.true_peak_dbtp.is_sign_negative());
}
