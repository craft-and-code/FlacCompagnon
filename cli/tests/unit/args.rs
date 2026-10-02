use super::*;
use std::path::Path;

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
    assert_eq!(args.selection().names(), ["phase", "loudness"]);
}

#[test]
fn json_destination_is_kept_separate_from_audio_targets() {
    let args = Args::parse(["--json".into(), "report.json".into(), "album".into()])
        .unwrap()
        .unwrap();
    assert_eq!(args.json.as_deref(), Some(Path::new("report.json")));
    assert_eq!(args.targets, ["album"]);
    assert!(args.selection().is_full());
}

#[test]
fn invalid_json_layout_and_conflicting_destinations_are_rejected() {
    for options in [
        vec!["--json-layout", "wrong", "album"],
        vec!["--json-layout", "album", "--json", "report.json", "album"],
    ] {
        assert!(Args::parse(options.into_iter().map(str::to_string)).is_err());
    }
}

#[test]
fn flac_md5_alone_skips_full_analysis_and_json_defaults_to_all_analyses() {
    for (options, expected) in [
        (vec!["-a", "flac-md5", "track.flac"], true),
        (vec!["--json", "report.json", "track.flac"], false),
        (vec!["--json-layout", "album", "track.flac"], false),
        (vec!["-a", "flac-md5,loudness", "track.flac"], false),
        (vec!["track.flac"], false),
    ] {
        let args = Args::parse(options.into_iter().map(String::from))
            .expect("valid args")
            .expect("scan");
        assert_eq!(args.md5_only(), expected);
        if args.json.is_some() || args.json_layout.is_some() || args.analyses.is_empty() {
            assert!(args.selection().is_full());
        }
    }
}

#[test]
fn selected_analyses_and_json_are_rejected_in_either_argument_order() {
    for options in [
        vec!["-a", "flac-md5", "--json", "report.json", "track.flac"],
        vec![
            "-j",
            "report.json",
            "--analysis=loudness,phase",
            "track.flac",
        ],
        vec![
            "--json-layout",
            "album",
            "-a",
            "phase",
            "-a",
            "loudness",
            "track.flac",
        ],
        vec!["-a", "fingerprints", "--json-layout=artist", "track.flac"],
    ] {
        let error = match Args::parse(options.into_iter().map(String::from)) {
            Err(error) => error,
            Ok(_) => panic!("JSON must not silently override an explicit analysis selection"),
        };
        assert!(error.contains("JSON reports require all analyses"));
        assert!(error.contains("Remove --analysis"));
    }
}
