//! The generic decode-and-analyze path: any format Symphonia supports, fed
//! frame by frame into a [`StreamAnalyzer`].
//!
//! FLAC does not come through here — it has a fused path ([`super::flac`])
//! that also computes the STREAMINFO MD5 in the same pass — but it is the
//! fallback when that path fails on a malformed FLAC.

use std::path::Path;

use symphonia::core::audio::AudioBufferRef;
use symphonia::core::errors::Error as SymError;

use super::container::{codec_label, format_label};
use super::probe::{probe, InterleavedBuf};
use super::DecodeOutcome;
use crate::analysis::analyzer::StreamAnalyzer;
use crate::AnalysisError;

/// Decode `path` and run streaming analysis over its samples.
pub fn decode_and_analyze(path: &Path) -> Result<DecodeOutcome, AnalysisError> {
    let mut probed = probe(path, true)?;
    let sample_rate = probed.sample_rate()?;
    let channels = probed.channels()?;
    let codec = codec_label(probed.params.codec).map(str::to_string);
    let declared_bits = probed.params.bits_per_sample;
    let declared_duration = probed
        .params
        .n_frames
        .map(|n| n as f64 / sample_rate as f64)
        .unwrap_or(0.0);
    let track_id = probed.track_id;
    let mut decoder = probed.make_decoder()?;

    // Integer conversion aligns samples to 32 bits. Restore the declared
    // width without a float round-trip, which would erase low bits above 24.
    let int_shift = declared_bits
        .filter(|b| (1..=32).contains(b))
        .map(|b| 32 - b);

    let mut analyzer = StreamAnalyzer::new(channels);
    let mut buf = InterleavedBuf::<f32>::default();
    let mut int_buf = InterleavedBuf::<i32>::default();
    let mut int_packet: Vec<i32> = Vec::new();
    let mut frame_count: u64 = 0;
    let mut saw_integer = false;

    loop {
        let packet = match probed.format.next_packet() {
            Ok(p) => p,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::ResetRequired) => {
                return Err(AnalysisError::Decode(
                    "stream changed during analysis".into(),
                ));
            }
            Err(e) => return Err(AnalysisError::Decode(format!("packet error: {e}"))),
        };
        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Float sources carry no meaningful integer bit depth.
                let is_int = !matches!(&decoded, AudioBufferRef::F32(_) | AudioBufferRef::F64(_));

                // Reconstruct native integers for the whole packet, once, into
                // a buffer reused across packets rather than a fresh Vec each
                // time (this runs per packet for the length of the file).
                int_packet.clear();
                if let (true, Some(shift)) = (is_int, int_shift) {
                    int_packet.extend(int_buf.fill(decoded.clone()).iter().map(|&s| s >> shift));
                }
                let f32_samples = buf.fill(decoded);
                let ints_ready = !int_packet.is_empty();
                saw_integer |= ints_ready;

                let ch = channels.max(1);
                let n_frames = f32_samples.len() / ch;
                for f in 0..n_frames {
                    let base = f * ch;
                    let frame = &f32_samples[base..base + ch];
                    let ints = ints_ready.then(|| &int_packet[base..base + ch]);
                    analyzer.push_frame(frame, ints);
                }
                frame_count += n_frames as u64;
            }
            // Missing samples could contain the only non-zero low bit. An
            // incomplete decode cannot establish whole-file zero padding.
            Err(SymError::DecodeError(e)) => {
                return Err(AnalysisError::Decode(format!("corrupt audio packet: {e}")));
            }
            Err(e) => return Err(AnalysisError::Decode(format!("decode error: {e}"))),
        }
    }

    // A normal end-of-packets also occurs on truncated PCM. Do not turn
    // analysis of a prefix into an exact claim about the entire file.
    if saw_integer && probed.params.n_frames.is_some_and(|n| frame_count < n) {
        return Err(AnalysisError::Decode(
            "incomplete integer audio stream".into(),
        ));
    }

    // Prefer the container's declared length; fall back to what we counted
    // when it doesn't declare one (common for streamed/edited files).
    let duration_secs = if declared_duration > 0.0 {
        declared_duration
    } else {
        frame_count as f64 / sample_rate as f64
    };

    Ok(DecodeOutcome {
        format: format_label(path, codec.as_deref()),
        codec,
        sample_rate,
        channels,
        declared_bits,
        duration_secs,
        analyzer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::detections::classify;
    use crate::transcode::LatticeSkip;

    fn measure(bits: u16, channels: u16, samples: &[i32]) -> Option<u32> {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("integer.wav");
        let mut writer = hound::WavWriter::create(
            &path,
            hound::WavSpec {
                channels,
                sample_rate: 96_000,
                bits_per_sample: bits,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .expect("WAV writer");
        for &sample in samples {
            writer.write_sample(sample).expect("valid integer sample");
        }
        writer.finalize().expect("finish WAV");
        let decoded = decode_and_analyze(&path).expect("decode WAV");
        let summary = decoded
            .analyzer
            .finish(decoded.sample_rate, decoded.declared_bits);
        let verdict = classify(
            &summary,
            decoded.sample_rate,
            decoded.declared_bits,
            summary.real_bit_depth,
            Err(LatticeSkip::TooShort),
        );
        if samples.iter().all(|&s| s == 0) {
            assert_eq!(verdict.summary, "Unknown");
            assert!(verdict.detail.contains("Digital silence"));
        }
        assert_eq!(
            verdict.upscaling,
            summary.real_bit_depth.is_some_and(|r| r < u32::from(bits))
        );
        summary.real_bit_depth
    }

    #[test]
    fn digital_silence_does_not_claim_one_bit_upscaling() {
        for bits in [8, 16, 24, 32] {
            assert_eq!(measure(bits, 1, &[0; 64]), None);
        }
    }

    #[test]
    fn full_resolution_32bit_samples_keep_their_low_bits() {
        // All are odd, but f32 rounds every one to an even integer.
        let samples = [0x4000_0001, -0x4000_0001, 0x2000_0001, -0x2000_0001];
        assert_eq!(measure(32, 1, &samples), Some(32));
    }

    #[test]
    fn exact_padding_is_detected_across_integer_widths() {
        for (bits, source) in [(16, 8), (24, 16), (24, 20), (32, 16), (32, 24)] {
            let shift = bits - source;
            let samples = [123 << shift, -125 << shift, 0, 127 << shift];
            assert_eq!(measure(bits, 1, &samples), Some(u32::from(source)));
        }
    }

    #[test]
    fn a_non_dither_residue_in_the_last_packet_and_other_channel_prevents_upscaling() {
        let mut samples = vec![123 << 8; 20_000];
        *samples.last_mut().expect("nonempty fixture") = 101;
        assert_eq!(measure(24, 2, &samples), Some(24));
    }

    #[test]
    fn quiet_integer_audio_does_not_lose_effective_depth() {
        assert_eq!(measure(24, 1, &[1, -1, 3, -3]), Some(24));
    }

    #[test]
    fn dither_in_low_bits_prevents_a_padding_claim() {
        let samples = [(123 << 8) + 1, (-125 << 8) - 1, 127 << 8];
        assert_eq!(measure(24, 1, &samples), Some(24));
    }

    #[test]
    fn audacity_style_dithered_16bit_export_is_detected_in_24bit() {
        let samples: Vec<i32> = (0..20_000)
            .map(|n| {
                let source_16bit = ((n % 60_001) - 30_000) << 8;
                let dither = (n % 21) - 10;
                source_16bit + dither
            })
            .collect();
        assert_eq!(measure(24, 1, &samples), Some(16));
    }

    #[test]
    fn truncated_pcm_cannot_receive_a_whole_file_padding_verdict() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("truncated.wav");
        let mut writer = hound::WavWriter::create(
            &path,
            hound::WavSpec {
                channels: 1,
                sample_rate: 96_000,
                bits_per_sample: 24,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .expect("WAV writer");
        for _ in 0..20_000 {
            writer.write_sample(123i32 << 8).expect("padded sample");
        }
        writer.write_sample(1i32).expect("only nonzero low bit");
        writer.finalize().expect("finish WAV");
        assert!(decode_and_analyze(&path).is_ok());
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("open WAV");
        let len = file.metadata().expect("WAV length").len();
        file.set_len(len - 3).expect("remove last sample");
        assert!(decode_and_analyze(&path).is_err());
    }

    #[test]
    fn floating_point_audio_has_no_integer_padding_verdict() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("float.wav");
        let mut writer = hound::WavWriter::create(
            &path,
            hound::WavSpec {
                channels: 1,
                sample_rate: 96_000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .expect("WAV writer");
        for sample in [0.25f32, -0.25, 0.5, -0.5] {
            writer.write_sample(sample).expect("float sample");
        }
        writer.finalize().expect("finish WAV");
        let decoded = decode_and_analyze(&path).expect("decode WAV");
        assert_eq!(
            decoded
                .analyzer
                .finish(decoded.sample_rate, decoded.declared_bits)
                .real_bit_depth,
            None
        );
    }
}
