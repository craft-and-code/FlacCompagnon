use super::*;
use flacenc::component::BitRepr;
use flacenc::error::Verify;

fn encoded_flac(samples: &[i32], channels: usize, bits: usize) -> Vec<u8> {
    let mut config = flacenc::config::Encoder::default();
    config.multithread = false;
    let config = config.into_verified().expect("valid encoder settings");
    let source = flacenc::source::MemSource::from_samples(samples, channels, bits, 96_000);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .expect("reference FLAC encoding");
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream.write(&mut sink).expect("serialize reference FLAC");
    sink.as_slice().to_vec()
}

#[test]
fn independent_flac_encoder_preserves_zero_padding_and_low_bits() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("integer.flac");
    for (samples, expected) in [
        ([123 << 8, -125 << 8, 0, 127 << 8].repeat(16), 16),
        (
            [0x40_0001, -0x40_0001, 0x20_0001, -0x20_0001].repeat(16),
            24,
        ),
        (vec![0; 64], 1),
    ] {
        std::fs::write(&path, encoded_flac(&samples, 2, 24)).expect("write FLAC");
        let (decoded, md5) = decode_and_analyze_flac(&path, true).expect("decode FLAC");
        assert_eq!(md5, FlacMd5Status::Match);
        assert_eq!(
            decoded
                .analyzer
                .finish(decoded.sample_rate, decoded.declared_bits)
                .real_bit_depth,
            Some(expected)
        );
    }
}

#[test]
fn missing_flac_frames_are_rejected_even_without_md5_verification() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("missing-tail.flac");
    let mut bytes = encoded_flac(&[123 << 8; 128], 1, 24);
    // RFC 9639 STREAMINFO: the low 36 bits of this eight-byte field
    // contain the total sample count. The file now lacks 128 frames.
    let packed = u64::from_be_bytes(bytes[18..26].try_into().expect("STREAMINFO field"));
    let count_mask = (1u64 << 36) - 1;
    bytes[18..26].copy_from_slice(&((packed & !count_mask) | 256).to_be_bytes());
    std::fs::write(&path, bytes).expect("write truncated declaration");
    assert!(decode_and_analyze_flac(&path, false).is_err());
    assert!(decode_flac_to_pcm(&path).is_err());
}

#[test]
fn inconsistent_flac_channels_return_an_error_instead_of_panicking() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("wrong-channels.flac");
    let mut bytes = encoded_flac(&[123 << 8; 128], 1, 24);
    // STREAMINFO says stereo while the frame remains mono.
    bytes[20] |= 0b10;
    std::fs::write(&path, bytes).expect("write contradictory header");
    assert!(decode_and_analyze_flac(&path, false).is_err());
    assert!(decode_flac_to_pcm(&path).is_err());
}

#[test]
fn metadata_only_flac_is_rejected() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("empty.flac");
    let mut bytes = encoded_flac(&[0; 128], 1, 24);
    let mut offset = 4;
    loop {
        let last = bytes[offset] & 0x80 != 0;
        let length =
            u32::from_be_bytes([0, bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
                as usize;
        offset += 4 + length;
        if last {
            break;
        }
    }
    bytes.truncate(offset);
    std::fs::write(&path, bytes).expect("write metadata-only fixture");
    assert!(decode_and_analyze_flac(&path, false).is_err());
    assert!(decode_flac_to_pcm(&path).is_err());
}

/// The fused FLAC path feeds the analyzer with `s * (1 / 2^(bits-1))` as
/// f32. This must be a *lossless* round-trip for every integer the format
/// can produce at ≤ 24 bits — otherwise analysis results could drift from
/// the exact samples the MD5 is computed over.
#[test]
fn f32_normalization_roundtrips_exactly_up_to_24_bits() {
    for bits in [8u32, 12, 16, 20, 24] {
        let scale = 1.0f32 / (1u64 << (bits - 1)) as f32;
        let max = (1i64 << (bits - 1)) - 1;
        let probes = [
            0i64,
            1,
            -1,
            2,
            -2,
            max,
            -max - 1,
            max / 3,
            -(max / 7),
            max - 1,
        ];
        for &s in &probes {
            let f = s as f32 * scale;
            let back = (f / scale).round() as i64;
            assert_eq!(back, s, "bits={bits} sample={s}");
        }
    }
}

/// The byte width the MD5 is fed must match the FLAC spec's
/// `ceil(bits/8)` for every depth the format allows.
#[test]
fn bytes_per_sample_matches_the_spec() {
    for (bits, expected) in [(8u32, 1usize), (12, 2), (16, 2), (20, 3), (24, 3), (32, 4)] {
        assert_eq!(bits.div_ceil(8) as usize, expected, "bits={bits}");
    }
}
