use super::*;

#[test]
fn matching_flac_md5_is_displayed_as_ok_without_option_debug_syntax() {
    assert_eq!(flac_md5_label(Some(&FlacMd5Status::Match)), "OK");
}

#[test]
fn unavailable_flac_md5_is_explicit() {
    assert_eq!(flac_md5_label(None), "N/A");
}

#[test]
fn failed_flac_md5_checks_keep_the_core_verdict() {
    assert_eq!(flac_md5_label(Some(&FlacMd5Status::Mismatch)), "MISMATCH");
    assert_eq!(
        flac_md5_label(Some(&FlacMd5Status::Error("read failed".into()))),
        "check error: read failed"
    );
}
