//! Independent integer fixtures exercise the complete decoder-to-verdict path.
//! The source is quantized before export; no detector helper generates truth.

use std::{f64::consts::TAU, path::Path};

use flaccompagnon_core::{
    analysis::{
        analyzer::AnalysisSummary,
        bitdepth::BitDepthMethod,
        detections::{classify, Detections},
    },
    decode::{decode_and_analyze, decode_and_analyze_flac, DecodeOutcome, FlacMd5Status},
    pipeline::analyze_file_cancellable,
    transcode::TranscodeEvidence,
    FileAnalysis, ScanOptions,
};
use flacenc::{component::BitRepr, error::Verify};

const RATE: u32 = 44_100;
const FRAMES: usize = 16_384;

struct Noise(u64);

impl Noise {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.0 >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

fn source_16() -> Vec<i32> {
    (0..FRAMES)
        .map(|n| {
            let time = n as f64 / f64::from(RATE);
            let value = 0.53 * (TAU * 997.3 * time).sin()
                + 0.21 * (TAU * 1739.1 * time).sin()
                + 0.09 * (TAU * 137.7 * time).cos();
            (value * 32768.0).round() as i32
        })
        .collect()
}

fn native_24() -> Vec<i32> {
    (0..FRAMES)
        .map(|n| (6_000_000.0 * (TAU * 997.3 * n as f64 / f64::from(RATE)).sin()).round() as i32)
        .collect()
}

fn export_24(shaped: bool) -> Vec<i32> {
    let mut noise = Noise(0x5eeda11);
    let mut previous = 0.0;
    source_16()
        .into_iter()
        .map(|sample| {
            // Two independent uniforms produce TPDF noise. First differencing
            // moves its energy upward without touching the 16-bit source.
            let current = (noise.next() - noise.next()) * if shaped { 6.0 } else { 1.0 };
            let residual = if shaped { current - previous } else { current };
            previous = current;
            (sample << 8) + residual.round() as i32
        })
        .collect()
}

fn write_wav(path: &Path, bits: u16, channels: u16, samples: &[i32]) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: RATE,
        bits_per_sample: bits,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create fixture");
    for &sample in samples {
        writer.write_sample(sample).expect("write fixture sample");
    }
    writer.finalize().expect("finish fixture");
}

fn write_flac(path: &Path, samples: &[i32]) {
    let mut config = flacenc::config::Encoder::default();
    config.multithread = false;
    config.block_size = 4096;
    let config = config.into_verified().expect("valid encoder config");
    let source = flacenc::source::MemSource::from_samples(samples, 1, 24, RATE as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .expect("encode independent fixture");
    let mut bytes = flacenc::bitsink::ByteSink::new();
    stream.write(&mut bytes).expect("serialize fixture");
    std::fs::write(path, bytes.as_slice()).expect("save fixture");
}

fn finish(decoded: DecodeOutcome) -> AnalysisSummary {
    assert_eq!(decoded.sample_rate, RATE);
    decoded
        .analyzer
        .finish(decoded.sample_rate, decoded.declared_bits)
}

fn wav_summary(bits: u16, channels: u16, samples: &[i32]) -> AnalysisSummary {
    let directory = tempfile::tempdir().expect("fixture directory");
    let path = directory.path().join("fixture.wav");
    write_wav(&path, bits, channels, samples);
    let decoded = decode_and_analyze(&path).expect("decode fixture");
    assert_eq!(decoded.declared_bits, Some(u32::from(bits)));
    assert_eq!(decoded.channels, usize::from(channels));
    finish(decoded)
}

fn verdict(summary: &AnalysisSummary, declared: u32) -> Detections {
    // Upscaling is independent of codec detection; avoid running an expensive
    // unrelated codec search and supply its completed negative result.
    classify(
        summary,
        RATE,
        Some(declared),
        summary.real_bit_depth,
        Ok(TranscodeEvidence {
            likelihood: 0.0,
            offset: 0,
            detected: false,
        }),
    )
}

fn assert_depth(summary: &AnalysisSummary, depth: u32, stored: u32, method: BitDepthMethod) {
    assert_eq!(summary.real_bit_depth, Some(depth));
    let evidence = summary.bit_depth_evidence.expect("integer evidence");
    assert_eq!(evidence.stored_bits, stored);
    assert_eq!(evidence.method, method);
}

#[test]
fn wav_padding_and_export_dither_retain_the_known_16_bit_source() {
    let padded: Vec<_> = source_16().into_iter().map(|sample| sample << 8).collect();
    for (samples, stored, method) in [
        (padded, 16, BitDepthMethod::Stored),
        (export_24(false), 24, BitDepthMethod::NarrowGrid),
        (export_24(true), 24, BitDepthMethod::NarrowGrid),
    ] {
        let summary = wav_summary(24, 1, &samples);
        assert_depth(&summary, 16, stored, method);
        let detection = verdict(&summary, 24);
        assert!(detection.upscaling, "{}", detection.detail);
        assert_eq!(detection.summary, "Flagged");
    }
}

#[test]
fn shaped_export_residual_larger_than_eight_lsb_does_not_hide_upscaling() {
    let samples = export_24(true);
    let residual_max = samples
        .iter()
        .map(|v| ((v + 128).rem_euclid(256) - 128).abs())
        .max();
    assert!(residual_max.is_some_and(|max| (9..=12).contains(&max)));
    assert_depth(
        &wav_summary(24, 1, &samples),
        16,
        24,
        BitDepthMethod::NarrowGrid,
    );
}

#[test]
fn independent_flac_encoding_matches_both_decoder_paths() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let path = directory.path().join("export.flac");
    for samples in [
        source_16().into_iter().map(|v| v << 8).collect::<Vec<_>>(),
        export_24(false),
        export_24(true),
    ] {
        write_flac(&path, &samples);
        let generic = finish(decode_and_analyze(&path).expect("generic FLAC decode"));
        let (decoded, md5) = decode_and_analyze_flac(&path, true).expect("fused FLAC decode");
        assert_eq!(md5, FlacMd5Status::Match);
        let fused = finish(decoded);
        assert_eq!(generic.real_bit_depth, Some(16));
        assert_eq!(fused.real_bit_depth, generic.real_bit_depth);
        assert_eq!(fused.bit_depth_evidence, generic.bit_depth_evidence);
        assert!(verdict(&fused, 24).upscaling);
    }
}

