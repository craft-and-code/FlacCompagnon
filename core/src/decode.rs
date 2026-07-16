//! Multi-format decoding via Symphonia, streamed into a [`StreamAnalyzer`].
//!
//! Every packet is exposed as a normalized `f32` interleaved buffer used for
//! spectral / clipping / stereo analysis. For integer PCM sources the exact
//! integer sample values are reconstructed from those floats (the conversion is
//! lossless for <= 24-bit content, since an `f32` mantissa represents every
//! integer up to 2^24 exactly) and fed to the bit-depth estimator.

use std::fs::File;
use std::path::Path;

use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::analyzer::StreamAnalyzer;
use crate::flac_md5::FlacMd5Status;
use crate::AnalysisError;

/// Result of decoding a file: metadata plus a fully-fed analyzer ready for
/// [`StreamAnalyzer::finish`].
pub struct DecodeOutcome {
    pub format: String,
    pub sample_rate: u32,
    pub channels: usize,
    pub declared_bits: Option<u32>,
    pub duration_secs: f64,
    pub analyzer: StreamAnalyzer,
}

/// Decode `path` and run streaming analysis over its samples.
pub fn decode_and_analyze(path: &Path) -> Result<DecodeOutcome, AnalysisError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .map_err(|e| AnalysisError::Decode(format!("probe failed: {e}")))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| AnalysisError::Decode("no default track".into()))?;
    let track_id = track.id;
    let params = track.codec_params.clone();

    let sample_rate = params
        .sample_rate
        .ok_or_else(|| AnalysisError::Decode("unknown sample rate".into()))?;
    let channels = params
        .channels
        .map(|c| c.count())
        .ok_or_else(|| AnalysisError::Decode("unknown channel layout".into()))?;
    let declared_bits = params.bits_per_sample;
    let duration_secs = params
        .n_frames
        .map(|n| n as f64 / sample_rate as f64)
        .unwrap_or(0.0);

    let format_name = format_label(path);
    // Integer reconstruction scale (full-scale = 2^(bits-1)).
    let int_scale: Option<f32> = declared_bits.map(|b| 2f32.powi(b as i32 - 1));

    let mut decoder = symphonia::default::get_codecs()
        .make(&params, &DecoderOptions::default())
        .map_err(|e| AnalysisError::Decode(format!("no decoder: {e}")))?;

    let mut analyzer = StreamAnalyzer::new(channels);
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut buf_frames: u64 = 0; // frames the reusable buffer was sized for
    let mut frame_count: u64 = 0;

    loop {
        let packet = match format.next_packet() {
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
                let is_int =
                    !matches!(&decoded, AudioBufferRef::F32(_) | AudioBufferRef::F64(_));

                // Size (and grow, if a later packet is larger — block sizes can
                // vary within a stream) the reusable f32 buffer.
                let needed = decoded.capacity() as u64;
                if sample_buf.is_none() || needed > buf_frames {
                    let spec = *decoded.spec();
                    sample_buf = Some(SampleBuffer::<f32>::new(needed, spec));
                    buf_frames = needed;
                }
                let sbuf = sample_buf.as_mut().unwrap();
                sbuf.copy_interleaved_ref(decoded);
                let f32_samples = sbuf.samples();

                // Reconstruct native integers for the whole packet, once.
                let int_packet: Option<Vec<i32>> = match (is_int, int_scale) {
                    (true, Some(scale)) => Some(
                        f32_samples
                            .iter()
                            .map(|&s| (s * scale).round() as i32)
                            .collect(),
                    ),
                    _ => None,
                };

                let ch = channels.max(1);
                let n_frames = f32_samples.len() / ch;
                for f in 0..n_frames {
                    let base = f * ch;
                    let frame = &f32_samples[base..base + ch];
                    let ints = int_packet.as_ref().map(|v| &v[base..base + ch]);
                    analyzer.push_frame(frame, ints);
                }
                frame_count += n_frames as u64;
            }
            Err(SymError::DecodeError(_)) => continue, // skip a corrupt packet
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(AnalysisError::Decode(format!("decode error: {e}"))),
        }
    }

    let duration_secs = if duration_secs > 0.0 {
        duration_secs
    } else {
        frame_count as f64 / sample_rate as f64
    };

    Ok(DecodeOutcome {
        format: format_name,
        sample_rate,
        channels,
        declared_bits,
        duration_secs,
        analyzer,
    })
}

