use super::*;

#[test]
fn explicit_subsets_do_not_enable_unrequested_measurements() {
    let selected = AnalysisSelection::from_names(["flac-md5", "loudness"]).expect("known names");
    assert!(selected.enabled(AnalysisKind::FlacMd5));
    assert!(selected.enabled(AnalysisKind::Loudness));
    assert!(!selected.enabled(AnalysisKind::Authenticity));
    assert!(!selected.enabled(AnalysisKind::Fingerprints));
    assert!(!selected.spectrum());
    assert!(!selected.bit_depth());
    assert!(selected.audio());
    assert_eq!(selected.names(), ["loudness", "flac-md5"]);
}

#[test]
fn authenticity_dependencies_do_not_change_explicit_selection() {
    let selected = AnalysisSelection::from_names(["authenticity"]).expect("known name");
    assert!(selected.spectrum() && selected.bit_depth() && selected.sample_peak());
    assert_eq!(selected.names(), ["authenticity"]);
    assert!(!selected.enabled(AnalysisKind::Clipping));
}

#[test]
fn invalid_selection_is_rejected() {
    assert!(AnalysisSelection::from_names(["unknown"]).is_err());
    assert!(!AnalysisSelection::from_names(["fingerprints"])
        .expect("name")
        .audio());
}

#[test]
fn dsd_authenticity_needs_spectrum_without_pcm_only_detectors() {
    let selection = AnalysisSelection::from_names(["authenticity", "loudness", "clicks"])
        .expect("known names")
        .dsd_pcm();
    assert_eq!(selection.names(), ["spectrum", "loudness"]);
    assert!(!selection.bit_depth() && !selection.sample_peak());
    assert!(!AnalysisSelection::from_names(["bit-depth", "dropouts"])
        .expect("known names")
        .dsd_pcm()
        .audio());
}
