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
use super::{validate_decoded_frames, DecodeOutcome};
use crate::analysis::analyzer::StreamAnalyzer;
use crate::AnalysisError;

/// Decode `path` and run streaming analysis over its samples.
pub fn decode_and_analyze(path: &Path) -> Result<DecodeOutcome, AnalysisError> {
    let mut probed = probe(path, true)?;
    let sample_rate = probed.sample_rate()?;
    let channels = probed.channels()?;
    if sample_rate == 0 || channels == 0 {
        return Err(AnalysisError::Decode(
            "invalid audio stream parameters".into(),
        ));
    }
    let codec = codec_label(probed.params.codec).map(str::to_string);
    let declared_bits = probed.params.bits_per_sample;
    let declared_duration = probed
        .params
        .n_frames
        .map(|n| n as f64 / sample_rate as f64)
        .unwrap_or(0.0);
    let track_id = probed.track_id;
    let mut decoder = probed.make_decoder()?;

    // Symphonia integer conversion aligns samples to 32 bits. Restore the
    // declared width without a float round-trip, which erases low bits >24.
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
                if decoded.spec().rate != sample_rate || decoded.spec().channels.count() != channels
                {
                    return Err(AnalysisError::Decode(
                        "stream changed during analysis".into(),
                    ));
                }
                // Float sources carry no meaningful integer bit depth.
                let is_int = !matches!(&decoded, AudioBufferRef::F32(_) | AudioBufferRef::F64(_));
                if is_int && declared_bits.is_some() && int_shift.is_none() {
                    return Err(AnalysisError::Decode("invalid integer sample width".into()));
                }

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

                let n_frames = f32_samples.len() / channels;
                let mut int_frames = int_packet.chunks_exact(channels);
                for frame in f32_samples.chunks_exact(channels) {
                    analyzer.push_frame(frame, int_frames.next());
                }
                frame_count += n_frames as u64;
            }
            Err(e) => return Err(AnalysisError::Decode(format!("decode error: {e}"))),
        }
    }

    // A corrupt or truncated tail may hold the only non-zero low bit.
    // Integer formats have exact frame counts; lossy duration hints may
    // include encoder padding, so those remain presentation metadata.
    validate_decoded_frames(
        frame_count,
        saw_integer.then_some(probed.params.n_frames).flatten(),
    )?;

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

    fn write_integer_wav(path: &Path, bits: u16, channels: u16, samples: &[i32]) {
        let mut writer = hound::WavWriter::create(
            path,
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
    }

    fn measure(bits: u16, channels: u16, samples: &[i32]) -> Option<u32> {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("integer.wav");
        write_integer_wav(&path, bits, channels, samples);
        let decoded = decode_and_analyze(&path).expect("decode WAV");
        decoded
            .analyzer
            .finish(decoded.sample_rate, decoded.declared_bits)
            .real_bit_depth
    }

    #[test]
    fn full_resolution_32bit_samples_keep_their_low_bits() {
        // Each integer is odd; a float round-trip erases its low bit.
        let samples = [0x4000_0001, -0x4000_0001, 0x2000_0001, -0x2000_0001];
        assert_eq!(measure(32, 1, &samples), Some(32));
    }

    #[test]
    fn exact_padding_survives_integer_conversion_at_every_wav_width() {
        for (bits, source) in [(8, 4), (16, 8), (24, 16), (24, 20), (32, 16), (32, 24)] {
            let shift = bits - source;
            let samples = [3 << shift, -5 << shift, 0, 7 << shift];
            assert_eq!(measure(bits, 1, &samples), Some(u32::from(source)));
        }
    }

    #[test]
    fn full_depth_in_the_last_packet_and_second_channel_is_not_lost() {
        let mut samples = vec![123 << 8; 20_000];
        // A whole native-depth tail, only in the second channel, prevents
        // silence, channel downmix or early-stop shortcuts hiding evidence.
        for sample in samples.iter_mut().skip(18_001).step_by(2) {
            *sample = 101;
        }
        assert_eq!(measure(24, 2, &samples), Some(24));
    }

    #[test]
    fn low_amplitude_samples_still_use_the_full_integer_resolution() {
        assert_eq!(measure(24, 1, &[1, -1, 3, -3]), Some(24));
    }

    #[test]
    fn digital_silence_keeps_the_historical_one_bit_measurement() {
        for bits in [8, 16, 24, 32] {
            assert_eq!(measure(bits, 1, &[0; 64]), Some(1));
        }
    }

    #[test]
    fn truncated_pcm_cannot_receive_a_whole_file_padding_verdict() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("truncated.wav");
        let mut samples = vec![123 << 8; 20_000];
        samples.push(1);
        write_integer_wav(&path, 24, 1, &samples);
        assert!(decode_and_analyze(&path).is_ok());
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("open WAV");
        let len = file.metadata().expect("WAV length").len();
        file.set_len(len - 3).expect("remove final sample");
        assert!(decode_and_analyze(&path).is_err());
    }

    #[test]
    fn empty_pcm_does_not_receive_a_clean_or_padded_verdict() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("empty.wav");
        write_integer_wav(&path, 24, 1, &[]);
        assert!(decode_and_analyze(&path).is_err());
    }

    #[test]
    fn malformed_input_returns_an_error() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("garbage.wav");
        std::fs::write(&path, b"not a WAV stream").expect("write malformed fixture");
        assert!(decode_and_analyze(&path).is_err());
    }

    #[test]
    fn floating_point_audio_has_no_integer_padding_measurement() {
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
