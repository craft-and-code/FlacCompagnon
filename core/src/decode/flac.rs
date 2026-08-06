//! FLAC's fused decode path: one pass that feeds the streaming analyzer *and*
//! computes the STREAMINFO MD5 over the exact original integer samples.
//!
//! Fused because both need every sample of the file: doing them separately
//! would decode the whole track twice for no gain. claxon is used here rather
//! than Symphonia because it hands back the raw decoded integers, which is
//! what the MD5 must be computed over — a float round-trip would not be
//! bit-exact for the hash even though it is exact for the analysis.

use std::path::Path;

use md5::{Digest, Md5};

use super::DecodeOutcome;
use crate::analyzer::StreamAnalyzer;
use crate::flac_md5::FlacMd5Status;
use crate::AnalysisError;

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
        let n = block.duration() as usize;
        if hasher.is_some() {
            byte_row.clear();
            byte_row.reserve(n * channels * bytes_per_sample);
        }
        for t in 0..n {
            for c in 0..channels {
                let s = block.channel(c as u32)[t];
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
            sample_rate,
            channels,
            declared_bits: Some(bits),
            duration_secs,
            analyzer,
        },
        md5_status,
    ))
}

#[cfg(test)]
mod tests {
    /// The fused FLAC path feeds the analyzer with `s * (1 / 2^(bits-1))` as
    /// f32. This must be a *lossless* round-trip for every integer the format
    /// can produce at ≤ 24 bits — otherwise analysis results could drift from
    /// the exact samples the MD5 is computed over.
    #[test]
    fn f32_normalization_roundtrips_exactly_up_to_24_bits() {
        for bits in [8u32, 12, 16, 20, 24] {
            let scale = 1.0f32 / (1u64 << (bits - 1)) as f32;
            let max = (1i64 << (bits - 1)) - 1;
            let probes = [0i64, 1, -1, 2, -2, max, -max - 1, max / 3, -(max / 7), max - 1];
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
        for (bits, expected) in [
            (8u32, 1usize),
            (12, 2),
            (16, 2),
            (20, 3),
            (24, 3),
            (32, 4),
        ] {
            assert_eq!(bits.div_ceil(8) as usize, expected, "bits={bits}");
        }
    }
}
