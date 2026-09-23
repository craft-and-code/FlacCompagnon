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
