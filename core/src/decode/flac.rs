//! FLAC's fused decode path: one pass that feeds the streaming analyzer *and*
//! computes the STREAMINFO MD5 over the exact original integer samples.
//!
//! Fused because both need every sample of the file: doing them separately
//! would decode the whole track twice for no gain. claxon is used here rather
//! than Symphonia because it hands back the raw decoded integers, which is
//! what the MD5 and bit-depth measurement must use. Spectral analysis can
//! use normalized floats without discarding the raw integer evidence.

use std::path::Path;

use md5::{Digest, Md5};

use super::FlacMd5Status;
use super::{validate_decoded_frames, DecodeOutcome};
use crate::analysis::analyzer::StreamAnalyzer;
use crate::AnalysisError;

/// Decode a FLAC into an interleaved `f32` buffer using claxon.
///
/// # Why this exists next to the Symphonia path
///
/// [`super::decode_to_pcm`] normally goes through Symphonia, which handles
/// every format the app accepts. Symphonia is also the stricter of the two
/// FLAC readers, and it refuses streams that claxon and ffmpeg accept — a
/// STREAMINFO that reports a variable block size while the frame headers say
/// fixed, for one, which is a file this app used to produce itself (see
/// `convert::flac::declare_fixed_block_size`). That bug is fixed at the
/// source now, but files written before the fix are on users' disks, and the
/// transcoding detector reported them as un-analysable.
///
/// So FLAC — the format this app is named after and the one it is most often
/// pointed at — gets decoded by the same library that already decodes it for
/// analysis and MD5 verification everywhere else. Symphonia stays as the
/// fallback, which keeps a claxon-specific failure from being fatal either.
///
/// Deliberately *not* merged with [`decode_and_analyze_flac`]: that function
/// exists to hash and analyse in one pass and would have to grow a mode flag
/// and a discarded output to serve this caller. The two share a library, not
/// a reason to change.
pub fn decode_flac_to_pcm(path: &Path) -> Result<crate::decode::PcmAudio, AnalysisError> {
    let mut reader = claxon::FlacReader::open(path)
        .map_err(|e| AnalysisError::Decode(format!("flac open failed: {e}")))?;
    let info = reader.streaminfo();
    let (sample_rate, channels, bits) = (
        info.sample_rate,
        info.channels as usize,
        info.bits_per_sample,
    );
    if sample_rate == 0 || channels == 0 || bits == 0 || bits > 32 {
        return Err(AnalysisError::Decode("invalid FLAC stream info".into()));
    }
    let scale = 1.0f32 / (1u64 << (bits - 1)) as f32;

    let mut samples: Vec<f32> = Vec::new();
    // `info.samples` is a hint from a header this crate does not trust, so it
    // sizes the allocation but never bounds the loop.
    if let Some(total) = info.samples {
        if let Ok(n) = usize::try_from(total) {
            samples.reserve(n.saturating_mul(channels).min(1 << 28));
        }
    }

    let mut blocks = reader.blocks();
    let mut buffer: Vec<i32> = Vec::new();
    let mut frame_count = 0;
    loop {
        let block = match blocks.read_next_or_eof(buffer) {
            Ok(Some(b)) => b,
            Ok(None) => break,
            Err(e) => return Err(AnalysisError::Decode(format!("flac decode error: {e}"))),
        };
        validate_block_channels(&block, channels)?;
        for t in 0..block.duration() as usize {
            for c in 0..channels {
                samples.push(block_sample(&block, c, t)? as f32 * scale);
            }
        }
        frame_count += u64::from(block.duration());
        buffer = block.into_buffer();
    }
    validate_decoded_frames(frame_count, info.samples)?;
    Ok(crate::decode::PcmAudio {
        samples,
        sample_rate,
        channels,
    })
}

