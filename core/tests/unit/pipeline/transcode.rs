use super::*;

#[test]
fn a_missing_codec_cannot_be_cleared_by_the_other_codecs_negative() {
    let negative = || TranscodeEvidence {
        likelihood: 0.01,
        offset: 0,
        detected: false,
    };
    assert!(matches!(
        combine(None, Some(negative()), 0.0125, 0.031),
        Err(LatticeSkip::TooShort)
    ));
    assert!(matches!(
        combine(Some(negative()), None, 0.0125, 0.031),
        Err(LatticeSkip::TooShort)
    ));
    assert!(
        !combine(Some(negative()), Some(negative()), 0.0125, 0.031)
            .unwrap()
            .detected
    );
}

#[test]
fn a_positive_remains_useful_when_the_other_codec_cannot_run() {
    let positive = || TranscodeEvidence {
        likelihood: 0.1,
        offset: 7,
        detected: true,
    };
    assert!(
        combine(None, Some(positive()), 0.0125, 0.031)
            .unwrap()
            .detected
    );
    assert!(
        combine(Some(positive()), None, 0.0125, 0.031)
            .unwrap()
            .detected
    );
}

#[test]
fn deinterleave_splits_channels_in_order() {
    // L R L R L R
    let inter = [1.0f32, -1.0, 2.0, -2.0, 3.0, -3.0];
    let got = deinterleave(&inter, 2).expect("two channels");
    assert_eq!(got.len(), 2);
    assert_eq!(got[0], vec![1.0, 2.0, 3.0]);
    assert_eq!(got[1], vec![-1.0, -2.0, -3.0]);
}

#[test]
fn mono_stays_one_channel() {
    let got = deinterleave(&[1.0f32, 2.0, 3.0], 1).expect("mono");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0], vec![1.0, 2.0, 3.0]);
}

/// Surround is truncated to the front pair, and — the part worth
/// asserting — the front pair must still be the *right* samples, not
/// whatever the first two slots of a 6-channel frame happen to hold after
/// a stride mistake.
#[test]
fn surround_keeps_the_front_pair_correctly_strided() {
    let mut inter = Vec::new();
    for f in 0..4 {
        for c in 0..6 {
            inter.push((f * 10 + c) as f32);
        }
    }
    let got = deinterleave(&inter, 6).expect("surround");
    assert_eq!(got.len(), MAX_CHANNELS);
    assert_eq!(got[0], vec![0.0, 10.0, 20.0, 30.0]);
    assert_eq!(got[1], vec![1.0, 11.0, 21.0, 31.0]);
}

/// A decoder returning a partial final frame must not produce channels of
/// unequal length: the detector zips them against each other.
#[test]
fn a_truncated_final_frame_is_dropped_not_padded() {
    // Five samples across two channels: two whole frames plus a stray.
    let got = deinterleave(&[1.0f32, 2.0, 3.0, 4.0, 5.0], 2).expect("ragged");
    assert_eq!(got[0].len(), got[1].len(), "channels must match in length");
    assert_eq!(got[0], vec![1.0, 3.0]);
    assert_eq!(got[1], vec![2.0, 4.0]);
}

#[test]
fn degenerate_inputs_are_refused_without_panicking() {
    assert!(deinterleave(&[], 2).is_none(), "empty");
    assert!(deinterleave(&[1.0, 2.0], 0).is_none(), "zero channels");
    // Fewer samples than one frame: nothing to analyze.
    assert!(deinterleave(&[1.0], 2).is_none(), "partial single frame");
}

/// The reason matters as much as the refusal: these two used to be the
/// same `None` as "clean-but-short", and that ambiguity is what made a
/// dash in the Lattice column impossible to act on.
#[test]
fn an_undecodable_file_says_so_rather_than_giving_a_verdict() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("not-audio.flac");
    std::fs::write(&path, b"this is not a FLAC file").expect("write");
    assert!(matches!(
        detect(&path, &AacParams::default(), &Mp3Params::default(), &|| {
            false
        }),
        Err(LatticeSkip::Undecodable(_))
    ));
    // A missing file, too.
    assert!(matches!(
        detect(
            dir.path().join("absent.wav").as_path(),
            &AacParams::default(),
            &Mp3Params::default(),
            &|| false
        ),
        Err(LatticeSkip::Undecodable(_))
    ));
}

#[test]
fn cancellation_is_checked_before_the_decode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("whatever.flac");
    assert_eq!(
        detect(&path, &AacParams::default(), &Mp3Params::default(), &|| {
            true
        }),
        Err(LatticeSkip::Cancelled)
    );
}

/// Every reason must read as a sentence a user can act on — an empty or
/// duplicated one would leave the table saying nothing again.
#[test]
fn every_skip_reason_is_distinct_and_non_empty() {
    let all = [
        LatticeSkip::Undecodable("boom".into()),
        LatticeSkip::NoSamples,
        LatticeSkip::UnsupportedRate(96_000),
        LatticeSkip::TooShort,
        LatticeSkip::Cancelled,
    ];
    let texts: Vec<String> = all.iter().map(|s| s.to_string()).collect();
    for t in &texts {
        assert!(!t.is_empty());
    }
    for (i, a) in texts.iter().enumerate() {
        for b in texts.iter().skip(i + 1) {
            assert_ne!(a, b);
        }
    }
    assert!(texts[0].contains("boom"), "{}", texts[0]);
    assert!(texts[2].contains("96.0 kHz"), "{}", texts[2]);
}
