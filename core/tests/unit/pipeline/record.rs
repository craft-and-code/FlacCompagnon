use super::*;

/// The skeleton must be usable for a file the decoder will later reject —
/// that is the whole reason it exists before decoding.
#[test]
fn a_file_that_cannot_be_decoded_still_gets_size_and_fingerprints() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("not-audio.flac");
    std::fs::write(&path, b"this is not a FLAC file").expect("write");

    let r = skeleton(&path);
    assert_eq!(r.file_name, "not-audio.flac");
    assert_eq!(r.size_bytes, 23);
    assert!(r.modified_unix.is_some());
    // The fingerprints are of the bytes on disk, so they exist here even
    // though nothing about this file is audio.
    assert_eq!(r.file_md5.as_deref().map(str::len), Some(32));
    assert_eq!(r.file_crc32.as_deref().map(str::len), Some(8));
    // Nothing has been measured yet, so nothing is claimed.
    assert!(!r.detections.upscaling && !r.detections.upsampling && !r.detections.transcoding);
    assert_eq!(r.sample_rate, 0);
    assert!(r.error.is_none());
}

/// A path that does not exist must not panic, and must not invent values.
#[test]
fn a_missing_file_leaves_the_optional_fields_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let r = skeleton(&dir.path().join("absent.flac"));
    assert_eq!(r.size_bytes, 0);
    assert!(r.modified_unix.is_none());
    assert!(r.file_md5.is_none());
    assert!(r.file_crc32.is_none());
}

#[test]
fn bitrate_is_size_over_duration_and_refuses_the_degenerate_cases() {
    // 1 MB over 8 seconds = 1000 kbps exactly.
    assert_eq!(bitrate_kbps(1_000_000, 8.0), Some(1000));
    assert_eq!(bitrate_kbps(1_000_000, 0.0), None, "zero duration");
    assert_eq!(
        bitrate_kbps(0, 300.0),
        Some(0),
        "an empty file has no bitrate, not no answer"
    );
    assert_eq!(bitrate_kbps(1_000_000, f64::NAN), None, "NaN duration");
}