/// Fused single-pass FLAC decode.
///
/// Correctness guarantees:
/// * The MD5 is hashed from claxon's raw decoded integers — no float
///   round-trip — using the exact layout mandated by the FLAC spec:
///   interleaved samples, each written as `ceil(bits/8)` little-endian
///   two's-complement bytes. Bit-identical to what `flac -t` verifies.
/// * The analyzer receives `s / 2^(bits-1)` as `f32`, which is exact for
///   FLAC's ≤ 24-bit integers (every such integer fits in the f32 mantissa),
///   plus the raw integers for effective-bit-depth analysis.
///
/// When `verify_md5` is false the hash comparison is skipped (the decode cost
/// is the same either way since analysis needs every sample).
pub fn decode_and_analyze_flac(
    path: &Path,
    verify_md5: bool,
) -> Result<(DecodeOutcome, FlacMd5Status), AnalysisError> {
    let mut reader = claxon::FlacReader::open(path)
        .map_err(|e| AnalysisError::Decode(format!("flac open failed: {e}")))?;
    let info = reader.streaminfo();
    let sample_rate = info.sample_rate;
    let channels = info.channels as usize;
    let bits = info.bits_per_sample;
    // Guards the shifts and the per-sample byte width below; a STREAMINFO
    // claiming 0 or > 32 bits is either corrupt or hostile.
    if sample_rate == 0 || channels == 0 || bits == 0 || bits > 32 {
        return Err(AnalysisError::Decode("invalid FLAC stream info".into()));
    }
    let total_frames_hint = info.samples;
    let stored = info.md5sum;
    let has_signature = stored != [0u8; 16];

    let scale = 1.0f32 / (1u64 << (bits - 1)) as f32;
    let bytes_per_sample = bits.div_ceil(8) as usize;
    let mut hasher = (has_signature && verify_md5).then(Md5::new);

    let mut analyzer = StreamAnalyzer::new(channels);
    let mut frame_f32 = vec![0.0f32; channels];
    let mut frame_i32 = vec![0i32; channels];
    let mut byte_row: Vec<u8> = Vec::new();
    let mut frame_count: u64 = 0;

    let mut blocks = reader.blocks();
    let mut buffer: Vec<i32> = Vec::new();
    loop {
        let block = match blocks.read_next_or_eof(buffer) {
            Ok(Some(b)) => b,
            Ok(None) => break,
            Err(e) => return Err(AnalysisError::Decode(format!("flac decode error: {e}"))),
        };
        validate_block_channels(&block, channels)?;
        let n = block.duration() as usize;
        if hasher.is_some() {
            byte_row.clear();
            byte_row.reserve(n * channels * bytes_per_sample);
        }
        for t in 0..n {
            for c in 0..channels {
                let s = block_sample(&block, c, t)?;
                frame_i32[c] = s;
                frame_f32[c] = s as f32 * scale;
                if hasher.is_some() {
                    let le = (s as u32).to_le_bytes();
                    byte_row.extend_from_slice(&le[..bytes_per_sample]);
                }
            }
            analyzer.push_frame(&frame_f32, Some(&frame_i32));
            frame_count += 1;
        }
        if let Some(h) = &mut hasher {
            h.update(&byte_row);
        }
        buffer = block.into_buffer();
    }
    validate_decoded_frames(frame_count, total_frames_hint)?;

    let md5_status = if !has_signature {
        FlacMd5Status::NoSignature
    } else if let Some(h) = hasher {
        // `finalize` consumes the hasher, so this cannot be a match guard.
        if h.finalize().as_slice() == stored.as_slice() {
            FlacMd5Status::Match
        } else {
            FlacMd5Status::Mismatch
        }
    } else {
        // A signature is present but verification was not asked for.
        FlacMd5Status::Present
    };

    let duration_secs = total_frames_hint
        .map(|n| n as f64 / sample_rate as f64)
        .unwrap_or(frame_count as f64 / sample_rate as f64);

    Ok((
        DecodeOutcome {
            format: "FLAC".to_string(),
            codec: None, // container already says everything this field would
            sample_rate,
            channels,
            declared_bits: Some(bits),
            duration_secs,
            analyzer,
        },
        md5_status,
    ))
}

fn validate_block_channels(
    block: &claxon::frame::Block,
    channels: usize,
) -> Result<(), AnalysisError> {
    if block.channels() as usize != channels {
        return Err(AnalysisError::Decode(
            "FLAC channel count changed during decoding".into(),
        ));
    }
    Ok(())
}

fn block_sample(
    block: &claxon::frame::Block,
    channel: usize,
    frame: usize,
) -> Result<i32, AnalysisError> {
    // `channel()` can panic, so validate against this block, not STREAMINFO.
    if channel >= block.channels() as usize {
        return Err(AnalysisError::Decode("invalid FLAC channel index".into()));
    }
    block
        .channel(channel as u32)
        .get(frame)
        .copied()
        .ok_or_else(|| AnalysisError::Decode("incomplete FLAC block".into()))
}

#[cfg(test)]
mod tests {
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
}
