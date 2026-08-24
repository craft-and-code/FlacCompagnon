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
    std::fs::write(dest, sink.as_slice()).map_err(|e| ConvertError::Io(name(), e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic tone in, decoded back with `claxon` — a different crate
    /// from the one that encoded it — and compared sample-for-sample. FLAC is
    /// lossless by definition, so this must be an exact match, not just a
    /// close one; using an independent decoder is what makes the ground
    /// truth here actually independent of `flacenc`'s own correctness.
    #[test]
    fn round_trips_bit_exact_through_an_independent_decoder() {
        let sample_rate = 44_100u32;
        let channels = 2usize;
        let bits = 16u32;

        // Two channels of a simple, non-trivial waveform — not silence, so a
        // bug that only shows up on nonzero samples wouldn't hide here.
        let frames = 2000;
        let mut samples = Vec::with_capacity(frames * channels);
        for t in 0..frames {
            let phase = t as f32 / sample_rate as f32;
            let l = (phase * 440.0 * std::f32::consts::TAU).sin() * 0.5;
            let r = (phase * 220.0 * std::f32::consts::TAU).sin() * 0.5;
            samples.push(l);
            samples.push(r);
        }
        let expected_ints = f32_to_ints(&samples, bits);

        let pcm = PcmAudio {
            samples,
            sample_rate,
            channels,
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("tone.flac");
        encode(&pcm, &dest, bits, FlacEffort::Balanced).expect("encode");

        let mut reader = claxon::FlacReader::open(&dest).expect("reopen");
        let decoded: Vec<i32> = reader
            .samples()
            .map(|s| s.expect("decode"))
            .collect();

        // A plain `assert_eq!` on a several-thousand-element Vec dumps both
        // sides in full, which is unreadable — this instead reports the
        // length (a truncated/padded tail block would show up here) and, if
        // lengths match, the first differing index with a window either side
        // (a channel swap, an off-by-one, or an isolated rounding
        // difference each look different in that window).
        assert_eq!(
            decoded.len(),
            expected_ints.len(),
            "decoded {} samples, expected {}",
            decoded.len(),
            expected_ints.len()
        );
        if let Some(i) = (0..decoded.len()).find(|&i| decoded[i] != expected_ints[i]) {
            let lo = i.saturating_sub(6);
            let hi = (i + 6).min(decoded.len());
            panic!(
                "first mismatch at index {i} (frame {}, {}): decoded={:?} expected={:?}",
                i / channels,
                if i % channels == 0 { "L" } else { "R" },
                &decoded[lo..hi],
                &expected_ints[lo..hi],
            );
        }
    }
}
