//! Strict packet visitation shared by spectrograms and complete FLAC decoding.

use super::probe::{probe, validate_pcm_samples, InterleavedBuf};
use super::{flac, validate_decoded_frames, BasicInfo};
use crate::AnalysisError;
use std::path::Path;

/// Visit native-rate PCM packets without retaining the complete file.
/// FLAC uses its strict claxon path; other formats use the analysis decoder.
/// The returned length counts complete decoded frames, not a duration hint.
pub fn visit_pcm(
    path: &Path,
    sink: &mut dyn FnMut(&[f32]) -> Result<(), AnalysisError>,
    cancelled: &dyn Fn() -> bool,
) -> Result<(BasicInfo, u64), AnalysisError> {
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("flac"))
    {
        let mut reader = flac::open_flac(path)?;
        let stream = reader.streaminfo();
        let info = BasicInfo {
            sample_rate: stream.sample_rate,
            channels: stream.channels as usize,
            bits: Some(stream.bits_per_sample),
            format: "FLAC".into(),
        };
        let frames = visit_flac_reader(&mut reader, sink, cancelled)?;
        return Ok((info, frames));
    }
    let mut source = probe(path, true)?;
    let info = BasicInfo {
        sample_rate: source.sample_rate()?,
        channels: source.channels()?,
        bits: source.params.bits_per_sample,
        format: super::container::format_label(
            path,
            super::container::codec_label(source.params.codec),
        ),
    };
    let declared = source.params.n_frames;
    let mut integer = false;
    let mut buffer = InterleavedBuf::<f32>::default();
    let frames = source.visit_decoded(cancelled, &mut |decoded| {
        integer |= !matches!(
            &decoded,
            symphonia::core::audio::AudioBufferRef::F32(_)
                | symphonia::core::audio::AudioBufferRef::F64(_)
        );
        let samples = buffer.fill(decoded);
        validate_pcm_samples(samples)?;
        sink(samples)
    })?;
    validate_decoded_frames(frames, integer.then_some(declared).flatten())?;
    Ok((info, frames))
}

pub(super) fn visit_flac_reader(
    reader: &mut claxon::FlacReader<std::fs::File>,
    sink: &mut dyn FnMut(&[f32]) -> Result<(), AnalysisError>,
    cancelled: &dyn Fn() -> bool,
) -> Result<u64, AnalysisError> {
    let info = reader.streaminfo();
    let channels = info.channels as usize;
    let scale = 1.0f32 / (1u64 << (info.bits_per_sample - 1)) as f32;
    let mut blocks = reader.blocks();
    let mut buffer = Vec::new();
    let mut samples = Vec::new();
    let mut frames = 0_u64;
    loop {
        if cancelled() {
            return Err(AnalysisError::Decode("cancelled".into()));
        }
        let block = match blocks.read_next_or_eof(buffer) {
            Ok(Some(block)) => block,
            Ok(None) => break,
            Err(error) => return Err(AnalysisError::Decode(format!("flac decode error: {error}"))),
        };
        flac::validate_block_channels(&block, channels)?;
        samples.clear();
        for time in 0..block.duration() as usize {
            for channel in 0..channels {
                samples.push(flac::block_sample(&block, channel, time)? as f32 * scale);
            }
        }
        sink(&samples)?;
        frames = frames
            .checked_add(u64::from(block.duration()))
            .ok_or_else(|| AnalysisError::Decode("audio frame count overflow".into()))?;
        buffer = block.into_buffer();
    }
    validate_decoded_frames(frames, info.samples)?;
    Ok(frames)
}
