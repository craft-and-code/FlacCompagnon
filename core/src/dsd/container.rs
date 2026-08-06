//! Exact DSF / DFF container parsing.
//!
//! DSF (Sony) is little-endian with a fixed header layout; DFF (Philips
//! DSDIFF) is a big-endian IFF chunk tree. Both are parsed directly from the
//! first bytes of the file — this authenticates the *container* and says
//! nothing about whether its content is genuine DSD (that's [`super::spectral`]).
//!
//! Every read is bounds-checked against the buffer actually read from disk:
//! these headers come from files the app exists to be suspicious of, so a
//! truncated or hostile header must produce an error, never a panic.

use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::AnalysisError;

/// Base DSD64 bit rate (64 × 44.1 kHz).
pub const DSD64_RATE: u32 = 2_822_400;

/// How much of the file to read looking for the header. Both formats put
/// everything this parser needs near the start; DFF's chunk walk stops as soon
/// as it has the rate and channel count.
const HEADER_SCAN_BYTES: usize = 16384;

/// Exact information read from a DSF/DFF header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsdInfo {
    /// Container: "DSF" or "DFF".
    pub container: &'static str,
    /// 1-bit sample rate (e.g. 2 822 400 for DSD64).
    pub sample_rate: u32,
    /// Channel count declared by the header.
    pub channels: usize,
    /// Total 1-bit samples per channel, when the header declares it.
    pub sample_count: Option<u64>,
    /// DSD speed grade: 64, 128, 256… (sample_rate / 44100 rounded).
    pub multiple: u32,
    /// `true` for DST-compressed DFF (content analysis unavailable).
    pub dst_compressed: bool,
}

impl DsdInfo {
    /// Display label, e.g. "DSD64".
    pub fn label(&self) -> String {
        format!("DSD{}", self.multiple)
    }

    /// Track length in seconds, derived from the declared sample count and
    /// rate; `0.0` when the header didn't declare a sample count.
    pub fn duration_secs(&self) -> f64 {
        self.sample_count
            .map(|n| n as f64 / self.sample_rate as f64)
            .unwrap_or(0.0)
    }
}

/// `N` bytes at `at`, or `None` if they aren't all there.
///
/// The callers below walk offsets derived from the file's own declared chunk
/// sizes — exactly the input that cannot be trusted to stay inside the buffer —
/// so every read has to answer `None` rather than panic. Note `checked_add`
/// and not `at + N`: such an offset can be enormous, and `at + N` would
/// overflow (a panic in debug) *before* `get` ever got to answer.
fn rd<const N: usize>(b: &[u8], at: usize) -> Option<[u8; N]> {
    b.get(at..at.checked_add(N)?)?.try_into().ok()
}

// Fixed-width integer reads, all built on `rd` so none of them can be the one
// that forgets a bounds check.

fn rd_u32_le(b: &[u8], at: usize) -> Option<u32> {
    rd(b, at).map(u32::from_le_bytes)
}
fn rd_u64_le(b: &[u8], at: usize) -> Option<u64> {
    rd(b, at).map(u64::from_le_bytes)
}
fn rd_u32_be(b: &[u8], at: usize) -> Option<u32> {
    rd(b, at).map(u32::from_be_bytes)
}
fn rd_u64_be(b: &[u8], at: usize) -> Option<u64> {
    rd(b, at).map(u64::from_be_bytes)
}
fn rd_u16_be(b: &[u8], at: usize) -> Option<u16> {
    rd(b, at).map(u16::from_be_bytes)
}

fn truncated(what: &str) -> AnalysisError {
    AnalysisError::Decode(format!("truncated {what} header"))
}

/// Parse a DSF or DFF header (first bytes of the file decide which).
pub fn parse(path: &Path) -> Result<DsdInfo, AnalysisError> {
    let mut f = std::fs::File::open(path)?;
    let mut head = vec![0u8; HEADER_SCAN_BYTES];
    let n = f.read(&mut head)?;
    head.truncate(n);
    if head.len() >= 4 && &head[0..4] == b"DSD " {
        parse_dsf(&head)
    } else if head.len() >= 16 && &head[0..4] == b"FRM8" && &head[12..16] == b"DSD " {
        parse_dff(&head)
    } else {
        Err(AnalysisError::Decode("not a DSF/DFF file".into()))
    }
}

