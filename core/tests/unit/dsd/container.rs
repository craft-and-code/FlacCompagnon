use super::*;

fn parse_dff(bytes: &[u8]) -> Result<DsdInfo, AnalysisError> {
    parse_dff_reader(&mut std::io::Cursor::new(bytes), bytes.len() as u64)
}

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

// DSDIFF 1.5 §§2.3 and 3.2: parent sizes exclude their 12-byte headers;
// channel IDs occupy four bytes per channel and compression names are counted.
fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend((body.len() as u64).to_be_bytes());
    bytes.extend(body);
    if !body.len().is_multiple_of(2) {
        bytes.push(0);
    }
    bytes
}

fn dff_header(properties: &[u8]) -> Vec<u8> {
    let mut form = b"DSD ".to_vec();
    let mut prop = b"SND ".to_vec();
    prop.extend(properties);
    form.extend(chunk(b"PROP", &prop));
    chunk(b"FRM8", &form)
}

fn sound_properties(compression: &[u8; 4]) -> Vec<u8> {
    let mut properties = chunk(b"FS  ", &(2 * DSD64_RATE).to_be_bytes());
    let mut channels = 2u16.to_be_bytes().to_vec();
    channels.extend(b"SLFTSRGT");
    properties.extend(chunk(b"CHNL", &channels));
    let mut codec = compression.to_vec();
    codec.push(0); // empty compression name
    properties.extend(chunk(b"CMPR", &codec));
    properties
}

#[test]
fn parses_dff_header() {
    for (compression, expected_dst) in [(b"DSD ", false), (b"DST ", true)] {
        let header = dff_header(&sound_properties(compression));
        let info = parse_dff(&header).expect("valid header from the format specification");
        assert_eq!(info.container, "DFF");
        assert_eq!(info.multiple, 128);
        assert_eq!(info.channels, 2);
        assert_eq!(info.dst_compressed, expected_dst);
    }
}

#[test]
fn large_valid_dff_property_extensions_do_not_hide_sound_parameters() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("extended.dff");
    // Handcrafted from DSDIFF 1.5 §§2.3, 3.1 and 3.2. Extensions may be
    // unknown and large; their contents must be skipped rather than scanned.
    for before_known_properties in [false, true] {
        let extension = chunk(b"TEST", &vec![0; 32_769]); // includes odd-size padding
        let known = sound_properties(b"DST ");
        let mut properties = b"SND ".to_vec();
        if before_known_properties {
            properties.extend(&extension);
            properties.extend(&known);
        } else {
            properties.extend(&known);
            properties.extend(&extension);
        }
        let mut form = b"DSD ".to_vec();
        form.extend(chunk(b"FVER", &0x0105_0000u32.to_be_bytes()));
        form.extend(chunk(b"JUNK", &vec![0; 20_000]));
        form.extend(chunk(b"PROP", &properties));
        form.extend(chunk(b"DST ", &[]));
        std::fs::write(&path, chunk(b"FRM8", &form)).expect("extended DFF fixture");
        let info = parse(&path).expect("valid extended DFF sound properties");
        assert_eq!(info.sample_rate, 2 * DSD64_RATE);
        assert_eq!(info.channels, 2);
        assert!(info.dst_compressed);
    }
}

#[test]
fn dff_fields_cannot_read_bytes_outside_their_declared_chunk() {
    for (at, size) in [(4, 0u64), (20, 0), (42, 0)] {
        let mut properties = sound_properties(b"DSD ");
        properties[at..at + 8].copy_from_slice(&size.to_be_bytes());
        assert!(
            parse_dff(&dff_header(&properties)).is_err(),
            "field at {at}"
        );
    }
    let mut header = dff_header(&sound_properties(b"DSD "));
    // PROP now ends before the first property's four-byte sample-rate field.
    header[20..28].copy_from_slice(&16u64.to_be_bytes());
    assert!(parse_dff(&header).is_err());
}

#[test]
fn dff_rejects_missing_and_duplicate_compression_properties() {
    let mut properties = sound_properties(b"DST ");
    properties.truncate(38); // FS plus CHNL, without mandatory CMPR.
    assert!(parse_dff(&dff_header(&properties)).is_err());
    let mut properties = sound_properties(b"DSD ");
    properties.extend(chunk(b"CMPR", b"DST \0"));
    assert!(parse_dff(&dff_header(&properties)).is_err());
}

#[test]
fn every_truncated_dff_property_header_is_rejected_without_panicking() {
    let header = dff_header(&sound_properties(b"DSD "));
    for end in 0..header.len() {
        assert!(parse_dff(&header[..end]).is_err(), "{end}-byte prefix");
    }
}

#[test]
fn dsf_rejects_contradictory_chunk_sizes() {
    for (offset, size) in [(4, 0u64), (32, 51), (32, u64::MAX)] {
        let mut header = dsf_header(DSD64_RATE, 2, 100);
        header[offset..offset + 8].copy_from_slice(&size.to_le_bytes());
        assert!(parse_dsf(&header).is_err());
    }
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
