use super::*;

#[test]
fn unknown_analysis_is_rejected() {
    let parsed = Args::parse(["--analysis".into(), "unknown".into(), "track.flac".into()]);
    assert!(parsed.is_err());
}

#[test]
fn repeated_analyses_select_only_requested_terminal_fields() {
    let args = Args::parse([
        "-a".into(),
        "phase".into(),
        "--analysis=loudness".into(),
        "track.flac".into(),
    ])
    .unwrap()
    .unwrap();
    assert!(args.selected("phase"));
    assert!(args.selected("loudness"));
    assert!(!args.selected("clipping"));
}

#[test]
fn json_destination_is_kept_separate_from_audio_targets() {
    let args = Args::parse(["--json".into(), "report.json".into(), "album".into()])
        .unwrap()
        .unwrap();
    assert_eq!(args.json.as_deref(), Some(Path::new("report.json")));
    assert_eq!(args.targets, ["album"]);
}
