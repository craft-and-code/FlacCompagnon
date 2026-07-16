//! DSD (Direct Stream Digital) support: exact container parsing for DSF and
//! DFF files, and a calibrated heuristic that spots DSD files converted from a
//! PCM source ("fake DSD").
//!
//! # Container verification (exact)
//! DSF (Sony) and DFF (Philips DSDIFF) headers are parsed directly — magic
//! bytes, sample rate (2.8224 MHz × 1/2/4 → DSD64/128/256), channel count,
//! sample count, and DST compression flag. This authenticates the *container*.
//!
//! # PCM-source detection (calibrated heuristic)
//! Real DSD content blends smoothly into the sigma-delta noise shaping that
//! rises above ~25 kHz. A DSD file converted from 44.1/48 kHz PCM instead shows
//! a *digital brick wall* at the source's Nyquist (22.05 or 24 kHz): measured on
//! synthetic ground truth (a 2nd-order delta-sigma modulator fed native-band vs
//! 44.1k-band-limited content), the level drop across the boundary is ~3 dB for
//! native DSD and ~50 dB for PCM-sourced. The detector flags a drop ≥ 30 dB.

use std::io::Read;
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

    pub fn duration_secs(&self) -> f64 {
        self.sample_count
            .map(|n| n as f64 / self.sample_rate as f64)
            .unwrap_or(0.0)
    }
}

fn rd_u32_le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}
fn rd_u64_le(b: &[u8]) -> u64 {
    u64::from_le_bytes(b[..8].try_into().unwrap())
}
fn rd_u64_be(b: &[u8]) -> u64 {
    u64::from_be_bytes(b[..8].try_into().unwrap())
}

