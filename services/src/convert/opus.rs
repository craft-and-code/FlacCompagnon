//! Opus encoding: `audiopus` (vendors `libopus` via `audiopus_sys`, no
//! system library needed) produces raw Opus packets; this module also mixes
//! them into an Ogg container by hand (RFC 7845, "Ogg Opus"), using the
//! generic `ogg` crate's page writer — `audiopus` only speaks the codec, not
//! the container a `.opus` file actually needs in order to be playable at
//! all.
//!
//! Opus only accepts four fixed sample rates (8/12/16/24/48 kHz); a source at
//! any other rate (44.1 kHz being the common case for music) is resampled to
//! 48 kHz first with [`super::resample_linear`] — the same plain linear
//! interpolator [`super::mp3`] also uses, since neither format's own lossy
//! coding needs a mastering-grade resampler ahead of it (see that function's
//! own doc comment for the full reasoning).

use std::path::Path;

use audiopus::coder::Encoder;
use audiopus::{Application, Bitrate, Channels, SampleRate};
use ogg::writing::{PacketWriteEndInfo, PacketWriter};

use super::ConvertError;
use crate::decode::PcmAudio;

const OPUS_SAMPLE_RATE: u32 = 48_000;
/// 20ms frames — the size most Opus encoders default to, and a reasonable
/// balance of overhead vs. flexibility for a file being written to disk
/// rather than streamed (there's no real-time latency constraint either
/// way).
const FRAME_MS: u32 = 20;
const FRAME_SAMPLES_PER_CHANNEL: usize = (OPUS_SAMPLE_RATE * FRAME_MS / 1000) as usize;
/// Opus's own documented worst case for a packet at this frame size.
const MAX_PACKET_BYTES: usize = 4000;
/// Every page in one logical Ogg stream shares one serial number; this file
/// never holds more than one stream, so a fixed constant is enough.
const OGG_SERIAL: u32 = 1;

/// Encode `pcm` as Ogg Opus to `dest`, at `bitrate_kbps`.
pub(super) fn encode(pcm: &PcmAudio, dest: &Path, bitrate_kbps: u32) -> Result<(), ConvertError> {
    let name = || dest.display().to_string();

    let channels_enum = match pcm.channels {
        1 => Channels::Mono,
        2 => Channels::Stereo,
        n => {
            return Err(ConvertError::Unsupported(
                name(),
                format!("Opus only supports mono or stereo here (got {n} channels)"),
            ))
        }
    };

    let resampled = if pcm.sample_rate == OPUS_SAMPLE_RATE {
        pcm.samples.clone()
    } else {
        super::resample_linear(
            &pcm.samples,
            pcm.channels,
            pcm.sample_rate,
            OPUS_SAMPLE_RATE,
        )
    };

    let mut encoder = Encoder::new(SampleRate::Hz48000, channels_enum, Application::Audio)
        .map_err(|e| ConvertError::Encode(name(), e.to_string()))?;
    encoder
        .set_bitrate(Bitrate::BitsPerSecond(
            bitrate_kbps.saturating_mul(1000) as i32
        ))
        .map_err(|e| ConvertError::Encode(name(), e.to_string()))?;
    // Opus's own algorithmic delay — folded into the granule position so a
    // decoder can compensate for it, per RFC 7845 §4.
    let pre_skip = encoder.lookahead().unwrap_or(0).min(u16::MAX as u32) as u16;

    let file = std::fs::File::create(dest).map_err(|e| ConvertError::Io(name(), e.to_string()))?;
    let mut writer = PacketWriter::new(file);
    write_header_pages(&mut writer, pcm.channels as u8, pre_skip, pcm.sample_rate)
        .map_err(|e| ConvertError::Io(name(), e.to_string()))?;

    let frame_len = FRAME_SAMPLES_PER_CHANNEL * pcm.channels;
    let mut out_buf = vec![0u8; MAX_PACKET_BYTES];
    let mut samples_encoded: u64 = pre_skip as u64;
    // At least one frame even for a near-empty source, so the stream always
    // gets its `EndStream` packet.
    let total_frames = resampled.len().div_ceil(frame_len.max(1)).max(1);

    for frame_index in 0..total_frames {
        let start = frame_index * frame_len;
        let end = (start + frame_len).min(resampled.len());
        let mut frame = resampled.get(start..end).unwrap_or(&[]).to_vec();
        // Pads only the final, partial frame with silence — Opus frames must
        // all be the same fixed length; a player trims this back out using
        // the stream's final granule position, which stays at the *true*
        // sample count below rather than following this padding.
        frame.resize(frame_len, 0.0);

        let n = encoder
            .encode_float(&frame, &mut out_buf)
            .map_err(|e| ConvertError::Encode(name(), e.to_string()))?;

        let real_samples_this_frame = end.saturating_sub(start) / pcm.channels.max(1);
        samples_encoded += real_samples_this_frame as u64;

        let is_last = frame_index + 1 == total_frames;
        let end_info = if is_last {
            PacketWriteEndInfo::EndStream
        } else {
            PacketWriteEndInfo::NormalPacket
        };
        writer
            .write_packet(out_buf[..n].to_vec(), OGG_SERIAL, end_info, samples_encoded)
            .map_err(|e| ConvertError::Io(name(), e.to_string()))?;
    }

    Ok(())
}

/// Writes the two mandatory header pages an Ogg Opus stream opens with (RFC
/// 7845 §5.1/§5.2): `OpusHead` (channel count, pre-skip, the source's own
/// sample rate as an informational field only — decoders always run at
/// 48kHz internally) and `OpusTags` (just a vendor string; no comments —
/// track tags live in the file's own tag system, not duplicated into the
/// Opus stream itself). Each gets its own page per spec
/// (`PacketWriteEndInfo::EndPage`).
fn write_header_pages(
    writer: &mut PacketWriter<std::fs::File>,
    channels: u8,
    pre_skip: u16,
    input_sample_rate: u32,
) -> std::io::Result<()> {
    let mut head = Vec::with_capacity(19);
    head.extend_from_slice(b"OpusHead");
    head.push(1); // version
    head.push(channels);
    head.extend_from_slice(&pre_skip.to_le_bytes());
    head.extend_from_slice(&input_sample_rate.to_le_bytes());
    head.extend_from_slice(&0i16.to_le_bytes()); // output gain
    head.push(0); // channel mapping family 0: mono/stereo, no mapping table
    writer.write_packet(head, OGG_SERIAL, PacketWriteEndInfo::EndPage, 0)?;

    let vendor = b"FlacCompagnon";
    let mut tags = Vec::with_capacity(8 + 4 + vendor.len() + 4);
    tags.extend_from_slice(b"OpusTags");
    tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    tags.extend_from_slice(vendor);
    tags.extend_from_slice(&0u32.to_le_bytes()); // zero user comments
    writer.write_packet(tags, OGG_SERIAL, PacketWriteEndInfo::EndPage, 0)
}

#[cfg(test)]
#[path = "../../tests/unit/convert/opus.rs"]
mod tests;
