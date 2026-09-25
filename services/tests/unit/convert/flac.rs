use super::*;

/// The bug that made a FLAC this app wrote unreadable by this app.
///
/// Ground truth is the format specification, not the encoder: a stream
/// whose frames use the fixed blocking strategy must report
/// `min_blocksize == max_blocksize`. The length here is deliberately not
/// a multiple of the block size, which is the only case that triggers it
/// — the final short block is what `flacenc` reported as the minimum.
#[test]
fn streaminfo_declares_a_fixed_block_stream_even_with_a_short_final_block() {
    let channels = 2usize;
    let bits = 16u32;
    // Two full 4096-sample blocks plus a deliberately partial third.
    let frames = 4096 * 2 + 501;
    let mut samples = Vec::with_capacity(frames * channels);
    for t in 0..frames {
        let phase = t as f32 / 44_100.0;
        samples.push((phase * 440.0 * std::f32::consts::TAU).sin() * 0.5);
        samples.push((phase * 220.0 * std::f32::consts::TAU).sin() * 0.5);
    }
    let expected_ints = f32_to_ints(&samples, bits);
    let pcm = PcmAudio {
        samples,
        sample_rate: 44_100,
        channels,
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("partial-tail.flac");
    encode(&pcm, &dest, bits, FlacEffort::Balanced).expect("encode");

    let bytes = std::fs::read(&dest).expect("read back");
    assert_eq!(&bytes[..4], b"fLaC");
    let min = u16::from_be_bytes([bytes[8], bytes[9]]);
    let max = u16::from_be_bytes([bytes[10], bytes[11]]);
    assert_eq!(
        min, max,
        "STREAMINFO declares a variable-block stream ({min}/{max}) while the \
             frames use the fixed strategy — strict decoders reject this"
    );

    // And the patch must not have broken the audio: same independent
    // decoder, same bit-exact expectation as the round-trip test.
    let mut reader = claxon::FlacReader::open(&dest).expect("reopen");
    let decoded: Vec<i32> = reader.samples().map(|s| s.expect("decode")).collect();
    assert_eq!(decoded.len(), expected_ints.len());
    assert!(
        decoded == expected_ints,
        "patching STREAMINFO changed the decoded audio"
    );

    // The symptom this is all named after: a file this app wrote, that
    // the transcoding detector then could not decode a second time.
    let pcm = crate::decode::decode_to_pcm(&dest)
        .expect("the app must be able to re-read what it just wrote");
    assert_eq!(pcm.sample_rate, 44_100);
    assert_eq!(pcm.channels, channels);
    assert_eq!(pcm.samples.len(), frames * channels);
}

/// The patch refuses anything that is not the layout it expects, rather
/// than rewriting two arbitrary bytes of it.
#[test]
fn the_streaminfo_patch_refuses_a_stream_it_does_not_recognise() {
    assert!(declare_fixed_block_size(&mut []).is_err(), "empty");
    assert!(declare_fixed_block_size(&mut b"not a flac file at all".to_vec()).is_err());
    // Right marker, wrong first block type (4 = VORBIS_COMMENT).
    let mut wrong = b"fLaC".to_vec();
    wrong.push(4);
    wrong.extend_from_slice(&[0; 40]);
    assert!(declare_fixed_block_size(&mut wrong).is_err(), "wrong block");
    // Right marker and block type, truncated before the block sizes.
    let mut short = b"fLaC".to_vec();
    short.extend_from_slice(&[0, 0, 0, 34, 1, 2]);
    assert!(declare_fixed_block_size(&mut short).is_err(), "truncated");
}

/// A synthetic tone in, decoded back with `claxon` — a different crate
/// from the one that encoded it — and compared sample-for-sample. FLAC is
/// lossless by definition, so this must be an exact match, not just a
/// close one; using an independent decoder is what makes the ground
/// truth here actually independent of `flacenc`'s own correctness.
#[test]
fn round_trips_bit_exact_through_an_independent_decoder() {
    let sample_rate = 44_100u32;
    let channels = 2usize;
    let bits = 16u32;

    // Two channels of a simple, non-trivial waveform — not silence, so a
    // bug that only shows up on nonzero samples wouldn't hide here.
    let frames = 2000;
    let mut samples = Vec::with_capacity(frames * channels);
    for t in 0..frames {
        let phase = t as f32 / sample_rate as f32;
        let l = (phase * 440.0 * std::f32::consts::TAU).sin() * 0.5;
        let r = (phase * 220.0 * std::f32::consts::TAU).sin() * 0.5;
        samples.push(l);
        samples.push(r);
    }
    let expected_ints = f32_to_ints(&samples, bits);

    let pcm = PcmAudio {
        samples,
        sample_rate,
        channels,
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("tone.flac");
    encode(&pcm, &dest, bits, FlacEffort::Balanced).expect("encode");

    let mut reader = claxon::FlacReader::open(&dest).expect("reopen");
    let decoded: Vec<i32> = reader.samples().map(|s| s.expect("decode")).collect();

    // A plain `assert_eq!` on a several-thousand-element Vec dumps both
    // sides in full, which is unreadable — this instead reports the
    // length (a truncated/padded tail block would show up here) and, if
    // lengths match, the first differing index with a window either side
    // (a channel swap, an off-by-one, or an isolated rounding
    // difference each look different in that window).
    assert_eq!(
        decoded.len(),
        expected_ints.len(),
        "decoded {} samples, expected {}",
        decoded.len(),
        expected_ints.len()
    );
    if let Some(i) = (0..decoded.len()).find(|&i| decoded[i] != expected_ints[i]) {
        let lo = i.saturating_sub(6);
        let hi = (i + 6).min(decoded.len());
        panic!(
            "first mismatch at index {i} (frame {}, {}): decoded={:?} expected={:?}",
            i / channels,
            if i % channels == 0 { "L" } else { "R" },
            &decoded[lo..hi],
            &expected_ints[lo..hi],
        );
    }
}
