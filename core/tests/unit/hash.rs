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
    assert!(d
        .md5
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    assert!(d
        .crc32
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
}

#[test]
fn a_missing_file_is_an_error_not_a_fingerprint() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(file_digest(&dir.path().join("absent.flac")).is_err());
}