/// DSF (Sony): little-endian. Layout: "DSD " chunk (28 bytes), then "fmt "
/// chunk (52 bytes) holding version, format id, channel type, channel count,
/// sampling frequency, bits per sample, sample count, block size.
fn parse_dsf(head: &[u8]) -> Result<DsdInfo, AnalysisError> {
    const FMT: usize = 28; // "fmt " chunk starts right after the 28-byte DSD chunk
    const FMT_LEN: usize = 52; // the spec fixes the fmt chunk's size

    // The *whole* fmt chunk has to be there. Checking only the fields we read
    // would accept a file that stops at byte 72, since the last of them
    // (sample_count) ends exactly there — but such a file is truncated, and
    // the block size that follows is part of what makes it a valid DSF.
    if head.len() < FMT + FMT_LEN {
        return Err(truncated("DSF"));
    }
    if head.get(FMT..FMT + 4) != Some(b"fmt ") {
        return Err(AnalysisError::Decode("DSF: missing fmt chunk".into()));
    }
    // The per-field reads below are still bounds-checked rather than trusting
    // the length test above: one upfront check that every later offset
    // silently depends on is exactly the pattern that breaks when a field is
    // added.
    let channels = rd_u32_le(head, FMT + 24).ok_or_else(|| truncated("DSF"))? as usize;
    let sample_rate = rd_u32_le(head, FMT + 28).ok_or_else(|| truncated("DSF"))?;
    let bits = rd_u32_le(head, FMT + 32).ok_or_else(|| truncated("DSF"))?;
    let sample_count = rd_u64_le(head, FMT + 36).ok_or_else(|| truncated("DSF"))?;
    if bits != 1 && bits != 8 {
        return Err(AnalysisError::Decode(format!(
            "DSF: unexpected bits per sample {bits}"
        )));
    }
    validate(sample_rate, channels)?;
    Ok(DsdInfo {
        container: "DSF",
        sample_rate,
        channels,
        sample_count: Some(sample_count),
        multiple: multiple_of(sample_rate),
        dst_compressed: false,
    })
}

/// DFF (Philips DSDIFF): big-endian IFF. "FRM8" + size + "DSD ", then chunks;
/// "PROP" (type "SND ") contains "FS  " (rate), "CHNL" (count), "CMPR" (codec).
///
/// Chunk sizes come from the file, so every step forward is a checked add: a
/// header declaring a huge or overflowing size must end the walk, not wrap
/// around into a re-scan of the same bytes.
fn parse_dff(head: &[u8]) -> Result<DsdInfo, AnalysisError> {
    let mut sample_rate = 0u32;
    let mut channels = 0usize;
    let mut dst = false;
    // Annotated because `saturating_add` is called on it below: a bare
    // `12 + 4` stays an ambiguous `{integer}` until something pins it, and a
    // method call is not something that can.
    let mut pos: usize = 12 + 4; // after FRM8 header + form type

    // `saturating_add` in the condition, not just at the assignment below:
    // `pos` is derived from a size the file declared, so it can legitimately
    // reach `usize::MAX` after the saturating step — and `pos + 12` would then
    // overflow here, panicking in debug before the loop ever got a chance to
    // end. Same reasoning for `p` in the inner walk.
    while pos.saturating_add(12) <= head.len() {
        let id = &head[pos..pos + 4];
        let size = rd_u64_be(head, pos + 4).ok_or_else(|| truncated("DFF"))? as usize;
        let body = pos + 12;

        if id == b"PROP" && head.get(body..body + 4) == Some(b"SND ") {
            // Walk the local chunks inside PROP.
            let mut p = body + 4;
            let end = body.saturating_add(size).min(head.len());
            while p.saturating_add(12) <= end {
                let lid = &head[p..p + 4];
                let lsize = rd_u64_be(head, p + 4).ok_or_else(|| truncated("DFF"))? as usize;
                let lbody = p + 12;
                match lid {
                    b"FS  " => sample_rate = rd_u32_be(head, lbody).unwrap_or(sample_rate),
                    b"CHNL" => {
                        channels = rd_u16_be(head, lbody).map_or(channels, |c| c as usize)
                    }
                    b"CMPR" => dst = head.get(lbody..lbody + 4) == Some(b"DST "),
                    _ => {}
                }
                // IFF chunks are 2-byte aligned. A zero-size chunk would leave
                // `p` unchanged and spin forever, so the step is forced to be
                // strictly positive (12 bytes of header at minimum).
                let next = lbody.saturating_add(lsize).saturating_add(lsize & 1);
                if next <= p {
                    break;
                }
                p = next;
            }
        }

        let next = body.saturating_add(size).saturating_add(size & 1);
        if next <= pos {
            break; // same guard at the outer level
        }
        pos = next;
        if sample_rate != 0 && channels != 0 {
            break;
        }
    }

    validate(sample_rate, channels)?;
    Ok(DsdInfo {
        container: "DFF",
        sample_rate,
        channels,
        sample_count: None, // DFF stores a data-chunk byte size, not a count
        multiple: multiple_of(sample_rate),
        dst_compressed: dst,
    })
}

fn validate(sample_rate: u32, channels: usize) -> Result<(), AnalysisError> {
    // Accept DSD64..DSD512 at 44.1k- and 48k-based grids.
    let ok_rate = (2_000_000..=25_000_000).contains(&sample_rate);
    if !ok_rate || channels == 0 || channels > 8 {
        return Err(AnalysisError::Decode(format!(
            "implausible DSD parameters: {sample_rate} Hz, {channels} ch"
        )));
    }
    Ok(())
}

fn multiple_of(sample_rate: u32) -> u32 {
    ((sample_rate as f64 / 44_100.0) as u32).max(1)
}

#[cfg(test)]
mod tests {
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
}
