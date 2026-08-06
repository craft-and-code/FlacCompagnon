//! The generic decode-and-analyze path: any format Symphonia supports, fed
//! frame by frame into a [`StreamAnalyzer`].
//!
//! FLAC does not come through here — it has a fused path ([`super::flac`])
//! that also computes the STREAMINFO MD5 in the same pass — but it is the
//! fallback when that path fails on a malformed FLAC.

use std::path::Path;

use symphonia::core::audio::AudioBufferRef;
use symphonia::core::errors::Error as SymError;

use super::container::format_label;
use super::probe::{probe, InterleavedBuf};
use super::DecodeOutcome;
use crate::analyzer::StreamAnalyzer;
use crate::AnalysisError;

/// Decode `path` and run streaming analysis over its samples.
pub fn decode_and_analyze(path: &Path) -> Result<DecodeOutcome, AnalysisError> {
    let mut probed = probe(path, true)?;
    let sample_rate = probed.sample_rate()?;
    let channels = probed.channels()?;
    let declared_bits = probed.params.bits_per_sample;
    let declared_duration = probed
        .params
        .n_frames
        .map(|n| n as f64 / sample_rate as f64)
        .unwrap_or(0.0);
    let track_id = probed.track_id;
    let mut decoder = probed.make_decoder()?;

    // Integer reconstruction scale (full-scale = 2^(bits-1)).
    let int_scale: Option<f32> = declared_bits.map(|b| 2f32.powi(b as i32 - 1));

    let mut analyzer = StreamAnalyzer::new(channels);
    let mut buf = InterleavedBuf::default();
    let mut int_packet: Vec<i32> = Vec::new();
    let mut frame_count: u64 = 0;

    loop {
        let packet = match probed.format.next_packet() {
            Ok(p) => p,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::ResetRequired) => break,
            Err(e) => return Err(AnalysisError::Decode(format!("packet error: {e}"))),
        };
        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Float sources carry no meaningful integer bit depth.
                let is_int = !matches!(&decoded, AudioBufferRef::F32(_) | AudioBufferRef::F64(_));
                let f32_samples = buf.fill(decoded);

                // Reconstruct native integers for the whole packet, once, into
                // a buffer reused across packets rather than a fresh Vec each
                // time (this runs per packet for the length of the file).
                int_packet.clear();
                if let (true, Some(scale)) = (is_int, int_scale) {
                    int_packet.extend(f32_samples.iter().map(|&s| (s * scale).round() as i32));
                }
                let ints_ready = !int_packet.is_empty();

                let ch = channels.max(1);
                let n_frames = f32_samples.len() / ch;
                for f in 0..n_frames {
                    let base = f * ch;
                    let frame = &f32_samples[base..base + ch];
                    let ints = ints_ready.then(|| &int_packet[base..base + ch]);
                    analyzer.push_frame(frame, ints);
                }
                frame_count += n_frames as u64;
            }
            Err(SymError::DecodeError(_)) => continue, // skip a corrupt packet
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(AnalysisError::Decode(format!("decode error: {e}"))),
        }
    }

    // Prefer the container's declared length; fall back to what we counted
    // when it doesn't declare one (common for streamed/edited files).
    let duration_secs = if declared_duration > 0.0 {
        declared_duration
    } else {
        frame_count as f64 / sample_rate as f64
    };

    Ok(DecodeOutcome {
        format: format_label(path),
        sample_rate,
        channels,
        declared_bits,
        duration_secs,
        analyzer,
    })
}
