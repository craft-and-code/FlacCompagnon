//! End-to-end validation of the AAC re-quantization detector against a "toy
//! codec": MDCT (KBD window) → per-band quantization on the exact AAC grid
//! (`|X| = n^(4/3)·Δ`) → IMDCT + overlap-add. Re-analyzing that signal must
//! reveal the grid at onset 0; genuine noise must stay far below the detection
//! threshold. This mirrors the Python study used to calibrate the thresholds
//! (real ffmpeg transcodes scored 0.70–0.97; genuine audio ≤ 0.014).

use flaccompagnon_core::requant::{analyze_segment, kbd_window, Mdct, DETECT_RATE, L, N, SEGMENT_LEN, SWB_4448};

/// Deterministic noise in [-1, 1).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((self.0 >> 33) as f64 / (1u64 << 31) as f64) - 1.0
    }
}

/// Direct IMDCT + overlap-add of quantized coefficient frames (test-only, O(N²)).
fn imdct_ola(frames: &[Vec<f64>], win: &[f64]) -> Vec<f64> {
    let n0 = N as f64 / 2.0 + 0.5;
    let nfr = frames.len();
    let mut out = vec![0.0f64; (nfr + 1) * N];
    for (m, coefs) in frames.iter().enumerate() {
        for n in 0..L {
            let mut acc = 0.0;
            for (k, &c) in coefs.iter().enumerate() {
                acc += c
                    * (std::f64::consts::PI / N as f64 * (n as f64 + n0) * (k as f64 + 0.5)).cos();
            }
            out[m * N + n] += (2.0 / N as f64) * acc * win[n];
        }
    }
    out
}

/// Build a toy AAC transcode: quantize each band of each frame on the AAC grid.
fn toy_transcode(signal: &[f64], nfr: usize) -> Vec<f64> {
    let win = kbd_window();
    let mdct = Mdct::new();
    let mut frames: Vec<Vec<f64>> = Vec::with_capacity(nfr);
    let mut coefs = vec![0.0f64; N];
    for m in 0..nfr {
        mdct.forward(&signal[m * N..m * N + L], &win, &mut coefs);
        let mut q = vec![0.0f64; N];
        for b in 0..SWB_4448.len() - 1 {
            let (lo, hi) = (SWB_4448[b], SWB_4448[b + 1]);
            let mx = coefs[lo..hi].iter().fold(0.0f64, |a, &v| a.max(v.abs()));
            if mx <= 0.0 {
                continue;
            }
            // Δ chosen so the largest quantized magnitude is ~12. The coarse
            // grid guarantees each band's smallest surviving values quantize to
            // 1–3, which the detector's step estimator (divisors 1/2/3 of the
            // band minimum) is built for. Real music reaches those small values
            // naturally through its wide in-band dynamics; the synthetic noise
            // used here has a narrow Rayleigh spread, so a fine grid (e.g. ~120
            // levels) would leave band minima around 10–15 and the step would
            // be underestimated (measured: hit-rate 0.30 with 120 vs 0.98 with
            // 12 — validated against a bit-exact Python replica).
            let delta = (mx.powf(0.75) / 12.0).powf(4.0 / 3.0);
            let step = delta.powf(0.75);
            for k in lo..hi {
                let v = coefs[k];
                let qi = (v.abs().powf(0.75) / step).round();
                q[k] = v.signum() * qi.powf(4.0 / 3.0) * delta;
            }
        }
        frames.push(q);
    }
    imdct_ola(&frames, &win)
}

fn make_signal(len: usize, seed: u64) -> Vec<f64> {
    // Noise shaped to decay with frequency a little (more music-like than white):
    // simple one-pole lowpass mix keeps energy across all bands.
    let mut rng = Lcg(seed);
    let mut prev = 0.0f64;
    (0..len)
        .map(|_| {
            let w = rng.next();
            prev = 0.6 * prev + 0.4 * w;
            0.5 * prev + 0.1 * w
        })
        .collect()
}

#[test]
fn toy_aac_transcode_is_detected() {
    let nfr = 40; // enough frames to cover SEGMENT_LEN
    let sig = make_signal((nfr + 2) * N, 42);
    let rec = toy_transcode(&sig, nfr);
    assert!(rec.len() >= SEGMENT_LEN, "toy signal too short");
    let res = analyze_segment(&rec[..SEGMENT_LEN], None).expect("segment analyzed");
    assert!(
        res.rate >= DETECT_RATE,
        "toy transcode not detected: rate {} at onset {}",
        res.rate,
        res.onset
    );
    assert_eq!(res.onset, 0, "grid should be found at onset 0");
}

#[test]
fn genuine_noise_is_not_detected() {
    let sig = make_signal(SEGMENT_LEN + N, 4242);
    match analyze_segment(&sig[..SEGMENT_LEN], None) {
        None => {}
        Some(res) => assert!(
            res.rate < DETECT_RATE,
            "false positive: rate {} at onset {}",
            res.rate,
            res.onset
        ),
    }
}
