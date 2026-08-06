//! DSD decoding, delegated to ffmpeg.
//!
//! Symphonia has no DSD decoder, and demodulating a 1-bit sigma-delta stream
//! to PCM properly is a filter design problem in its own right — so this shells
//! out to the ffmpeg the user already has (the same one the spectrogram
//! feature needs). No shell is involved: arguments go through `Command::arg`,
//! never a command line.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

use super::DecodeOutcome;
use crate::analyzer::StreamAnalyzer;
use crate::AnalysisError;

/// Read size for ffmpeg's stdout. Large enough that the pipe is rarely the
/// bottleneck, small enough to stay a rounding error against the decoded
/// audio itself.
const PIPE_CHUNK: usize = 1 << 16;

/// Decode a DSD file (DSF/DFF) through `ffmpeg` and stream the PCM into the
/// analyzer.
///
/// ffmpeg converts DSD to float PCM at `dsd_rate / 8` (352.8 kHz for DSD64);
/// `decoded_rate` is that *decoded* rate, and it is what the spectral analysis
/// must use — not the 1-bit rate from the container header.
pub fn decode_and_analyze_dsd(
    ffmpeg: &str,
    path: &Path,
    channels: usize,
    decoded_rate: u32,
) -> Result<DecodeOutcome, AnalysisError> {
    if channels == 0 {
        return Err(AnalysisError::Decode("DSD: zero channels".into()));
    }

    let mut child = Command::new(ffmpeg)
        .arg("-v")
        .arg("error")
        .arg("-i")
        .arg(path)
        .args(["-f", "f32le", "-"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| AnalysisError::Decode(format!("ffmpeg spawn failed: {e}")))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| AnalysisError::Decode("ffmpeg produced no output pipe".into()))?;

    let mut analyzer = StreamAnalyzer::new(channels);
    let mut carry: Vec<u8> = Vec::new();
    let mut buf = vec![0u8; PIPE_CHUNK];
    let mut frame = vec![0f32; channels];
    let frame_bytes = channels * 4;
    let mut frame_count: u64 = 0;

    loop {
        let n = stdout
            .read(&mut buf)
            .map_err(|e| AnalysisError::Decode(format!("ffmpeg read failed: {e}")))?;
        if n == 0 {
            break;
        }
        carry.extend_from_slice(&buf[..n]);
        // Only whole frames can be pushed; a partial one at the end of this
        // read stays in `carry` and is completed by the next one.
        let whole = carry.len() / frame_bytes * frame_bytes;
        for chunk in carry[..whole].chunks_exact(frame_bytes) {
            for (c, s) in frame.iter_mut().enumerate() {
                let o = c * 4;
                *s = f32::from_le_bytes([chunk[o], chunk[o + 1], chunk[o + 2], chunk[o + 3]]);
            }
            analyzer.push_frame(&frame, None);
            frame_count += 1;
        }
        carry.drain(..whole);
    }

    let status = child
        .wait()
        .map_err(|e| AnalysisError::Decode(format!("ffmpeg wait failed: {e}")))?;
    if !status.success() || frame_count == 0 {
        return Err(AnalysisError::Decode(
            "ffmpeg could not decode this DSD file".into(),
        ));
    }

    Ok(DecodeOutcome {
        format: "DSD".to_string(),
        sample_rate: decoded_rate,
        channels,
        declared_bits: None, // 1-bit stream; PCM bit depth does not apply
        duration_secs: frame_count as f64 / decoded_rate.max(1) as f64,
        analyzer,
    })
}
