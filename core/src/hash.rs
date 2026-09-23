//! Whole-file fingerprints: MD5 and CRC32 over the bytes on disk.
//!
//! # What these are, and what they are not
//!
//! These hash the **file**, tags and cover art included — not the audio. Two
//! files carrying bit-identical audio but a different `ARTIST` tag get
//! different fingerprints here, and that is the point: this is the identity
//! of the file as an object, the value that `.sfv` checksums, download
//! verification and duplicate hunts all work with.
//!
//! The audio-level counterpart already exists elsewhere and answers a
//! different question: [`crate::decode::FlacMd5Status`] compares the decoded
//! samples against the MD5 a FLAC stores in its own header, which is
//! unchanged by retagging and identical across a FLAC and the WAV decoded
//! from it. Neither replaces the other, and confusing the two is easy enough
//! that the field names say `file_` out loud.
//!
//! # Why both, and why one pass
//!
//! CRC32 is what the `.sfv` files shipped with lossless releases use, and
//! what most download tools report; MD5 is what everything else uses. They
//! are wanted by different people for the same file, and computing them
//! separately would read the whole file twice for no reason — a 90 MB track
//! is not free to read, and a folder holds hundreds of them. One read feeds
//! both.
//!
//! CRC32 is a *checksum*, not a cryptographic hash: it detects accidental
//! corruption, and collisions can be constructed deliberately. It is here
//! because release conventions use it, not as evidence of anything.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use md5::{Digest, Md5};

/// How much is read per iteration. Large enough that the syscall overhead
/// disappears against the hashing, small enough to stay out of the way of the
/// analysis buffers running alongside it.
const CHUNK: usize = 1 << 20; // 1 MiB

/// A file's two fingerprints, as lowercase hex.
///
/// Strings rather than the raw values because every consumer — the table, the
/// CSV, the JSON report — wants the text, and formatting it once here keeps
/// the three from disagreeing about padding or case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDigest {
    /// MD5 of the file's bytes, 32 lowercase hex characters.
    pub md5: String,
    /// CRC32 (IEEE, the `.sfv` variant) of the file's bytes, 8 lowercase hex
    /// characters, zero-padded — an unpadded CRC is the classic way two
    /// tools disagree about the same file.
    pub crc32: String,
}

/// Read `path` once and return both fingerprints.
///
/// Errors are the caller's to interpret: an unreadable file still deserves a
/// row in the table, just without these two columns filled in.
pub fn file_digest(path: &Path) -> std::io::Result<FileDigest> {
    let mut file = File::open(path)?;
    let mut md5 = Md5::new();
    let mut crc = crc32fast::Hasher::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        // `get` rather than indexing: `read` promises `n <= buf.len()`, but
        // this crate does not take a promise as a reason to allow a panic on
        // a path that touches user files.
        let Some(chunk) = buf.get(..n) else { break };
        md5.update(chunk);
        crc.update(chunk);
    }
    Ok(FileDigest {
        md5: format!("{:x}", md5.finalize()),
        crc32: format!("{:08x}", crc.finalize()),
    })
}

#[cfg(test)]
#[path = "../tests/unit/hash.rs"]
mod tests;
