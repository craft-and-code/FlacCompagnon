//! FLAC MD5 signature status.
//!
//! Every FLAC STREAMINFO block stores an MD5 hash of the *unencoded* audio.
//! FlacCompagnon recomputes it during the single decode pass (see
//! [`crate::decode::decode_and_analyze_flac`]) over the exact original integer
//! samples — interleaved, each as `ceil(bits/8)` little-endian two's-complement
//! bytes, the layout the FLAC reference encoder uses — so a `Match` here is
//! equivalent to a successful `flac -t`.

use serde::{Deserialize, Serialize};

/// Result of inspecting a FLAC file's MD5 signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "detail")]
pub enum FlacMd5Status {
    /// STREAMINFO stored an all-zero MD5: the encoder never wrote a signature.
    NoSignature,
    /// Signature present; not verified (verification disabled in options).
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
