use super::*;

#[test]
fn accepts_a_canonical_mbid() {
    assert!(is_valid_mbid("c1a1c5f0-6b1e-4f3a-9b2d-1a2b3c4d5e6f"));
    // Case-insensitive: MusicBrainz emits lowercase, tags may not.
    assert!(is_valid_mbid("C1A1C5F0-6B1E-4F3A-9B2D-1A2B3C4D5E6F"));
}

#[test]
fn rejects_ids_that_could_steer_the_request_elsewhere() {
    // The id is interpolated into a URL path and can come from an
    // untrusted file's tags — these must never reach the network.
    assert!(!is_valid_mbid("../../etc/passwd"));
    assert!(!is_valid_mbid("c1a1c5f0-6b1e-4f3a-9b2d-1a2b3c4d5e6f/../x"));
    assert!(!is_valid_mbid("c1a1c5f0-6b1e-4f3a-9b2d-1a2b3c4d5e6f?inc=x"));
    assert!(!is_valid_mbid("c1a1c5f0-6b1e-4f3a-9b2d-1a2b3c4d5e6f#f"));
    assert!(!is_valid_mbid(""));
}

#[test]
fn rejects_malformed_mbids() {
    assert!(!is_valid_mbid("not-a-uuid"));
    // Right length, wrong separator position.
    assert!(!is_valid_mbid("c1a1c5f0x6b1e-4f3a-9b2d-1a2b3c4d5e6f"));
    // Right shape, non-hex character.
    assert!(!is_valid_mbid("g1a1c5f0-6b1e-4f3a-9b2d-1a2b3c4d5e6f"));
    // Too short / too long.
    assert!(!is_valid_mbid("c1a1c5f0-6b1e-4f3a-9b2d-1a2b3c4d5e6"));
    assert!(!is_valid_mbid("c1a1c5f0-6b1e-4f3a-9b2d-1a2b3c4d5e6ff"));
}

/// A 36-byte id whose bytes are multi-byte characters must be rejected on
/// shape, not indexed into — `is_valid_mbid` works on bytes, so this also
/// pins that it never splits a character.
#[test]
fn rejects_non_ascii_of_the_right_byte_length() {
    let id: String = "é".repeat(18); // 36 bytes, 18 chars
    assert_eq!(id.len(), 36);
    assert!(!is_valid_mbid(&id));
}
