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
mod tests {
    use super::*;

    /// Ground truth from the specifications, not from this implementation:
    /// RFC 1321 gives MD5("abc"), and CRC-32/ISO-HDLC's check value for
    /// "123456789" is the one every CRC catalogue lists.
    #[test]
    fn known_vectors_from_the_specifications() {
        let dir = tempfile::tempdir().expect("tempdir");

        let abc = dir.path().join("abc.bin");
        std::fs::write(&abc, b"abc").expect("write");
        assert_eq!(
            file_digest(&abc).expect("digest").md5,
            "900150983cd24fb0d6963f7d28e17f72"
        );

        let check = dir.path().join("check.bin");
        std::fs::write(&check, b"123456789").expect("write");
        assert_eq!(file_digest(&check).expect("digest").crc32, "cbf43926");
    }

    /// An empty file has fingerprints, and they are the well-known ones — a
    /// zero-length read must not be mistaken for a failure.
    #[test]
    fn an_empty_file_hashes_to_the_known_empty_values() {
        let dir = tempfile::tempdir().expect("tempdir");
        let empty = dir.path().join("empty.bin");
        std::fs::write(&empty, b"").expect("write");
        let d = file_digest(&empty).expect("digest");
        assert_eq!(d.md5, "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(d.crc32, "00000000");
    }

    /// The chunked loop must produce the same answer as one-shot hashing,
    /// which only a file larger than [`CHUNK`] can show.
    #[test]
    fn chunking_does_not_change_the_result() {
        let dir = tempfile::tempdir().expect("tempdir");
        let big = dir.path().join("big.bin");
        // Deterministic, non-repeating content: a repeating pattern would
        // hide a chunk-boundary mistake that a changing one exposes.
        let mut data = Vec::with_capacity(CHUNK * 2 + 1234);
        let mut x = 0x1234_5678u32;
        while data.len() < CHUNK * 2 + 1234 {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            data.extend_from_slice(&x.to_le_bytes());
        }
        std::fs::write(&big, &data).expect("write");

        let mut md5 = Md5::new();
        md5.update(&data);
        let mut crc = crc32fast::Hasher::new();
        crc.update(&data);

        let d = file_digest(&big).expect("digest");
        assert_eq!(d.md5, format!("{:x}", md5.finalize()));
        assert_eq!(d.crc32, format!("{:08x}", crc.finalize()));
    }

    /// Both fields are fixed-width hex — a CRC printed without padding is how
    /// a checksum silently stops matching the `.sfv` it is compared against.
    #[test]
    fn the_hex_is_padded_to_a_fixed_width() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Content chosen by search below is unnecessary: any file proves the
        // widths, and a leading-zero CRC is covered by the empty-file test.
        let p = dir.path().join("any.bin");
        std::fs::write(&p, b"whatever").expect("write");
        let d = file_digest(&p).expect("digest");
        assert_eq!(d.md5.len(), 32, "{}", d.md5);
        assert_eq!(d.crc32.len(), 8, "{}", d.crc32);
        assert!(d.md5.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
        assert!(d.crc32.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }

    #[test]
    fn a_missing_file_is_an_error_not_a_fingerprint() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(file_digest(&dir.path().join("absent.flac")).is_err());
    }
}
