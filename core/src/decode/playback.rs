//! Decoding for the preview player rather than for analysis: no analyzer, no
//! integer reconstruction, just interleaved `f32` out.
//!
//! Two shapes, because the player needs both:
//!
//! * [`PcmStreamDecoder`] — progressive. Exposes the sample rate and channel
//!   count from the header alone, then yields one packet at a time, so the
//!   audio device can open and start sounding before the file has finished
//!   decoding. This is the normal path.
//! * [`decode_to_pcm`] — whole file in memory. Only the resampling fallback
//!   needs this (it has to know the whole signal before it can resample it);
//!   a typical track is well under 100 MB decoded.
//!
//! DSD (`.dsf`/`.dff`) is not handled here — Symphonia cannot decode it, and
//! callers check for it before getting this far.

use std::path::Path;

use symphonia::core::codecs::Decoder;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatReader;

use super::probe::{probe, InterleavedBuf};
use crate::AnalysisError;

/// Fully-decoded PCM audio: interleaved `f32` samples in `[-1.0, 1.0]`, at the
/// file's own sample rate and channel count.
pub struct PcmAudio {
    /// Interleaved `f32` samples, `[-1.0, 1.0]`.
    pub samples: Vec<f32>,
    /// Sample rate the file was decoded at, in Hz.
    pub sample_rate: u32,
    /// Channel count the samples are interleaved across.
    pub channels: usize,
}

/// Decode `path` into an in-memory interleaved PCM buffer.
///
/// Built on [`PcmStreamDecoder`] rather than repeating the decode loop: that
/// costs one `Vec` per packet over a hand-rolled version, which is invisible
/// next to decoding the file at all, and it means a fix to the packet loop
/// cannot land in one of the two paths and miss the other.
pub fn decode_to_pcm(path: &Path) -> Result<PcmAudio, AnalysisError> {
    let mut decoder = PcmStreamDecoder::open(path)?;
    let sample_rate = decoder.sample_rate;
    let channels = decoder.channels;

    let mut samples: Vec<f32> = Vec::new();
    while let Some(chunk) = decoder.next_chunk()? {
        samples.extend_from_slice(&chunk);
    }

    if samples.is_empty() {
        return Err(AnalysisError::Decode("no audio data decoded".into()));
    }

    Ok(PcmAudio {
        samples,
        sample_rate,
        channels,
    })
}

/// Progressive decoder: opens the file and exposes its `sample_rate` /
/// `channels` immediately (from the container header, before decoding any
/// audio), then hands out one packet's worth of interleaved `f32` samples at
/// a time via [`next_chunk`](Self::next_chunk).
pub struct PcmStreamDecoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    buf: InterleavedBuf,
    /// Sample rate read from the container header, in Hz.
    pub sample_rate: u32,
    /// Channel count read from the container header.
    pub channels: usize,
}

impl PcmStreamDecoder {
    /// Open `path` and probe its header, without decoding any packets yet.
    pub fn open(path: &Path) -> Result<Self, AnalysisError> {
        let probed = probe(path, true)?;
        let sample_rate = probed.sample_rate()?;
        let channels = probed.channels()?;
        let decoder = probed.make_decoder()?;
        Ok(Self {
            format: probed.format,
            decoder,
            track_id: probed.track_id,
            buf: InterleavedBuf::default(),
            sample_rate,
            channels,
        })
    }

    /// Decode and return the next packet's interleaved `f32` samples, or
    /// `Ok(None)` at end of stream. A corrupt packet is skipped rather than
    /// ending the stream early — one bad packet in the middle of a file
    /// should cost a few milliseconds of audio, not the rest of the track.
    pub fn next_chunk(&mut self) -> Result<Option<Vec<f32>>, AnalysisError> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(None)
                }
                Err(SymError::ResetRequired) => return Ok(None),
                Err(e) => return Err(AnalysisError::Decode(format!("packet error: {e}"))),
            };
            if packet.track_id() != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(decoded) => return Ok(Some(self.buf.fill(decoded).to_vec())),
                Err(SymError::DecodeError(_)) => continue, // skip a corrupt packet
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(None)
                }
                Err(e) => return Err(AnalysisError::Decode(format!("decode error: {e}"))),
            }
        }
    }
}
