//! What a file *is* versus what it claims to be.
//!
//! [`detect_container`] reads magic bytes, [`ext_canonical`] reads the
//! extension, and the analyzer compares them — a `.flac` that is really an MP4
//! is worth saying out loud, and it is exactly the kind of thing a re-wrapped
//! lossy file looks like.

use std::path::Path;

use symphonia::core::codecs::{
    CodecType, CODEC_TYPE_AAC, CODEC_TYPE_ALAC, CODEC_TYPE_FLAC, CODEC_TYPE_MP1, CODEC_TYPE_MP2,
    CODEC_TYPE_MP3, CODEC_TYPE_OPUS, CODEC_TYPE_PCM_F32BE, CODEC_TYPE_PCM_F32LE,
    CODEC_TYPE_PCM_F64BE, CODEC_TYPE_PCM_F64LE, CODEC_TYPE_PCM_S16BE, CODEC_TYPE_PCM_S16LE,
    CODEC_TYPE_PCM_S24BE, CODEC_TYPE_PCM_S24LE, CODEC_TYPE_PCM_S32BE, CODEC_TYPE_PCM_S32LE,
    CODEC_TYPE_PCM_S8, CODEC_TYPE_PCM_U8, CODEC_TYPE_VORBIS,
};

/// Detect the *real* container from the file's magic bytes, independent of its
/// extension. Returns a canonical short name, or `None` if unrecognized.
pub fn detect_container(path: &Path) -> Option<&'static str> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut b = [0u8; 16];
    let n = f.read(&mut b).ok()?;
    let head = b.get(..n)?;
    if head.len() < 4 {
        return None;
    }
    // Helper: does the header have `tag` at `at`? False when it is too short,
    // so every check below is bounds-safe on a file of any length.
    let at = |pos: usize, tag: &[u8]| head.get(pos..pos + tag.len()) == Some(tag);

    if at(0, b"fLaC") {
        return Some("FLAC");
    }
    if at(0, b"DSD ") {
        return Some("DSF");
    }
    if at(0, b"FRM8") && at(12, b"DSD ") {
        return Some("DFF");
    }
    if (at(0, b"RIFF") || at(0, b"RF64")) && at(8, b"WAVE") {
        return Some("WAV");
    }
    if at(0, b"FORM") && (at(8, b"AIFF") || at(8, b"AIFC")) {
        return Some("AIFF");
    }
    if at(0, b"OggS") {
        return Some("OGG");
    }
    if at(0, b"caff") {
        return Some("CAF");
    }
    if at(4, b"ftyp") {
        return Some("MP4");
    }
    if at(0, b"ID3") {
        return Some("MP3");
    }
    // Frame-sync patterns, checked last: they are only two bytes, so a longer
    // magic that happens to start the same way must win first.
    if let (Some(&0xFF), Some(&b1)) = (head.first(), head.get(1)) {
        if b1 & 0xF6 == 0xF0 {
            return Some("AAC"); // ADTS
        }
        if b1 & 0xE0 == 0xE0 {
            return Some("MP3"); // MPEG-1/2 audio frame sync
        }
    }
    None
}

/// Canonical container name expected from a file's extension.
pub fn ext_canonical(path: &Path) -> Option<&'static str> {
    match lower_ext(path).as_deref() {
        Some("flac") => Some("FLAC"),
        Some("wav" | "wave") => Some("WAV"),
        Some("aif" | "aiff" | "aifc") => Some("AIFF"),
        Some("m4a" | "mp4" | "alac") => Some("MP4"),
        Some("caf") => Some("CAF"),
        // `.opus` is an Ogg container like the other two — the extension only
        // narrows the codec, not the wrapper. It has to be listed here even
        // though `format_label` shows it as "Opus": an extension this function
        // doesn't know returns `None`, and `flag_container_mismatch` treats
        // that as "unrecognized" and overwrites the label with the bare
        // detected container, which would throw the codec name away again.
        Some("ogg" | "oga" | "opus") => Some("OGG"),
        Some("mp3") => Some("MP3"),
        Some("aac") => Some("AAC"),
        Some("dsf") => Some("DSF"),
        Some("dff") => Some("DFF"),
        _ => None,
    }
}

