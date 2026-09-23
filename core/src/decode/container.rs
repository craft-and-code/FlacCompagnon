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
/// unrecognized, while `ext_canonical` returns a strict canonical name used to
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
        CODEC_TYPE_PCM_S8 | CODEC_TYPE_PCM_U8 | CODEC_TYPE_PCM_S16LE | CODEC_TYPE_PCM_S16BE
        | CODEC_TYPE_PCM_S24LE | CODEC_TYPE_PCM_S24BE | CODEC_TYPE_PCM_S32LE
        | CODEC_TYPE_PCM_S32BE | CODEC_TYPE_PCM_F32LE | CODEC_TYPE_PCM_F32BE
        | CODEC_TYPE_PCM_F64LE | CODEC_TYPE_PCM_F64BE => "PCM",
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
#[path = "../../tests/unit/decode/container.rs"]
mod tests;
