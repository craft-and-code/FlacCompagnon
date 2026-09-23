use super::*;

/// PCM WAV is trivially lossless at a fixed depth: what goes in as an
/// integer must come back as the exact same integer. `hound` reads its
/// own files back here — a second, independent decode isn't needed the
/// way it is for FLAC's compressed bitstream, since there is no
/// bitstream format to get subtly wrong, only a fixed-width sample
/// layout `hound` itself defines on both ends.
#[test]
fn round_trips_exactly_at_16_bit() {
    let samples = vec![-1.0f32, -0.5, 0.0, 0.25, 0.5, 1.0, -1.0, 0.5];
    let pcm = PcmAudio {
        samples: samples.clone(),
        sample_rate: 44_100,
        channels: 2,
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("tone.wav");
    encode(&pcm, &dest).expect("encode");

    let mut reader = hound::WavReader::open(&dest).expect("reopen");
    let decoded: Vec<i32> = reader
        .samples::<i16>()
        .map(|s| s.expect("decode") as i32)
        .collect();
    assert_eq!(decoded, super::super::f32_to_ints(&samples, BIT_DEPTH));
}

/// A malformed destination (a parent folder that doesn't exist) must
/// come back as a named [`ConvertError`], not a panic — `convert_file`
/// creates the parent first in the normal path, but this module's own
/// `encode` doesn't assume it always will.
#[test]
fn missing_parent_folder_is_a_named_error_not_a_panic() {
    let pcm = PcmAudio {
        samples: vec![0.0, 0.0],
        sample_rate: 44_100,
        channels: 2,
    };
    let dest = Path::new("/definitely/not/a/real/folder/track.wav");
    assert!(encode(&pcm, dest).is_err());
}