#[test]
fn directly_generated_native_24_bit_sine_and_noise_are_not_upscaled() {
    let mut random = Noise(0xfeed100);
    let sine = native_24();
    let noise: Vec<_> = (0..FRAMES)
        .map(|_| ((random.next() - 0.5) * 12_000_000.0).round() as i32)
        .collect();
    let noisy_sine: Vec<_> = sine
        .iter()
        .map(|v| v + ((random.next() - 0.5) * 100.0).round() as i32)
        .collect();
    for samples in [sine, noise, noisy_sine] {
        let summary = wav_summary(24, 1, &samples);
        assert_depth(&summary, 24, 24, BitDepthMethod::Stored);
        let detection = verdict(&summary, 24);
        assert!(!detection.upscaling, "{}", detection.detail);
        assert_eq!(detection.summary, "Clean");
    }
}

#[test]
fn one_low_depth_channel_does_not_override_a_native_channel() {
    let samples: Vec<_> = export_24(true)
        .into_iter()
        .zip(native_24())
        .flat_map(|(left, right)| [left, right])
        .collect();
    let summary = wav_summary(24, 2, &samples);
    assert_depth(&summary, 24, 24, BitDepthMethod::Stored);
    assert!(!verdict(&summary, 24).upscaling);
}

#[test]
fn native_tail_vetoes_an_apparently_upscaled_file() {
    let mut samples = export_24(true);
    samples.extend(native_24().into_iter().take(64));
    let summary = wav_summary(24, 1, &samples);
    assert_depth(&summary, 24, 24, BitDepthMethod::Stored);
    assert!(!verdict(&summary, 24).upscaling);
}

#[test]
fn decoded_digital_silence_keeps_the_one_bit_upscaling_verdict() {
    let summary = wav_summary(16, 2, &vec![0; FRAMES * 2]);
    assert_depth(&summary, 1, 1, BitDepthMethod::Stored);
    let detection = verdict(&summary, 16);
    assert!(detection.upscaling);
    assert_eq!(detection.summary, "Flagged");
    assert!(detection.detail.contains("digital silence"));
}

#[test]
fn saved_precision_evidence_round_trips_and_older_reports_still_load() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let path = directory.path().join("export.wav");
    write_wav(&path, 24, 1, &export_24(true));
    let report = analyze_file_cancellable(&path, &ScanOptions::default(), &|| true);
    assert!(report.error.is_none(), "{:?}", report.error);
    assert_eq!(report.real_bit_depth, Some(16));
    assert_eq!(
        report.bit_depth_evidence.expect("evidence").method,
        BitDepthMethod::NarrowGrid
    );
    let mut json = serde_json::to_value(&report).expect("serialize current report");
    let current: FileAnalysis = serde_json::from_value(json.clone()).expect("load current report");
    assert_eq!(current.bit_depth_evidence, report.bit_depth_evidence);
    json.as_object_mut()
        .expect("report object")
        .remove("bit_depth_evidence");
    let old: FileAnalysis = serde_json::from_value(json).expect("load old report without evidence");
    assert_eq!(old.bit_depth_evidence, None);
    assert_eq!(old.real_bit_depth, report.real_bit_depth);
}
