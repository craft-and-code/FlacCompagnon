use super::*;
use std::io::Write;

fn with_bytes(name: &str, bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    let mut f = std::fs::File::create(&path).expect("create");
    f.write_all(bytes).expect("write");
    (dir, path)
}

#[test]
fn detects_containers_from_magic_bytes() {
    let cases: &[(&[u8], &str)] = &[
        (b"fLaC\0\0\0\0\0\0\0\0\0\0\0\0", "FLAC"),
        (b"DSD \0\0\0\0\0\0\0\0\0\0\0\0", "DSF"),
        (b"FRM8\0\0\0\0\0\0\0\0DSD ", "DFF"),
        (b"RIFF\0\0\0\0WAVE\0\0\0\0", "WAV"),
        (b"RF64\0\0\0\0WAVE\0\0\0\0", "WAV"),
        (b"FORM\0\0\0\0AIFF\0\0\0\0", "AIFF"),
        (b"FORM\0\0\0\0AIFC\0\0\0\0", "AIFF"),
        (b"OggS\0\0\0\0\0\0\0\0\0\0\0\0", "OGG"),
        (b"caff\0\0\0\0\0\0\0\0\0\0\0\0", "CAF"),
        (b"\0\0\0\0ftyp\0\0\0\0\0\0\0\0", "MP4"),
        (b"ID3\x04\0\0\0\0\0\0\0\0\0\0\0", "MP3"),
    ];
    for (bytes, expected) in cases {
        let (_d, p) = with_bytes("probe.bin", bytes);
        assert_eq!(detect_container(&p), Some(*expected), "for {expected}");
    }
}

/// A file shorter than the magic being tested must answer `None`, not
/// index past the end of what was actually read.
#[test]
fn short_files_do_not_panic() {
    for len in 0..16usize {
        let bytes = vec![0xFFu8; len];
        let (_d, p) = with_bytes("short.bin", &bytes);
        // Only the assertion that it returns at all matters here.
        let _ = detect_container(&p);
    }
    // Four bytes is the minimum this reads at all (below that, no magic
    // could be told apart from a coincidence); an MPEG frame sync there
    // is recognized.
    let (_d, p) = with_bytes("sync.bin", &[0xFF, 0xFB, 0x90, 0x00]);
    assert_eq!(detect_container(&p), Some("MP3"));
    let (_d2, p2) = with_bytes("tooshort.bin", &[0xFF, 0xFB]);
    assert_eq!(detect_container(&p2), None);
}

/// The extension mapping and the magic-byte mapping must agree on the
/// names they share, or `analyze_file`'s mismatch check would fire on
/// every file of that type.
#[test]
fn extension_and_magic_use_the_same_names() {
    for (name, expected) in [
        ("a.flac", "FLAC"),
        ("a.wav", "WAV"),
        ("a.aiff", "AIFF"),
        ("a.ogg", "OGG"),
        // Same container as `.ogg`, so it must map to the same canonical
        // name — mapping it to nothing made `flag_container_mismatch`
        // treat every `.opus` as an unrecognized extension.
        ("a.opus", "OGG"),
        ("a.mp3", "MP3"),
        ("a.aac", "AAC"),
        ("a.dsf", "DSF"),
        ("a.dff", "DFF"),
        ("a.caf", "CAF"),
        ("a.m4a", "MP4"),
    ] {
        assert_eq!(ext_canonical(Path::new(name)), Some(expected), "{name}");
    }
    // Case-insensitive, as file systems are in practice.
    assert_eq!(ext_canonical(Path::new("A.FLAC")), Some("FLAC"));
    assert_eq!(ext_canonical(Path::new("noext")), None);
}

/// `.opus` says the codec, not the container: an Ogg stream under that
/// name can hold nothing else, so "OGG" would waste the label on the one
/// thing it doesn't need to say.
#[test]
fn opus_extension_is_labelled_by_its_codec() {
    assert_eq!(format_label(Path::new("a.opus"), None), "Opus");
    assert_eq!(format_label(Path::new("a.opus"), Some("Opus")), "Opus");
    // A generic Ogg keeps the container name — it could be either codec.
    assert_eq!(format_label(Path::new("a.ogg"), Some("Vorbis")), "OGG");
}

/// Symphonia identifies far more codecs than it decodes. The bare
/// "unsupported codec" it returns for Opus reads like a corrupt file, so
/// the one gap we know about is named explicitly — and anything else must
/// keep falling through to Symphonia's own wording rather than being
/// guessed at.
#[test]
fn known_missing_decoders_explain_themselves() {
    let opus = missing_decoder_reason(CODEC_TYPE_OPUS).expect("Opus is a known gap");
    assert!(opus.contains("Opus"), "{opus}");
    assert!(missing_decoder_reason(CODEC_TYPE_FLAC).is_none());
    assert!(missing_decoder_reason(CODEC_TYPE_VORBIS).is_none());
}

#[test]
fn format_label_falls_back_to_the_extension() {
    assert_eq!(format_label(Path::new("a.flac"), None), "FLAC");
    assert_eq!(format_label(Path::new("a.weird"), None), "WEIRD");
    assert_eq!(format_label(Path::new("noext"), None), "?");
}

/// The one extension `format_label` treats specially: an `.m4a`/`.mp4`
/// reads "ALAC/MP4" when the codec is ALAC or unresolved (the old,
/// extension-only behaviour), but must say "AAC/MP4" — not "ALAC/MP4" —
/// once the real codec says AAC, since that's a lossy file, not a
/// lossless one, and this app exists to catch exactly that kind of
/// mislabeling.
#[test]
fn mp4_format_label_follows_the_real_codec() {
    assert_eq!(format_label(Path::new("a.m4a"), None), "ALAC/MP4");
    assert_eq!(format_label(Path::new("a.m4a"), Some("ALAC")), "ALAC/MP4");
    assert_eq!(format_label(Path::new("a.m4a"), Some("AAC")), "AAC/MP4");
    assert_eq!(format_label(Path::new("a.mp4"), Some("AAC")), "AAC/MP4");
}
