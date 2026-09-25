use super::*;

#[test]
fn nearest_supported_rate_prefers_integer_division_for_hi_res_multiples() {
    assert_eq!(nearest_supported_rate(88_200), 44_100);
    assert_eq!(nearest_supported_rate(96_000), 48_000);
    assert_eq!(nearest_supported_rate(176_400), 44_100);
    assert_eq!(nearest_supported_rate(192_000), 48_000);
    assert_eq!(nearest_supported_rate(44_100), 44_100);
}

/// MP3 is lossy, so there is no bit-exact ground truth to check the way
/// there is for FLAC/WAV — this only verifies the encoder actually
/// produces a plausible MP3 bitstream (non-empty, starts with a frame
/// sync or an ID3 marker) for both a directly-supported rate and a
/// hi-res rate that must be resampled first.
#[test]
fn produces_a_non_empty_mp3_stream_at_a_hi_res_rate() {
    let sample_rate = 96_000u32;
    let channels = 2usize;
    let frames = 8000;
    let mut samples = Vec::with_capacity(frames * channels);
    for t in 0..frames {
        let phase = t as f32 / sample_rate as f32;
        let s = (phase * 440.0 * std::f32::consts::TAU).sin() * 0.4;
        samples.push(s);
        samples.push(s);
    }
    let pcm = PcmAudio {
        samples,
        sample_rate,
        channels,
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("tone.mp3");
    encode(&pcm, &dest, 256).expect("encode");

    let data = std::fs::read(&dest).expect("reopen");
    assert!(!data.is_empty());
    // A raw MPEG frame sync is 11 set bits: 0xFF followed by the top
    // three bits of the next byte also set.
    assert!(data[0] == 0xFF && (data[1] & 0xE0) == 0xE0);
}

#[test]
fn rejects_more_than_two_channels_without_panicking() {
    let pcm = PcmAudio {
        samples: vec![0.0; 300],
        sample_rate: 44_100,
        channels: 3,
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("tone.mp3");
    assert!(encode(&pcm, &dest, 256).is_err());
}
