//! What a file *is* versus what it claims to be.
//!
//! [`detect_container`] reads magic bytes, [`ext_canonical`] reads the
//! extension, and the analyzer compares them — a `.flac` that is really an MP4
//! is worth saying out loud, and it is exactly the kind of thing a re-wrapped
//! lossy file looks like.

use std::path::Path;

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
        Some("ogg" | "oga") => Some("OGG"),
        Some("mp3") => Some("MP3"),
        Some("aac") => Some("AAC"),
        Some("dsf") => Some("DSF"),
        Some("dff") => Some("DFF"),
        _ => None,
    }
}

/// Human-readable container/codec label from the file extension.
///
/// Deliberately not the same mapping as [`ext_canonical`]: this one is shown
/// to the user (so MP4 that holds ALAC reads "ALAC/MP4") and falls back to the
/// uppercased extension for anything unknown, while `ext_canonical` returns a
/// strict canonical name used to compare against the detected container.
pub(super) fn format_label(path: &Path) -> String {
    match lower_ext(path).as_deref() {
        Some("flac") => "FLAC",
        Some("wav" | "wave") => "WAV",
        Some("aif" | "aiff" | "aifc") => "AIFF",
        Some("alac" | "m4a" | "mp4") => "ALAC/MP4",
        Some("caf") => "CAF",
        Some("ogg" | "oga") => "OGG",
        Some("mp3") => "MP3",
        Some("aac") => "AAC",
        Some(other) => return other.to_uppercase(),
        None => "?",
    }
    .to_string()
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

    #[test]
    fn format_label_falls_back_to_the_extension() {
        assert_eq!(format_label(Path::new("a.flac")), "FLAC");
        assert_eq!(format_label(Path::new("a.m4a")), "ALAC/MP4");
        assert_eq!(format_label(Path::new("a.weird")), "WEIRD");
        assert_eq!(format_label(Path::new("noext")), "?");
    }
}
