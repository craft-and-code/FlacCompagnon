//! Exact DSF / DFF container parsing.
//!
//! DSF (Sony) is little-endian with a fixed header layout; DFF (Philips
//! DSDIFF) is a big-endian IFF chunk tree. Both are parsed directly from the
//! first bytes of the file — this authenticates the *container* and says
//! nothing about whether its content is genuine DSD (that's [`super::spectral`]).
//!
//! DSF fields are checked against its fixed header; DFF offsets are bounded
//! by each chunk, its parent and the actual file size. A truncated or hostile
//! header must produce an error, never a panic.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::AnalysisError;

/// Base DSD64 bit rate (64 × 44.1 kHz).
pub const DSD64_RATE: u32 = 2_822_400;

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
fn rd_u64_be(b: &[u8], at: usize) -> Option<u64> {
    rd(b, at).map(u64::from_be_bytes)
}

fn truncated(what: &str) -> AnalysisError {
    AnalysisError::Decode(format!("truncated {what} header"))
}

/// Parse a DSF or DFF header (first bytes of the file decide which).
pub fn parse(path: &Path) -> Result<DsdInfo, AnalysisError> {
    let mut file = std::fs::File::open(path)?;
    let magic = read_array::<4>(&mut file)?;
    file.seek(SeekFrom::Start(0))?;
    match &magic {
        b"DSD " => parse_dsf(&read_array::<80>(&mut file)?),
        b"FRM8" => {
            let size = file.metadata()?.len();
            parse_dff_reader(&mut file, size)
        }
        _ => Err(AnalysisError::Decode("not a DSF/DFF file".into())),
    }
}

fn read_array<const N: usize>(reader: &mut impl Read) -> Result<[u8; N], AnalysisError> {
    let mut bytes = [0; N];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
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
    if rd_u64_le(head, 4) != Some(28) || rd_u64_le(head, FMT + 4) != Some(FMT_LEN as u64) {
        return Err(AnalysisError::Decode(
            "DSF: invalid header chunk size".into(),
        ));
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

struct DffChunk {
    id: [u8; 4],
    size: u64,
    end: u64,
    next: u64,
}

/// DSDIFF 1.5 §2.3: sizes exclude the 12-byte header and optional even pad.
/// Bounds use u64 offsets so skipping a large extension never allocates it.
fn dff_chunk(
    reader: &mut (impl Read + Seek),
    at: u64,
    limit: u64,
) -> Result<DffChunk, AnalysisError> {
    let body = at
        .checked_add(12)
        .filter(|end| *end <= limit)
        .ok_or_else(|| truncated("DFF chunk"))?;
    reader.seek(SeekFrom::Start(at))?;
    let header = read_array::<12>(reader)?;
    let id = rd(&header, 0).ok_or_else(|| truncated("DFF chunk"))?;
    let size = rd_u64_be(&header, 4).ok_or_else(|| truncated("DFF chunk"))?;
    let end = body
        .checked_add(size)
        .ok_or_else(|| truncated("DFF chunk"))?;
    let next = end
        .checked_add(size & 1)
        .filter(|end| *end <= limit)
        .ok_or_else(|| truncated("DFF chunk"))?;
    Ok(DffChunk {
        id,
        size,
        end,
        next,
    })
}

/// Walk DFF headers without a fixed scan cutoff. Unrecognized extensions are
/// skipped on disk, while known fields remain bounded by their own payloads.
fn parse_dff_reader(
    reader: &mut (impl Read + Seek),
    file_size: u64,
) -> Result<DsdInfo, AnalysisError> {
    let header = read_array::<16>(reader)?;
    if header.get(..4) != Some(b"FRM8") || header.get(12..16) != Some(b"DSD ") {
        return Err(AnalysisError::Decode("not a DFF file".into()));
    }
    let form_end = rd_u64_be(&header, 4)
        .and_then(|size| size.checked_add(12))
        .filter(|end| (16..=file_size).contains(end))
        .ok_or_else(|| truncated("DFF"))?;
    let mut pos = 16;
    while pos < form_end {
        let chunk = dff_chunk(reader, pos, form_end)?;
        if &chunk.id == b"PROP" && chunk.size >= 4 && read_array::<4>(reader)? == *b"SND " {
            return parse_dff_properties(reader, chunk.end);
        }
        pos = chunk.next;
    }
    Err(AnalysisError::Decode(
        "DFF: missing sound properties".into(),
    ))
}

fn parse_dff_properties(
    reader: &mut (impl Read + Seek),
    end: u64,
) -> Result<DsdInfo, AnalysisError> {
    let mut sample_rate = None;
    let mut channels = None;
    let mut dst = None;
    let mut pos = reader.stream_position()?;
    while pos < end {
        let chunk = dff_chunk(reader, pos, end)?;
        // DSDIFF 1.5 §§3.2.1–3.2.3: mandatory properties are unique.
        match &chunk.id {
            b"FS  " if sample_rate.is_none() && chunk.size == 4 => {
                sample_rate = Some(u32::from_be_bytes(read_array(reader)?));
            }
            b"CHNL" if channels.is_none() && chunk.size >= 2 => {
                let count = u16::from_be_bytes(read_array(reader)?);
                if chunk.size != 2 + u64::from(count) * 4 {
                    return Err(truncated("DFF channel IDs"));
                }
                channels = Some(usize::from(count));
            }
            b"CMPR" if dst.is_none() && chunk.size >= 5 => {
                let codec = read_array::<5>(reader)?;
                let name_len = codec
                    .get(4)
                    .copied()
                    .ok_or_else(|| truncated("DFF compression"))?;
                if chunk.size != 5 + u64::from(name_len) {
                    return Err(truncated("DFF compression name"));
                }
                dst = Some(match codec.get(..4) {
                    Some(b"DSD ") => false,
                    Some(b"DST ") => true,
                    _ => return Err(AnalysisError::Decode("DFF: unsupported compression".into())),
                });
            }
            b"FS  " | b"CHNL" | b"CMPR" => {
                return Err(AnalysisError::Decode(
                    "DFF: invalid or duplicate sound property".into(),
                ));
            }
            _ => {}
        }
        pos = chunk.next;
    }
    let sample_rate = sample_rate.ok_or_else(|| truncated("DFF sample rate"))?;
    let channels = channels.ok_or_else(|| truncated("DFF channels"))?;
    let dst_compressed = dst.ok_or_else(|| truncated("DFF compression"))?;
    validate(sample_rate, channels)?;
    Ok(DsdInfo {
        container: "DFF",
        sample_rate,
        channels,
        sample_count: None,
        multiple: multiple_of(sample_rate),
        dst_compressed,
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
#[path = "../../tests/unit/dsd/container.rs"]
mod tests;
