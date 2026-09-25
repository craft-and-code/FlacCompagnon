use super::*;

#[test]
fn f32_to_ints_clamps_at_the_target_bit_depth() {
    let samples = [-2.0f32, -1.0, 0.0, 0.5, 1.0, 2.0];
    let got = f32_to_ints(&samples, 16);
    assert_eq!(got, vec![-32768, -32768, 0, 16384, 32767, 32767]);
}

/// Halving the rate must halve the frame count (not the sample count —
/// that stays interleaved by `channels`), and the resampled frames must
/// still be interleaved in the same channel order. A resampler that
/// treated the buffer as mono would pass a frame-count check and still
/// scramble left and right, which is why this asserts on values too.
#[test]
fn halving_the_rate_halves_the_frames_and_keeps_the_channels_apart() {
    // Two channels held at distinct constant values: any interpolation
    // between them stays at that value, so a channel swap is visible.
    let channels = 2;
    let samples: Vec<f32> = (0..8).flat_map(|_| [1.0f32, -1.0]).collect();
    let got = resample_linear(&samples, channels, 48_000, 24_000);
    assert_eq!(got.len(), 4 * channels);
    for frame in got.chunks(channels) {
        assert_eq!(frame[0], 1.0);
        assert_eq!(frame[1], -1.0);
    }
}

#[test]
fn an_unchanged_rate_is_returned_verbatim() {
    let samples = vec![0.25f32, -0.25, 0.5, -0.5];
    assert_eq!(resample_linear(&samples, 2, 44_100, 44_100), samples);
}

/// Degenerate inputs must return rather than panicking on a
/// divide-by-zero or an out-of-range index — the callers hand this
/// whatever `decode_to_pcm` produced, and a zero-length or zero-channel
/// decode is exactly the malformed-file case this app exists to run
/// into. A zero channel count has no frame layout to resample *to*, so
/// the buffer comes back untouched rather than emptied: dropping the
/// samples would turn a header oddity into silent data loss further down
/// the encoder chain.
#[test]
fn empty_or_zero_channel_input_does_not_panic() {
    assert!(resample_linear(&[], 2, 44_100, 48_000).is_empty());
    assert_eq!(
        resample_linear(&[0.1, 0.2], 0, 44_100, 48_000),
        vec![0.1, 0.2]
    );
}