/// Fused single-pass FLAC decode: feeds the streaming analyzer AND computes the
/// STREAMINFO MD5 over the exact original integer samples, in one pass.
///
/// Correctness guarantees:
/// * The MD5 is hashed from claxon's raw decoded integers — no float
///   round-trip — using the exact layout mandated by the FLAC spec:
///   interleaved samples, each written as `ceil(bits/8)` little-endian
///   two's-complement bytes. Bit-identical to what `flac -t` verifies.
/// * The analyzer receives `s / 2^(bits-1)` as `f32`, which is exact for
///   FLAC's ≤ 24-bit integers (every such integer fits in the f32 mantissa),
///   plus the raw integers for effective-bit-depth analysis.
///
/// When `verify_md5` is false the hash comparison is skipped (the decode cost
/// is the same either way since analysis needs every sample).
pub fn decode_and_analyze_flac(
    path: &Path,
    verify_md5: bool,
) -> Result<(DecodeOutcome, FlacMd5Status), AnalysisError> {
    use md5::{Digest, Md5};

    let mut reader = claxon::FlacReader::open(path)
        .map_err(|e| AnalysisError::Decode(format!("flac open failed: {e}")))?;
    let info = reader.streaminfo();
    let sample_rate = info.sample_rate;
    let channels = info.channels as usize;
    let bits = info.bits_per_sample;
    if sample_rate == 0 || channels == 0 || bits == 0 || bits > 32 {
        return Err(AnalysisError::Decode("invalid FLAC stream info".into()));
    }
    let total_frames_hint = info.samples;
    let stored = info.md5sum;
    let has_signature = stored != [0u8; 16];

    let scale = 1.0f32 / (1u64 << (bits - 1)) as f32;
    let bytes_per_sample = ((bits + 7) / 8) as usize;
    let mut hasher = if has_signature && verify_md5 {
        Some(Md5::new())
    } else {
        None
    };

    let mut analyzer = StreamAnalyzer::new(channels);
    let mut frame_f32 = vec![0.0f32; channels];
    let mut frame_i32 = vec![0i32; channels];
    let mut byte_row: Vec<u8> = Vec::new();
    let mut frame_count: u64 = 0;

    let mut blocks = reader.blocks();
    let mut buffer: Vec<i32> = Vec::new();
    loop {
        let block = match blocks.read_next_or_eof(buffer) {
            Ok(Some(b)) => b,
            Ok(None) => break,
            Err(e) => return Err(AnalysisError::Decode(format!("flac decode error: {e}"))),
        };
        let n = block.duration() as usize;
        if hasher.is_some() {
            byte_row.clear();
            byte_row.reserve(n * channels * bytes_per_sample);
        }
        for t in 0..n {
            for c in 0..channels {
                let s = block.channel(c as u32)[t];
                frame_i32[c] = s;
                frame_f32[c] = s as f32 * scale;
                if hasher.is_some() {
                    let le = (s as u32).to_le_bytes();
                    byte_row.extend_from_slice(&le[..bytes_per_sample]);
                }
            }
            analyzer.push_frame(&frame_f32, Some(&frame_i32));
            frame_count += 1;
        }
        if let Some(h) = &mut hasher {
            h.update(&byte_row);
        }
        buffer = block.into_buffer();
    }

    let md5_status = if !has_signature {
        FlacMd5Status::NoSignature
    } else if let Some(h) = hasher {
        if h.finalize().as_slice() == stored.as_slice() {
            FlacMd5Status::Match
        } else {
            FlacMd5Status::Mismatch
        }
    } else {
        FlacMd5Status::Present
    };

    let duration_secs = total_frames_hint
        .map(|n| n as f64 / sample_rate as f64)
        .unwrap_or(frame_count as f64 / sample_rate as f64);

    Ok((
        DecodeOutcome {
            format: "FLAC".to_string(),
            sample_rate,
            channels,
            declared_bits: Some(bits),
            duration_secs,
            analyzer,
        },
        md5_status,
    ))
}

/// Lightweight header information obtained without decoding the whole file.
#[derive(Debug, Clone)]
pub struct BasicInfo {
    pub sample_rate: u32,
    pub channels: usize,
    pub bits: Option<u32>,
    pub format: String,
}