/// Parse a DSF or DFF header (first bytes of the file decide which).
pub fn parse(path: &Path) -> Result<DsdInfo, AnalysisError> {
    let mut f = std::fs::File::open(path)?;
    let mut head = vec![0u8; 16384]; // headers are near the start in both formats
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
    if head.len() < 28 + 52 {
        return Err(AnalysisError::Decode("truncated DSF header".into()));
    }
    let fmt = &head[28..];
    if &fmt[0..4] != b"fmt " {
        return Err(AnalysisError::Decode("DSF: missing fmt chunk".into()));
    }
    let channels = rd_u32_le(&fmt[24..28]) as usize;
    let sample_rate = rd_u32_le(&fmt[28..32]);
    let bits = rd_u32_le(&fmt[32..36]);
    let sample_count = rd_u64_le(&fmt[36..44]);
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
fn parse_dff(head: &[u8]) -> Result<DsdInfo, AnalysisError> {
    let mut sample_rate = 0u32;
    let mut channels = 0usize;
    let mut dst = false;
    let mut pos = 12 + 4; // after FRM8 header + form type
    while pos + 12 <= head.len() {
        let id = &head[pos..pos + 4];
        let size = rd_u64_be(&head[pos + 4..pos + 12]) as usize;
        let body = pos + 12;
        if id == b"PROP" && body + 4 <= head.len() && &head[body..body + 4] == b"SND " {
            // walk local chunks inside PROP
            let mut p = body + 4;
            let end = (body + size).min(head.len());
            while p + 12 <= end {
                let lid = &head[p..p + 4];
                let lsize = rd_u64_be(&head[p + 4..p + 12]) as usize;
                let lbody = p + 12;
                if lid == b"FS  " && lbody + 4 <= head.len() {
                    sample_rate = u32::from_be_bytes(head[lbody..lbody + 4].try_into().unwrap());
                } else if lid == b"CHNL" && lbody + 2 <= head.len() {
                    channels =
                        u16::from_be_bytes(head[lbody..lbody + 2].try_into().unwrap()) as usize;
                } else if lid == b"CMPR" && lbody + 4 <= head.len() {
                    dst = &head[lbody..lbody + 4] == b"DST ";
                }
                p = lbody + lsize + (lsize & 1); // IFF chunks are 2-byte aligned
            }
        }
        pos = body + size + (size & 1);
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

// --- PCM-source (fake DSD) detection ----------------------------------------

/// Result of the PCM-source check on decoded DSD content.
#[derive(Debug, Clone, Copy)]
pub struct PcmSourceCheck {
    /// Boundary where the brick wall sits (22 050 or 24 000 Hz).
    pub boundary_hz: f64,
    /// Level drop across the boundary in dB.
    pub drop_db: f32,
}

/// Minimum drop across a PCM-Nyquist boundary to call it a brick wall.
/// Calibrated: native ≈ 3 dB, PCM-sourced ≈ 50 dB.
pub const PCM_CLIFF_DB: f32 = 30.0;

/// Inspect the averaged spectrum (from the analyzer, computed on the decoded
/// PCM at `decoded_rate`) for a digital brick wall at a PCM source's Nyquist.
pub fn pcm_source_check(
    spectrum_db: &[f32],
    decoded_rate: u32,
    fft_size: usize,
) -> Option<PcmSourceCheck> {
    if spectrum_db.len() < 16 || decoded_rate == 0 {
        return None;
    }
    let bin_hz = decoded_rate as f64 / fft_size as f64;
    let mean_band = |lo: f64, hi: f64| -> Option<f32> {
        let a = (lo / bin_hz) as usize;
        let b = ((hi / bin_hz) as usize).min(spectrum_db.len() - 1);
        if b <= a {
            return None;
        }
        let vals: Vec<f32> = spectrum_db[a..=b]
            .iter()
            .cloned()
            .filter(|v| v.is_finite())
            .collect();
        if vals.is_empty() {
            None
        } else {
            Some(vals.iter().sum::<f32>() / vals.len() as f32)
        }
    };

    let mut best: Option<PcmSourceCheck> = None;
    for boundary in [22_050.0f64, 24_000.0] {
        let below = mean_band(boundary - 2_000.0, boundary - 200.0);
        let above = mean_band(boundary + 300.0, boundary + 2_000.0);
        if let (Some(lo), Some(hi)) = (below, above) {
            // Require actual content below the boundary (not silence).
            if lo > -80.0 {
                let drop = lo - hi;
                if drop >= PCM_CLIFF_DB && best.map_or(true, |b| drop > b.drop_db) {
                    best = Some(PcmSourceCheck {
                        boundary_hz: boundary,
                        drop_db: drop,
                    });
                }
            }
        }
    }
    best
}

/// Detect the sigma-delta heritage of a DSD master inside hi-res *PCM*
/// (FLAC/WAV at ≥ 96 kHz): genuine PCM recordings decay monotonically into the
/// ultrasonic range, while DSD-converted PCM shows a valley around 22–30 kHz
/// followed by a strong **rising** noise ramp (measured on a real DSD-sourced
/// 192 kHz FLAC: valley ≈ −85 dB, 36–70 kHz ≈ −55 dB → +30 dB rise). A rise of
/// ≥ 15 dB with an audible ramp is flagged. Returns the rise in dB.
pub fn dsd_heritage_check(
    spectrum_db: &[f32],
    sample_rate: u32,
    fft_size: usize,
) -> Option<f32> {
    if sample_rate < 96_000 || spectrum_db.len() < 16 || fft_size == 0 {
        return None;
    }
    let nyq = sample_rate as f64 / 2.0;
    let bin_hz = sample_rate as f64 / fft_size as f64;
    let mean_band = |lo: f64, hi: f64| -> Option<f32> {
        let a = (lo / bin_hz) as usize;
        let b = ((hi / bin_hz) as usize).min(spectrum_db.len() - 1);
        if b <= a + 3 {
            return None;
        }
        let vals: Vec<f32> = spectrum_db[a..=b]
            .iter()
            .cloned()
            .filter(|v| v.is_finite())
            .collect();
        if vals.is_empty() {
            None
        } else {
            Some(vals.iter().sum::<f32>() / vals.len() as f32)
        }
    };
    let valley = mean_band(22_000.0, 30_000.0)?;
    let ramp = mean_band(36_000.0, (0.92 * nyq).min(75_000.0))?;
    let rise = ramp - valley;
    if rise >= 15.0 && ramp > -75.0 {
        Some(rise)
    } else {
        None
    }
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

    #[test]
    fn dsd_heritage_in_hires_pcm_is_detected() {
        // 192 kHz PCM, FFT 8192: mimic the measured DSD-sourced profile —
        // content valley ~-85 dB at 22-30 kHz, noise ramp ~-55 dB at 36-70 kHz.
        let fft = 8192usize;
        let rate = 192_000u32;
        let bin_hz = rate as f64 / fft as f64;
        let mut spec = vec![-85.0f32; fft / 2 + 1];
        for (i, v) in spec.iter_mut().enumerate() {
            let f = i as f64 * bin_hz;
            if f < 20_000.0 {
                *v = -50.0;
            } else if f > 34_000.0 {
                *v = -55.0;
            }
        }
        let rise = dsd_heritage_check(&spec, rate, fft).expect("detected");
        assert!(rise > 15.0, "rise {rise}");
        // Genuine hi-res PCM: monotonic decay, no ultrasonic rise.
        let genuine: Vec<f32> = (0..=fft / 2)
            .map(|i| -40.0 - 60.0 * (i as f32 / (fft / 2) as f32))
            .collect();
        assert!(dsd_heritage_check(&genuine, rate, fft).is_none());
    }

    #[test]
    fn pcm_cliff_is_flagged_and_native_is_not() {
        // Decoded rate 352.8 kHz, FFT 8192 -> Nyquist 176.4 kHz over 4097 bins.
        let fft = 8192usize;
        let rate = 352_800u32;
        let nbins = fft / 2 + 1;
        let bin_hz = rate as f64 / fft as f64;
        // PCM-sourced: content 0 dB up to 22.05 kHz, -55 dB above, noise ramp later.
        let mut fake = vec![-55.0f32; nbins];
        for (i, v) in fake.iter_mut().enumerate() {
            if (i as f64 * bin_hz) < 22_050.0 {
                *v = 0.0;
            }
        }
        let hit = pcm_source_check(&fake, rate, fft).expect("flagged");
        assert!((hit.boundary_hz - 22_050.0).abs() < 1.0);
        assert!(hit.drop_db > 30.0);
        // Native-like: gentle 3 dB step into the noise shaping.
        let mut native = vec![-3.0f32; nbins];
        for (i, v) in native.iter_mut().enumerate() {
            if (i as f64 * bin_hz) < 22_050.0 {
                *v = 0.0;
            }
        }
        assert!(pcm_source_check(&native, rate, fft).is_none());
    }
}
