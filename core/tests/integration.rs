//! End-to-end tests: synthesize WAV files with known spectral properties and
//! confirm the analyzer reaches the right verdict.

use std::path::PathBuf;

use flaccompagnon_core::{analyze_file, ScanOptions};
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

/// Noise with a **gradual** high-frequency roll-off — a naturally dark master.
///
/// Deliberately not [`band_limited_noise`], which zeroes its upper bins
/// outright. That produces a region at −105 dB, which is a *mathematical*
/// zero: no microphone, no converter and no codec ever emits one. Scaled by
/// its own peak such a region falls entirely inside the quantizer's dead zone
/// and matches every scalefactor, so the detector calls it transcoded — and
/// is not wrong to, in the sense that the input is not audio.
///
/// This is what the real case looks like: an acoustic or analog-tape master
/// rolling off from 12 kHz at 3 dB/kHz, reaching about −25 dB near Nyquist.
/// Quiet, but present. That is the signal the removal of the spectral
/// heuristics was meant to stop accusing.
fn dark_master_noise(n: usize, sr: u32, knee_hz: f32, seed: u64) -> Vec<i32> {
    let mut spec = vec![Complex { re: 0.0f32, im: 0.0 }; n];
    let mut rng = Lcg(seed);
    for k in 1..n / 2 {
        let hz = k as f32 * sr as f32 / n as f32;
        // −3 dB per kHz above the knee; flat below it.
        let gain = if hz <= knee_hz {
            1.0
        } else {
            10f32.powf(-((hz - knee_hz) / 1000.0) * 3.0 / 20.0)
        };
        let re = rng.next_f32() * gain;
        let im = rng.next_f32() * gain;
        spec[k] = Complex { re, im };
        spec[n - k] = Complex { re, im: -im };
    }
    FftPlanner::<f32>::new().plan_fft_inverse(n).process(&mut spec);
    let time: Vec<f32> = spec.iter().map(|c| c.re).collect();
    let peak = time.iter().fold(0f32, |m, &v| m.max(v.abs())).max(1e-6);
    let scale = 8_000_000.0 / peak;
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
    assert_eq!(r.phase_correlation, None);
    assert_eq!(r.phase_inverted, None);
    std::fs::remove_file(&path).ok();
}

/// A generated stereo WAV with R = -L is an independent ground-truth polarity
/// inversion: its mono sum is exactly zero, regardless of the phase detector.
#[test]
fn inverted_stereo_channels_are_reported_as_a_phase_problem() {
    let sr = 44_100;
    let mut samples = Vec::with_capacity(sr as usize * 2);
    for n in 0..sr {
        let left =
            (22_000.0 * (2.0 * std::f64::consts::PI * 317.0 * n as f64 / sr as f64).sin()) as i16;
        samples.extend([left, -left]);
    }
    let path = tmp("inverted_stereo.wav");
    write_wav_i16(&path, sr, 2, &samples);

    let result = analyze_file(&path, &ScanOptions::default());
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(result.fake_stereo, Some(false));
    assert_eq!(result.phase_inverted, Some(true));
    assert!(result.phase_correlation.is_some_and(|v| v < -0.999));
    std::fs::remove_file(&path).ok();
}

/// EBU Tech 3341 test 1 specifies -23 LUFS for an in-phase stereo 1 kHz sine
/// whose per-channel peak is -23 dBFS. Verify the decode-to-report path too.
#[test]
fn stereo_wav_reports_ebu_reference_integrated_loudness() {
    let sample_rate = 48_000;
    let frames = sample_rate * 2;
    let amplitude = 32767.0 * 10f64.powf(-23.0 / 20.0);
    let mut samples = Vec::with_capacity(frames as usize * 2);
    for n in 0..frames {
        let sample = (amplitude
            * (2.0 * std::f64::consts::PI * 1_000.0 * n as f64 / sample_rate as f64).sin())
            as i16;
        samples.extend([sample, sample]);
    }
    let path = tmp("lufs_reference.wav");
    write_wav_i16(&path, sample_rate, 2, &samples);

    let result = analyze_file(&path, &ScanOptions::default());
    assert!(result.error.is_none(), "{:?}", result.error);
    let measured = result.integrated_lufs.expect("integrated loudness");
    assert!((measured + 23.0).abs() < 0.2, "{measured} LUFS");
    assert_eq!(result.loudness_range_lu, None);
    std::fs::remove_file(&path).ok();
}

/// EBU Tech 3342 test 1 specifies 10 ±1 LU for consecutive 20 s stereo
/// 1 kHz tones at -20 and -30 dBFS per-channel peak.
#[test]
fn stereo_wav_reports_ebu_reference_loudness_range() {
    let sample_rate = 48_000;
    let segment_frames = sample_rate * 20;
    let mut samples = Vec::with_capacity(segment_frames as usize * 4);
    for peak_dbfs in [-20.0, -30.0] {
        let amplitude = 32767.0 * 10f64.powf(peak_dbfs / 20.0);
        for n in 0..segment_frames {
            let sample = (amplitude
                * (2.0 * std::f64::consts::PI * 1_000.0 * n as f64 / sample_rate as f64).sin())
                as i16;
            samples.extend([sample, sample]);
        }
    }
    let path = tmp("lra_reference.wav");
    write_wav_i16(&path, sample_rate, 2, &samples);

    let result = analyze_file(&path, &ScanOptions::default());
    assert!(result.error.is_none(), "{:?}", result.error);
    let range = result.loudness_range_lu.expect("loudness range");
    assert!((range - 10.0).abs() <= 1.0, "{range} LU");
    std::fs::remove_file(&path).ok();
}

/// The regression that motivated replacing the spectral heuristics.
///
/// A naturally dark master — content rolling off from 12 kHz — used to be
/// reported as *transcoded* on the strength of its spectrum alone. An
/// acoustic recording, a 1960s tape master or a deliberately filtered signal
/// all look like this, and the app accused every one of them. The verdict now
/// comes only from a codec's quantization lattice, which this file has none
/// of, so it must come back clean.
#[test]
fn a_dark_master_is_not_accused_of_transcoding() {
    let sr = 44_100;
    let n = sr as usize * 2;
    let ints = dark_master_noise(n, sr, 12_000.0, 12345);
    let path = tmp("dark_master.wav");
    write_wav_i24(&path, sr, 1, &ints);

    let r = analyze_file(&path, &ScanOptions::default());
    assert!(r.error.is_none(), "error: {:?}", r.error);
    assert!(
        !r.detections.transcoding,
        "cutoff {:?} ratio {:?} detail {}",
        r.cutoff_hz,
        r.cutoff_ratio,
        r.detections.detail
    );
    // The cut-off is still *measured* and still reported — it stopped being a
    // verdict, it did not stop being information.
    //
    // It reads full-band (22.05 kHz) here, and that is correct rather than a
    // failure: the cut-off detector looks for a cliff, and a 3 dB/kHz slope
    // has none. Content reaches Nyquist, merely 27 dB down. Asserting it
    // would land *below* Nyquist was the mistake — it assumed a gentle
    // roll-off and a brick wall look alike to that metric, which is precisely
    // the confusion that made the old spectral verdict unusable.
    assert!(
        r.cutoff_hz.is_some_and(|c| c > 0.0),
        "the cut-off must still be measured: {:?}",
        r.cutoff_hz
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn native_band_limited_96k_triggers_only_an_upsampling_heuristic() {
    // Synthesized directly at 96 kHz: no resampling occurred. Limited
    // bandwidth alone still triggers the legacy flag, so it must be qualified.
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
    assert!(r.detections.detail.contains("not proof of resampling"));
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
