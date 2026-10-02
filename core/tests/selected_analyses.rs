//! Selection correctness against independently encoded PCM fixtures.
use flaccompagnon_core::{analyze_file_selected, AnalysisSelection, FlacMd5Status, ScanOptions};
use flacenc::component::BitRepr;
use flacenc::error::Verify;

fn encoded_flac() -> Vec<u8> {
    let mut config = flacenc::config::Encoder::default();
    config.multithread = false;
    let config = config.into_verified().expect("encoder settings");
    let samples = [123, -456, 789, -901].repeat(4096);
    let source = flacenc::source::MemSource::from_samples(&samples, 2, 16, 44_100);
    let stream =
        flacenc::encode_with_fixed_block_size(&config, source, config.block_size).expect("encode");
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream.write(&mut sink).expect("serialize");
    sink.as_slice().to_vec()
}

#[test]
fn combining_md5_bit_depth_and_stereo_leaves_unrequested_results_unmeasured() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("track.flac");
    std::fs::write(&path, encoded_flac()).expect("fixture");
    let selection =
        AnalysisSelection::from_names(["flac-md5", "bit-depth", "stereo"]).expect("names");
    let file = analyze_file_selected(&path, &ScanOptions::default(), selection);
    assert!(file.error.is_none());
    assert_eq!(file.flac_md5, Some(FlacMd5Status::Match));
    assert_eq!(file.real_bit_depth, Some(16));
    assert!(file.fake_stereo.is_some() && file.stereo_balance.is_some());
    assert!(file.integrated_lufs.is_none() && file.cutoff_hz.is_none());
    assert!(file.file_md5.is_none() && file.file_crc32.is_none());
    assert!(file.phase_correlation.is_none() && file.local_phase.is_none());
    assert!(file.high_frequency_stereo.is_none() && file.discontinuities.is_none());
    assert!(file.dc_offset.is_none() && file.dr_db.is_none() && file.badge.is_none());
    assert_eq!(file.detections.summary, "Not analyzed");
    assert!(file.covers_selection(selection));
    assert!(!file.covers_selection(AnalysisSelection::all()));
    let mut projected = file;
    projected.restrict_to(AnalysisSelection::all());
    assert!(!projected.covers_selection(AnalysisSelection::all()));
    assert!(projected.covers_selection(selection));
}

#[test]
fn fingerprints_alone_do_not_decode_malformed_audio() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("not-audio.flac");
    std::fs::write(&path, b"abc").expect("fixture");
    let selection = AnalysisSelection::from_names(["fingerprints"]).expect("name");
    let file = analyze_file_selected(&path, &ScanOptions::default(), selection);
    assert!(file.error.is_none());
    assert_eq!(
        file.file_md5.as_deref(),
        Some("900150983cd24fb0d6963f7d28e17f72")
    );
    assert_eq!(file.file_crc32.as_deref(), Some("352441c2"));
    assert!(file.flac_md5.is_none());
    assert!(file.cutoff_hz.is_none());
    assert_eq!(file.detections.summary, "Not analyzed");
}

#[test]
fn fingerprints_alone_still_report_unreadable_files() {
    let dir = tempfile::tempdir().expect("directory");
    let selection = AnalysisSelection::from_names(["fingerprints"]).expect("name");
    let file = analyze_file_selected(
        &dir.path().join("missing.flac"),
        &ScanOptions::default(),
        selection,
    );
    assert!(file.error.is_some());
    assert!(file.file_md5.is_none() && file.file_crc32.is_none());
}

#[test]
fn authenticity_dependencies_are_not_reported_as_extra_requested_measurements() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("track.flac");
    std::fs::write(&path, encoded_flac()).expect("fixture");
    let selection = AnalysisSelection::from_names(["authenticity"]).expect("name");
    let file = analyze_file_selected(&path, &ScanOptions::default(), selection);
    assert!(file.error.is_none());
    assert!(file.flac_md5.is_none() && file.real_bit_depth.is_none() && file.cutoff_hz.is_none());
    assert_eq!(file.analyses_run, Some(vec!["authenticity".into()]));
    assert_ne!(file.detections.summary, "Not analyzed");
}
