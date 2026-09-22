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
mod tests {
    use super::*;

    /// The bug that made a FLAC this app wrote unreadable by this app.
    ///
    /// Ground truth is the format specification, not the encoder: a stream
    /// whose frames use the fixed blocking strategy must report
    /// `min_blocksize == max_blocksize`. The length here is deliberately not
    /// a multiple of the block size, which is the only case that triggers it
    /// — the final short block is what `flacenc` reported as the minimum.
    #[test]
    fn streaminfo_declares_a_fixed_block_stream_even_with_a_short_final_block() {
        let channels = 2usize;
        let bits = 16u32;
        // Two full 4096-sample blocks plus a deliberately partial third.
        let frames = 4096 * 2 + 501;
        let mut samples = Vec::with_capacity(frames * channels);
        for t in 0..frames {
            let phase = t as f32 / 44_100.0;
            samples.push((phase * 440.0 * std::f32::consts::TAU).sin() * 0.5);
            samples.push((phase * 220.0 * std::f32::consts::TAU).sin() * 0.5);
        }
        let expected_ints = f32_to_ints(&samples, bits);
        let pcm = PcmAudio {
            samples,
            sample_rate: 44_100,
            channels,
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("partial-tail.flac");
        encode(&pcm, &dest, bits, FlacEffort::Balanced).expect("encode");

        let bytes = std::fs::read(&dest).expect("read back");
        assert_eq!(&bytes[..4], b"fLaC");
        let min = u16::from_be_bytes([bytes[8], bytes[9]]);
        let max = u16::from_be_bytes([bytes[10], bytes[11]]);
        assert_eq!(
            min, max,
            "STREAMINFO declares a variable-block stream ({min}/{max}) while the \
             frames use the fixed strategy — strict decoders reject this"
        );

        // And the patch must not have broken the audio: same independent
        // decoder, same bit-exact expectation as the round-trip test.
        let mut reader = claxon::FlacReader::open(&dest).expect("reopen");
        let decoded: Vec<i32> = reader.samples().map(|s| s.expect("decode")).collect();
        assert_eq!(decoded.len(), expected_ints.len());
        assert!(
            decoded == expected_ints,
            "patching STREAMINFO changed the decoded audio"
        );

        // The symptom this is all named after: a file this app wrote, that
        // the transcoding detector then could not decode a second time.
        let pcm = crate::decode::decode_to_pcm(&dest).expect("the app must be able to re-read what it just wrote");
        assert_eq!(pcm.sample_rate, 44_100);
        assert_eq!(pcm.channels, channels);
        assert_eq!(pcm.samples.len(), frames * channels);
    }

    /// The patch refuses anything that is not the layout it expects, rather
    /// than rewriting two arbitrary bytes of it.
    #[test]
    fn the_streaminfo_patch_refuses_a_stream_it_does_not_recognise() {
        assert!(declare_fixed_block_size(&mut []).is_err(), "empty");
        assert!(declare_fixed_block_size(&mut b"not a flac file at all".to_vec()).is_err());
        // Right marker, wrong first block type (4 = VORBIS_COMMENT).
        let mut wrong = b"fLaC".to_vec();
        wrong.push(4);
        wrong.extend_from_slice(&[0; 40]);
        assert!(declare_fixed_block_size(&mut wrong).is_err(), "wrong block");
        // Right marker and block type, truncated before the block sizes.
        let mut short = b"fLaC".to_vec();
        short.extend_from_slice(&[0, 0, 0, 34, 1, 2]);
        assert!(declare_fixed_block_size(&mut short).is_err(), "truncated");
    }

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
