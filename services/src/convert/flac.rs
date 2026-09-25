//! FLAC encoding via the pure-Rust `flacenc` crate — no C toolchain, no
//! system library, which is why FLAC is the safest of the four conversion
//! targets to depend on, and why it's the default.

use std::path::Path;

use flacenc::component::BitRepr;
use flacenc::error::Verify;

use super::{f32_to_ints, ConvertError, FlacEffort};
use crate::decode::PcmAudio;

/// Encode `pcm` as FLAC to `dest`, at `bit_depth` bits per sample (16 or 24
/// in practice — see [`super::source_bit_depth`]). `dest`'s parent folder is
/// assumed to already exist ([`super::convert_file`] creates it once for
/// whichever encoder ends up handling the file).
pub(super) fn encode(
    pcm: &PcmAudio,
    dest: &Path,
    bit_depth: u32,
    effort: FlacEffort,
) -> Result<(), ConvertError> {
    let name = || dest.display().to_string();
    let samples = f32_to_ints(&pcm.samples, bit_depth);

    // `flacenc`'s `par` feature (on by default) also defaults `multithread`
    // to `true`. This isn't needed here — files are already encoded in
    // parallel one level up (`commands::batch::parallel_map_ordered`), so
    // enabling per-file parallelism too would only oversubscribe the
    // machine's cores for no benefit — so it's turned off regardless of
    // correctness. (The actual bit-exactness bug this module hit — see
    // `core/Cargo.toml`'s comment on the `flacenc` version pin — turned out
    // to be unrelated to threading: a source shorter than one block got
    // padded to a full block on encode, fixed by moving to flacenc >=0.5.)
    // `Encoder` is `#[non_exhaustive]`, so it can't be built with struct-
    // literal syntax outside its own crate even with `..Default::default()`
    // — the field is set on an already-constructed instance instead.
    let mut config = flacenc::config::Encoder::default();
    config.multithread = false;

    // `FlacEffort` in this app's own terms, translated into the two knobs
    // that actually move the needle in `flacenc`: how far the linear
    // predictor search goes, and whether mid-side stereo is tried. Both are
    // pure search effort — every level produces a bit-identical decode, only
    // the time spent and the resulting size differ.
    //
    // The default is left untouched for `Balanced` on purpose: `flacenc`'s
    // own defaults are the reference point, and re-stating them here would
    // silently freeze today's values if upstream ever tunes them.
    match effort {
        FlacEffort::Fast => {
            config.subframe_coding.qlpc.lpc_order = 4;
            config.stereo_coding.use_midside = false;
        }
        FlacEffort::Balanced => {}
        FlacEffort::Maximum => {
            // 24 rather than FLAC's maximum of 32: past roughly this point
            // the extra orders buy fractions of a percent for a search cost
            // that keeps climbing.
            config.subframe_coding.qlpc.lpc_order = 24;
            config.stereo_coding.use_midside = true;
        }
    }

    let config = config
        .into_verified()
        .map_err(|e| ConvertError::Encode(name(), format!("invalid encoder config: {e:?}")))?;
    let source = flacenc::source::MemSource::from_samples(
        &samples,
        pcm.channels,
        bit_depth as usize,
        pcm.sample_rate as usize,
    );
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|e| ConvertError::Encode(name(), format!("{e:?}")))?;

    let mut sink = flacenc::bitsink::ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|e| ConvertError::Encode(name(), format!("{e:?}")))?;
    let mut bytes = sink.as_slice().to_vec();
    declare_fixed_block_size(&mut bytes)
        .map_err(|e| ConvertError::Encode(name(), e.to_string()))?;
    std::fs::write(dest, &bytes).map_err(|e| ConvertError::Io(name(), e.to_string()))
}

/// Make STREAMINFO's `min_blocksize` match `max_blocksize`.
///
/// # Why this is needed
///
/// `encode_with_fixed_block_size` does exactly what it says — every frame
/// header carries the *fixed* blocking strategy — but the STREAMINFO it
/// writes reports the smallest block that actually occurred, and the final
/// block of a track is almost never full. A 10 900 224-sample file at a
/// 4096-sample block size ends on 768 samples, so STREAMINFO went out saying
/// `min 768 / max 4096`.
///
/// In the FLAC format, `min_blocksize == max_blocksize` is not a coincidence
/// to be recomputed — it *is* the flag for a fixed-block stream. Writing them
/// unequal declares a variable-block stream while every frame header says
/// fixed, and the file contradicts itself. Tolerant decoders (ffmpeg, claxon)
/// read the frame headers and never notice; stricter ones reject the file
/// outright, and one of those is Symphonia — which this app itself uses for
/// the transcoding detector's second decode. The symptom was a FLAC this app
/// had just written, that this app could then not re-read.
///
/// Two bytes, at a fixed offset the format guarantees: STREAMINFO must be the
/// first metadata block, so it starts at 8 (4 magic + 4 block header) and its
/// two 16-bit block sizes are the first fields. Nothing is checksummed over
/// them — STREAMINFO's own MD5 covers the decoded audio, not the header — so
/// rewriting them in place is safe.
fn declare_fixed_block_size(bytes: &mut [u8]) -> Result<(), &'static str> {
    // Indexed through `get`/`get_mut` even though this buffer is ours: the
    // encoder could fail in a way that returns a short buffer, and a panic
    // here would take down a whole conversion batch.
    if bytes.get(..4) != Some(b"fLaC") {
        return Err("encoder produced a stream without a FLAC marker");
    }
    // The first metadata block must be STREAMINFO (block type 0) and 34 bytes.
    if bytes.get(4).map(|b| b & 0x7f) != Some(0) {
        return Err("encoder produced a stream whose first block is not STREAMINFO");
    }
    let Some(max) = bytes.get(10..12) else {
        return Err("encoder produced a truncated STREAMINFO");
    };
    let max = [max[0], max[1]];
    let Some(min) = bytes.get_mut(8..10) else {
        return Err("encoder produced a truncated STREAMINFO");
    };
    min.copy_from_slice(&max);
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/convert/flac.rs"]
mod tests;
