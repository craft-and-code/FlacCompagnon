use super::*;

/// A full round trip through the real encoder: encode a short stereo
/// tone, then read the file back with `ogg`'s own `PacketReader` and
/// confirm the container shape a real Opus decoder would also check —
/// `OpusHead`/`OpusTags` magic first, then at least one audio packet with
/// a positive granule position. This doesn't decode the Opus audio
/// itself (lossy, so there is no bit-exact ground truth the way FLAC/WAV
/// have) — it verifies the *container* this module hand-builds is the
/// shape RFC 7845 requires, independent of `audiopus`'s own correctness.
#[test]
fn produces_a_well_formed_ogg_opus_stream() {
    let sample_rate = 44_100u32;
    let channels = 2usize;
    let frames = 4000;
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
    let dest = dir.path().join("tone.opus");
    encode(&pcm, &dest, 96).expect("encode");

    let file = std::fs::File::open(&dest).expect("reopen");
    let mut reader = ogg::reading::PacketReader::new(file);
    let head = reader
        .read_packet()
        .expect("read head packet")
        .expect("head packet present");
    assert!(head.data.starts_with(b"OpusHead"));
    let tags = reader
        .read_packet()
        .expect("read tags packet")
        .expect("tags packet present");
    assert!(tags.data.starts_with(b"OpusTags"));
    let first_audio = reader
        .read_packet()
        .expect("read first audio packet")
        .expect("audio packet present");
    assert!(!first_audio.data.is_empty());
    assert!(first_audio.absgp_page() > 0);
}
