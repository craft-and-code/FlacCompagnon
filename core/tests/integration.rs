//! End-to-end tests: synthesize WAV files with known spectral properties and
//! confirm the analyzer reaches the right verdict.

use std::path::PathBuf;

use flaccompagnon_core::{analyze_file, ScanOptions, TranscodeState};
use rustfft::{num_complex::Complex, FftPlanner};

/// Synthesize genuinely band-limited noise: build a spectrum that is random
/// below `cutoff_hz` and exactly zero above it, then inverse-FFT. This produces
/// a hard spectral ceiling (a real dead zone) with no leakage, unlike a sum of
/// discrete tones. Returned as 24-bit samples so the quantization floor sits
/// around -140 dBFS — far below the detector floor — leaving a clean dead zone
/// (16-bit noise would sit right at the threshold and mask it).
fn band_limited_noise(n: usize, sr: u32, cutoff_hz: f32, seed: u64) -> Vec<i32> {
    let mut spec = vec![Complex { re: 0.0f32, im: 0.0 }; n];
    let cutoff_bin = ((cutoff_hz as f64 * n as f64 / sr as f64) as usize).min(n / 2);
    let mut rng = Lcg(seed);
    for k in 1..cutoff_bin {
        let re = rng.next_f32();
        let im = rng.next_f32();
        spec[k] = Complex { re, im };
        spec[n - k] = Complex { re, im: -im }; // conjugate symmetry -> real output
    }
    FftPlanner::<f32>::new().plan_fft_inverse(n).process(&mut spec);
    let time: Vec<f32> = spec.iter().map(|c| c.re).collect();
    let peak = time.iter().fold(0f32, |m, &v| m.max(v.abs())).max(1e-6);
    let scale = 8_000_000.0 / peak; // ~ -0.4 dBFS in the 24-bit range
    time.iter().map(|v| (v * scale) as i32).collect()
}

fn tmp(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("flaccompagnon_test_{name}"));
    p
}

/// Simple deterministic LCG in [-1, 1).
struct Lcg(u64);
impl Lcg {
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((self.0 >> 33) as f32 / (1u64 << 31) as f32) - 1.0
    }
}

fn write_wav_i16(path: &PathBuf, sr: u32, channels: u16, samples: &[i16]) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: sr,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for &s in samples {
        w.write_sample(s).unwrap();
    }
    w.finalize().unwrap();
}

fn write_wav_i24(path: &PathBuf, sr: u32, channels: u16, samples: &[i32]) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: sr,
        bits_per_sample: 24,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for &s in samples {
        w.write_sample(s).unwrap();
    }
    w.finalize().unwrap();
}

#[test]
fn full_band_noise_is_clean() {
    let sr = 44_100;
    let n = sr as usize * 2;
    let mut rng = Lcg(12345);
    let samples: Vec<i16> = (0..n)
        .map(|_| (rng.next_f32() * 30_000.0) as i16)
        .collect();
    let path = tmp("clean.wav");
    write_wav_i16(&path, sr, 1, &samples);

    let r = analyze_file(&path, &ScanOptions::default());
    assert!(r.error.is_none(), "error: {:?}", r.error);
    assert_eq!(
        r.detections.summary, "Clean",
        "cutoff {:?}, detail {}",
        r.cutoff_hz, r.detections.detail
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn band_limited_signal_is_transcoded() {
    // Band-limited to 15 kHz -> a hard dead zone well below the 22.05 kHz
    // Nyquist, the signature of a lossy source at a standard rate.
    let sr = 44_100;
    let n = sr as usize * 2;
    let ints = band_limited_noise(n, sr, 15_000.0, 12345);
    let path = tmp("transcoded.wav");
    write_wav_i24(&path, sr, 1, &ints);

    let r = analyze_file(&path, &ScanOptions::default());
    assert!(r.error.is_none(), "error: {:?}", r.error);
    assert_eq!(
        r.detections.transcoding,
        TranscodeState::Detected,
        "cutoff {:?} ratio {:?} detail {}",
        r.cutoff_hz,
        r.cutoff_ratio,
        r.detections.detail
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn cd_content_in_96k_is_upsampled() {
    // Content only up to 18 kHz but placed in a 96 kHz container.
    let sr = 96_000;
    let n = sr as usize; // 1 second
    let ints = band_limited_noise(n, sr, 18_000.0, 999);
    let path = tmp("upsampled.wav");
    write_wav_i24(&path, sr, 1, &ints);

    let r = analyze_file(&path, &ScanOptions::default());
    assert!(r.error.is_none(), "error: {:?}", r.error);
    assert!(
        r.detections.upsampling,
        "cutoff {:?}, detail {}",
        r.cutoff_hz, r.detections.detail
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn fake_24bit_is_detected() {
    // 16-bit content shifted into a 24-bit container (low 8 bits always zero).
    let sr = 44_100;
    let n = sr as usize;
    let mut rng = Lcg(777);
    let samples: Vec<i32> = (0..n)
        .map(|_| ((rng.next_f32() * 30_000.0) as i32) << 8)
        .collect();
    let path = tmp("fake24.wav");
    write_wav_i24(&path, sr, 1, &samples);

    let r = analyze_file(&path, &ScanOptions::default());
    assert!(r.error.is_none(), "error: {:?}", r.error);
    assert_eq!(r.declared_bits, Some(24));
    assert!(
        r.detections.upscaling,
        "real bits {:?}",
        r.real_bit_depth
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn dual_mono_is_fake_stereo() {
    let sr = 44_100;
    let n = sr as usize;
    let mut rng = Lcg(2024);
    let mut interleaved = Vec::with_capacity(n * 2);
    for _ in 0..n {
        let s = (rng.next_f32() * 25_000.0) as i16;
        interleaved.push(s); // L
        interleaved.push(s); // R (identical)
    }
    let path = tmp("dualmono.wav");
    write_wav_i16(&path, sr, 2, &interleaved);

    let r = analyze_file(&path, &ScanOptions::default());
    assert!(r.error.is_none(), "error: {:?}", r.error);
    assert_eq!(r.fake_stereo, Some(true));
    std::fs::remove_file(&path).ok();
}