/// Read basic stream parameters from a file's header only (fast — no full
/// decode). Used e.g. to caption spectrogram images.
pub fn probe_info(path: &Path) -> Result<BasicInfo, AnalysisError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| AnalysisError::Decode(format!("probe failed: {e}")))?;
    let track = probed
        .format
        .default_track()
        .ok_or_else(|| AnalysisError::Decode("no default track".into()))?;
    let p = &track.codec_params;
    Ok(BasicInfo {
        sample_rate: p.sample_rate.unwrap_or(0),
        channels: p.channels.map(|c| c.count()).unwrap_or(0),
        bits: p.bits_per_sample,
        format: format_label(path),
    })
}

/// Detect the *real* container from the file's magic bytes, independent of its
/// extension. Returns a canonical short name, or `None` if unrecognized.
pub fn detect_container(path: &Path) -> Option<&'static str> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut b = [0u8; 16];
    let n = f.read(&mut b).ok()?;
    if n < 4 {
        return None;
    }
    if &b[0..4] == b"fLaC" {
        return Some("FLAC");
    }
    if (&b[0..4] == b"RIFF" || &b[0..4] == b"RF64") && n >= 12 && &b[8..12] == b"WAVE" {
        return Some("WAV");
    }
    if &b[0..4] == b"FORM" && n >= 12 && (&b[8..12] == b"AIFF" || &b[8..12] == b"AIFC") {
        return Some("AIFF");
    }
    if &b[0..4] == b"OggS" {
        return Some("OGG");
    }
    if &b[0..4] == b"caff" {
        return Some("CAF");
    }
    if n >= 8 && &b[4..8] == b"ftyp" {
        return Some("MP4");
    }
    if &b[0..3] == b"ID3" {
        return Some("MP3");
    }
    if n >= 2 && b[0] == 0xFF && (b[1] & 0xF6) == 0xF0 {
        return Some("AAC"); // ADTS
    }
    if n >= 2 && b[0] == 0xFF && (b[1] & 0xE0) == 0xE0 {
        return Some("MP3"); // MPEG-1/2 audio frame sync
    }
    None
}

/// Canonical container name expected from a file's extension.
pub fn ext_canonical(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("flac") => Some("FLAC"),
        Some("wav") | Some("wave") => Some("WAV"),
        Some("aif") | Some("aiff") | Some("aifc") => Some("AIFF"),
        Some("m4a") | Some("mp4") | Some("alac") => Some("MP4"),
        Some("caf") => Some("CAF"),
        Some("ogg") | Some("oga") => Some("OGG"),
        Some("mp3") => Some("MP3"),
        Some("aac") => Some("AAC"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    /// The fused FLAC path feeds the analyzer with `s * (1 / 2^(bits-1))` as
    /// f32. This must be a *lossless* round-trip for every integer the format
    /// can produce at ≤ 24 bits — otherwise analysis results could drift from
    /// the exact samples the MD5 is computed over.
    #[test]
    fn f32_normalization_roundtrips_exactly_up_to_24_bits() {
        for bits in [8u32, 12, 16, 20, 24] {
            let scale = 1.0f32 / (1u64 << (bits - 1)) as f32;
            let max = (1i64 << (bits - 1)) - 1;
            let probes = [0i64, 1, -1, 2, -2, max, -max - 1, max / 3, -(max / 7), max - 1];
            for &s in &probes {
                let f = s as f32 * scale;
                let back = (f / scale).round() as i64;
                assert_eq!(back, s, "bits={bits} sample={s}");
            }
        }
    }
}

/// Human-readable container/codec label from the file extension.
fn format_label(path: &Path) -> String {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("flac") => "FLAC",
        Some("wav") | Some("wave") => "WAV",
        Some("aif") | Some("aiff") | Some("aifc") => "AIFF",
        Some("alac") | Some("m4a") | Some("mp4") => "ALAC/MP4",
        Some("caf") => "CAF",
        Some("ogg") | Some("oga") => "OGG",
        Some("mp3") => "MP3",
        Some("aac") => "AAC",
        Some(other) => return other.to_uppercase(),
        None => "?",
    }
    .to_string()
}
