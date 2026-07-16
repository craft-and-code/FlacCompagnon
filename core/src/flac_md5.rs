//! Native FLAC MD5 signature handling.
//!
//! Every FLAC STREAMINFO block stores an MD5 hash of the *unencoded* audio.
//! `flac -t` recomputes it to prove the file decodes to exactly the original
//! samples. We do the same thing natively with `claxon` + `md-5`, so no external
//! `flac` binary is required.
//!
//! The MD5 is computed over the interleaved samples, each written as a
//! little-endian signed integer using `ceil(bits_per_sample / 8)` bytes — the
//! exact layout the FLAC reference encoder uses.

use std::path::Path;

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};

/// Result of inspecting a FLAC file's MD5 signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "detail")]
pub enum FlacMd5Status {
    /// STREAMINFO stored an all-zero MD5: the encoder never wrote a signature.
    NoSignature,
    /// Signature present; not verified (fast mode — STREAMINFO only).
    Present,
    /// Signature present and the decoded audio matches it. File is intact.
    Match,
    /// Signature present but the decoded audio does NOT match: corruption or
    /// a non-conforming encoder.
    Mismatch,
    /// The file could not be read/decoded to check.
    Error(String),
}

impl FlacMd5Status {
    /// Short, log-friendly label.
    pub fn label(&self) -> String {
        match self {
            FlacMd5Status::NoSignature => "No MD5 signature".to_string(),
            FlacMd5Status::Present => "MD5 present (unverified)".to_string(),
            FlacMd5Status::Match => "MD5 OK".to_string(),
            FlacMd5Status::Mismatch => "MD5 MISMATCH".to_string(),
            FlacMd5Status::Error(e) => format!("MD5 check error: {e}"),
        }
    }
}

/// Inspect (and optionally verify) the MD5 signature of a FLAC file.
///
/// When `verify` is false only STREAMINFO is read (instant); when true the file
/// is fully decoded and the hash recomputed (equivalent to `flac -t`).
pub fn check_flac_md5(path: &Path, verify: bool) -> FlacMd5Status {
    let mut reader = match claxon::FlacReader::open(path) {
        Ok(r) => r,
        Err(e) => return FlacMd5Status::Error(e.to_string()),
    };

    let info = reader.streaminfo();
    let stored = info.md5sum;
    if stored == [0u8; 16] {
        return FlacMd5Status::NoSignature;
    }
    if !verify {
        return FlacMd5Status::Present;
    }

    let bits = info.bits_per_sample;
    let bytes_per_sample = ((bits + 7) / 8) as usize;

    let mut hasher = Md5::new();
    for sample in reader.samples() {
        let s = match sample {
            Ok(v) => v,
            Err(e) => return FlacMd5Status::Error(e.to_string()),
        };
        let le = (s as u32).to_le_bytes();
        hasher.update(&le[..bytes_per_sample]);
    }
    let digest = hasher.finalize();

    if digest.as_slice() == stored.as_slice() {
        FlacMd5Status::Match
    } else {
        FlacMd5Status::Mismatch
    }
}
