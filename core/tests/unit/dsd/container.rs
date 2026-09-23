use super::*;

/// Build a minimal valid DSF header (the exact layout our parser reads).
fn dsf_header(rate: u32, channels: u32, samples: u64) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend(b"DSD ");
    v.extend(28u64.to_le_bytes());
    v.extend(0u64.to_le_bytes()); // total size (unused by parser)
    v.extend(0u64.to_le_bytes()); // metadata ptr
    v.extend(b"fmt ");
    v.extend(52u64.to_le_bytes());
    v.extend(1u32.to_le_bytes()); // version
    v.extend(0u32.to_le_bytes()); // format id
    v.extend(2u32.to_le_bytes()); // channel type
    v.extend(channels.to_le_bytes());
    v.extend(rate.to_le_bytes());
    v.extend(1u32.to_le_bytes()); // bits per sample
    v.extend(samples.to_le_bytes());
    v.extend(4096u32.to_le_bytes());
    v.extend(0u32.to_le_bytes());
    v
}

#[test]
fn parses_dsf_header() {
    let h = dsf_header(DSD64_RATE, 2, 2_822_400 * 60);
    let info = parse_dsf(&h).expect("valid");
    assert_eq!(info.container, "DSF");
    assert_eq!(info.sample_rate, DSD64_RATE);
    assert_eq!(info.channels, 2);
    assert_eq!(info.multiple, 64);
    assert_eq!(info.label(), "DSD64");
    assert!((info.duration_secs() - 60.0).abs() < 1e-9);
}

#[test]
fn rejects_garbage_dsf() {
    let mut h = dsf_header(DSD64_RATE, 2, 100);
    h[28] = b'X'; // corrupt fmt magic
    assert!(parse_dsf(&h).is_err());
    let h2 = dsf_header(1234, 2, 100); // implausible rate
    assert!(parse_dsf(&h2).is_err());
}

/// Every truncation of a valid DSF header must be an error, never a panic:
/// these bytes come from a file the app is meant to distrust.
#[test]
fn truncated_dsf_never_panics() {
    let full = dsf_header(DSD64_RATE, 2, 100);
    for cut in 0..full.len() {
        assert!(
            parse_dsf(&full[..cut]).is_err(),
            "a {cut}-byte DSF header should not parse"
        );
    }
}

#[test]
fn parses_dff_header() {
    // FRM8 + size + "DSD " + PROP("SND " { FS, CHNL })
    let mut v = Vec::new();
    v.extend(b"FRM8");
    v.extend(1000u64.to_be_bytes());
    v.extend(b"DSD ");
    // PROP chunk
    let mut prop = Vec::new();
    prop.extend(b"SND ");
    prop.extend(b"FS  ");
    prop.extend(4u64.to_be_bytes());
    prop.extend((2 * DSD64_RATE).to_be_bytes()); // DSD128
    prop.extend(b"CHNL");
    prop.extend(6u64.to_be_bytes());
    prop.extend(2u16.to_be_bytes());
    prop.extend(b"SLFT"); // channel ids (ignored)
    v.extend(b"PROP");
    v.extend((prop.len() as u64).to_be_bytes());
    v.extend(&prop);
    let info = parse_dff(&v).expect("valid");
    assert_eq!(info.container, "DFF");
    assert_eq!(info.multiple, 128);
    assert_eq!(info.channels, 2);
}

/// A DFF whose chunks declare a size of zero used to leave the walk
/// offset unchanged, looping forever. The parser must terminate on any
/// input, however malformed.
#[test]
fn dff_with_zero_sized_chunks_terminates() {
    let mut v = Vec::new();
    v.extend(b"FRM8");
    v.extend(0u64.to_be_bytes());
    v.extend(b"DSD ");
    let mut prop = Vec::new();
    prop.extend(b"SND ");
    prop.extend(b"FS  ");
    prop.extend(0u64.to_be_bytes()); // zero-size inner chunk
    prop.extend((2 * DSD64_RATE).to_be_bytes());
    v.extend(b"PROP");
    v.extend(0u64.to_be_bytes()); // zero-size outer chunk
    v.extend(&prop);
    // Terminating at all is the assertion; the result is "implausible
    // parameters" because no channel count was ever found.
    assert!(parse_dff(&v).is_err());
}

/// A chunk size large enough to overflow a `usize` add must end the walk
/// rather than wrap around.
#[test]
fn dff_with_overflowing_chunk_size_terminates() {
    let mut v = Vec::new();
    v.extend(b"FRM8");
    v.extend(u64::MAX.to_be_bytes());
    v.extend(b"DSD ");
    v.extend(b"PROP");
    v.extend(u64::MAX.to_be_bytes());
    v.extend(b"SND ");
    v.extend([0u8; 64]);
    assert!(parse_dff(&v).is_err());
}
