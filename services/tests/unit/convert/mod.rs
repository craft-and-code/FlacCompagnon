use super::*;

/// Cancellation is checked before the first packet is decoded, so an
/// already-cancelled batch never reads or writes anything at all — and
/// in particular never creates `dest`, which is what [`undo_batch`]
/// would otherwise have to clean up.
#[test]
fn an_already_cancelled_convert_writes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("in.wav");
    let dest = dir.path().join("out/track.flac");
    // Contents don't matter: cancellation is checked before any decode.
    std::fs::write(&src, b"not really a wav").expect("write");

    let settings = ConvertSettings {
        format: ConvertFormat::Flac,
        bitrate_kbps: None,
        flac_effort: FlacEffort::default(),
        preserve_modtime: false,
    };
    let err = convert_file(&src, &dest, &settings, &|| true).expect_err("cancelled");
    assert!(matches!(err, ConvertError::Cancelled(_)), "{err:?}");
    assert!(!dest.exists());
}

/// DSD is rejected on the extension alone, before any decode — so this
/// stays a named error rather than whatever Symphonia would have said
/// about a container it can't open.
#[test]
fn dsd_is_rejected_without_decoding() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("in.dsf");
    let dest = dir.path().join("out.flac");
    std::fs::write(&src, b"DSD ").expect("write");

    let settings = ConvertSettings {
        format: ConvertFormat::Flac,
        bitrate_kbps: None,
        flac_effort: FlacEffort::default(),
        preserve_modtime: false,
    };
    let err = convert_file(&src, &dest, &settings, &|| false).expect_err("unsupported");
    assert!(matches!(err, ConvertError::Unsupported(_, _)), "{err:?}");
    assert!(!dest.exists());
}

#[test]
fn converting_a_file_onto_itself_cannot_destroy_the_source() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&source, spec).unwrap();
    for sample in [0i16, 100, -100, 0] {
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();
    let original = std::fs::read(&source).unwrap();
    let settings = ConvertSettings {
        format: ConvertFormat::Wav,
        bitrate_kbps: None,
        flac_effort: FlacEffort::default(),
        preserve_modtime: false,
    };
    assert!(convert_file(&source, &source, &settings, &|| false).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
}
