//! WAV encoding via `hound` — the same crate this crate's own tests already
//! use to synthesize fixtures (see e.g. `tags::mod::tests::synth_wav`), so
//! the call shape here follows an already-proven pattern rather than a fresh
//! guess at the API.
//!
//! Fixed at 16-bit PCM: unlike FLAC, WAV has no lossless-at-any-depth
//! encoder API to lean on here, and 16-bit keeps `hound`'s typed
//! `write_sample::<i16>` call unambiguous. This isn't a hi-res archival
//! format in this app's conversion panel — it exists as a guaranteed-honest
//! PCM copy of a file this app may have flagged as fake-lossless, and 16-bit
//! already exceeds what a genuinely lossy source ever contained.

use std::path::Path;

use super::ConvertError;
use crate::decode::PcmAudio;

const BIT_DEPTH: u32 = 16;

/// Encode `pcm` as 16-bit PCM WAV to `dest`.
pub(super) fn encode(pcm: &PcmAudio, dest: &Path) -> Result<(), ConvertError> {
    let name = || dest.display().to_string();
    let spec = hound::WavSpec {
        channels: pcm.channels as u16,
        sample_rate: pcm.sample_rate,
        bits_per_sample: BIT_DEPTH as u16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer =
        hound::WavWriter::create(dest, spec).map_err(|e| ConvertError::Encode(name(), e.to_string()))?;

    let ints = super::f32_to_ints(&pcm.samples, BIT_DEPTH);
    for s in ints {
        writer
            .write_sample(s as i16)
            .map_err(|e| ConvertError::Encode(name(), e.to_string()))?;
    }
    writer
        .finalize()
        .map_err(|e| ConvertError::Encode(name(), e.to_string()))
}

#[cfg(test)]
#[path = "../../tests/unit/convert/wav.rs"]
mod tests;