/// Human-readable container/codec label from the file extension, and — for
/// containers that can hold more than one codec — the codec Symphonia
/// actually identified.
///
/// Deliberately not the same mapping as [`ext_canonical`]: this one is shown
/// to the user, and falls back to the uppercased extension for anything
/// unknown, while `ext_canonical` returns a strict canonical name used to
/// compare against the detected container.
///
/// `codec` only changes anything for `.m4a`/`.mp4`/`.alac`: an MP4 container
/// can hold ALAC (lossless) or AAC (lossy), and always assuming ALAC — as an
/// earlier version of this function did — meant an AAC-in-MP4 file (lossy)
/// displayed as "ALAC/MP4", reading as lossless when it plainly is not, in
/// an app whose whole point is catching exactly that kind of
/// mislabeling. `None` (codec unresolved, e.g. a probe failure) falls back
/// to the previous "ALAC/MP4" default, since that's still the more common
/// case for this extension.
pub(super) fn format_label(path: &Path, codec: Option<&str>) -> String {
    match lower_ext(path).as_deref() {
        Some("flac") => "FLAC".to_string(),
        Some("wav" | "wave") => "WAV".to_string(),
        Some("aif" | "aiff" | "aifc") => "AIFF".to_string(),
        Some("alac" | "m4a" | "mp4") => match codec {
            Some("ALAC") | None => "ALAC/MP4".to_string(),
            Some(other) => format!("{other}/MP4"),
        },
        Some("caf") => "CAF".to_string(),
        Some("ogg" | "oga") => "OGG".to_string(),
        // An `.opus` file is an Ogg stream too, but unlike a generic `.ogg` it
        // can only carry Opus — so the label says the codec outright, where
        // "OGG" would leave the one useful fact to the codec column.
        Some("opus") => "Opus".to_string(),
        Some("mp3") => "MP3".to_string(),
        Some("aac") => "AAC".to_string(),
        Some(other) => other.to_uppercase(),
        None => "?".to_string(),
    }
}

/// Human-readable codec name for containers that can hold more than one —
/// an M4A/MP4 might be ALAC or AAC, an OGG might be Vorbis or Opus, and the
/// container name alone can't tell those apart (see
/// [`crate::types::FileAnalysis::codec`] and [`format_label`], above, which
/// both use this). `None` for anything not in this list, which just means
/// the caller falls back to a container-only label; it is never wrong, only
/// sometimes less specific.
pub(super) fn codec_label(codec: CodecType) -> Option<&'static str> {
    Some(match codec {
        CODEC_TYPE_AAC => "AAC",
        CODEC_TYPE_ALAC => "ALAC",
        CODEC_TYPE_FLAC => "FLAC",
        CODEC_TYPE_MP1 => "MP1",
        CODEC_TYPE_MP2 => "MP2",
        CODEC_TYPE_MP3 => "MP3",
        CODEC_TYPE_OPUS => "Opus",
        CODEC_TYPE_VORBIS => "Vorbis",
        CODEC_TYPE_PCM_S8
        | CODEC_TYPE_PCM_U8
        | CODEC_TYPE_PCM_S16LE
        | CODEC_TYPE_PCM_S16BE
        | CODEC_TYPE_PCM_S24LE
        | CODEC_TYPE_PCM_S24BE
        | CODEC_TYPE_PCM_S32LE
        | CODEC_TYPE_PCM_S32BE
        | CODEC_TYPE_PCM_F32LE
        | CODEC_TYPE_PCM_F32BE
        | CODEC_TYPE_PCM_F64LE
        | CODEC_TYPE_PCM_F64BE => "PCM",
        _ => return None,
    })
}

/// Why no decoder exists for `codec`, when the reason is a known gap in
/// Symphonia rather than an unrecognized stream.
///
/// Symphonia probes far more codecs than it can decode: it will happily
/// identify an Opus track in an Ogg stream and then have nothing to hand it
/// to, because `symphonia-codec-opus` is a placeholder that isn't even pulled
/// in by the `all` feature. The raw failure that comes back from
/// `get_codecs().make()` is a bare "unsupported codec", which reads like a
/// corrupt file rather than a missing feature. This turns the handful of cases
/// we know about into something a user can act on.
///
/// Returns `None` for anything else, so a genuinely unreadable stream still
/// reports Symphonia's own error rather than a guess.
pub(super) fn missing_decoder_reason(codec: CodecType) -> Option<&'static str> {
    Some(match codec {
        // Encoding *to* Opus works (that path uses libopus directly), which is
        // why this one is worth naming: the app can write a format it cannot
        // read back, and that asymmetry is otherwise baffling.
        CODEC_TYPE_OPUS => {
            "Opus decoding is not supported yet — Symphonia has no Opus decoder \
             (conversion to Opus does work, it uses libopus directly)"
        }
        _ => return None,
    })
}

fn lower_ext(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
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
}
