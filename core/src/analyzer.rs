//! Streaming analyzer.
//!
//! The decoder feeds audio one frame at a time (a frame = one sample per
//! channel). Storing an entire album in memory would be wasteful, so every
//! metric is accumulated incrementally:
//!
//! * spectrum      -> Hann-windowed FFT of a mono downmix, averaged over windows
//! * clipping      -> full-scale sample counting with run detection
//! * fake stereo   -> energy of the L-R difference vs. the signal energy
//! * real bitdepth -> bitwise OR of every integer sample value
//!
//! Nothing here depends on a specific file format; [`decode`](crate::decode)
//! adapts each codec to the [`StreamAnalyzer::push_frame`] interface.

use std::sync::Arc;

use rustfft::{num_complex::Complex, Fft, FftPlanner};

use crate::mdct::{Mdct, AAC_N};
use crate::truepeak::TruePeak;
use crate::{bitdepth, clipping, requant, spectrum, ClippingInfo};

/// Full-scale detection threshold (normalized). Samples with |value| at or
/// above this are treated as clipped.
const CLIP_THRESHOLD: f32 = 0.9997;
/// FFT window size. 8192 gives ~5.4 Hz resolution at 44.1 kHz.
const FFT_SIZE: usize = 8192;

// --- MDCT (AAC-SIN transcode detector) -------------------------------------
/// Analyze one in every `MDCT_STRIDE` overlapping MDCT hops, so the transform
/// samples the whole track without processing every frame (O(N²) per frame).
const MDCT_STRIDE: u64 = 4;
/// Cap on analyzed MDCT frames — keeps a long album fast.
const MDCT_MAX_FRAMES: u32 = 240;
/// A dead zone only "exists" if the per-frame cutoff is below this fraction of N.
const MDCT_DEAD_TOP_RATIO: f32 = 0.92;

// --- Re-quantization detector segment buffers -------------------------------
/// Preferred segment start (sample index; multiple of 1024 ≈ 8 s at 44.1 kHz).
const REQ_MAIN_START: u64 = 344 * 1024;

// --- Dynamics (DR) ----------------------------------------------------------
/// Frames per RMS block (~3 s at 44.1 kHz). The exact wall-clock length is not
/// critical: the DR estimate averages the loudest blocks, so the granularity
/// only needs to be long enough to smooth individual drum hits.
const DYN_BLOCK_FRAMES: u64 = 131_072;
/// Fraction of the loudest blocks that defines the "loud passages" RMS,
/// mirroring the classic DR-meter approach (peak vs loudest-20% RMS).
const DYN_TOP_FRACTION: f64 = 0.2;

/// Aggregated results produced by [`StreamAnalyzer::finish`].
#[derive(Debug, Clone)]
pub struct AnalysisSummary {
    pub cutoff_hz: f64,
    pub cutoff_ratio: f64,
    /// How sharply the level drops at the cutoff (dB). Large == a brick wall.
    pub cliff_db: f32,
    /// Mean level just above the cutoff (dB rel. peak) — the dead-zone depth.
    pub above_db: f32,
    /// Averaged magnitude spectrum in dB, one entry per FFT bin (0..=N/2).
    pub spectrum_db: Vec<f32>,
    pub clipping: ClippingInfo,
    pub fake_stereo: bool,
    pub real_bit_depth: Option<u32>,

    /// Dynamic-range estimate in dB: peak level vs the RMS of the loudest 20%
    /// of ~3 s blocks (crest factor of the loud passages, DR-meter style).
    /// High values (>= 12 dB) indicate a dynamic master; low values (< 8 dB)
    /// a loudness-war master. `None` for silent or extremely short streams.
    pub dr_db: Option<f32>,

    // --- MDCT (AAC-SIN) transcode evidence ---
    /// Mean per-frame MDCT cutoff as a fraction of Nyquist (dead-zone frames).
    pub mdct_cutoff_ratio: Option<f64>,
    /// Mean level (dB rel. frame peak) of the MDCT dead zone above the cutoff.
    pub mdct_dead_db: Option<f32>,
    /// Fraction of analyzed MDCT frames that showed a high-frequency dead zone.
    pub mdct_dead_fraction: Option<f32>,

    // --- AAC re-quantization evidence (strongest transcode proof) ---
    /// Hit-rate of on-grid bands at the best synchronized MDCT onset (0..1).
    /// ≥ [`requant::DETECT_RATE`] means an AAC source. `None` when the check
    /// did not run (unsupported rate or file too short).
    pub requant_rate: Option<f32>,
}

/// Incremental audio analyzer.
pub struct StreamAnalyzer {
    channels: usize,

    // --- spectrum ---
    fft: Arc<dyn Fft<f32>>,
    hann: Vec<f32>,
    frame_buf: Vec<f32>, // mono accumulation buffer, length grows to FFT_SIZE
    power_acc: Vec<f64>, // accumulated |X|^2 per bin, length FFT_SIZE/2 + 1
    window_count: u64,

    // --- clipping ---
    clip_state: clipping::ClipState,
    true_peak: TruePeak,

    // --- fake stereo ---
    diff_energy: f64,
    l_energy: f64,
    r_energy: f64,
    identical_frames: u64,
    total_frames: u64,

    // --- bit depth ---
    int_or_mask: u32,
    saw_integer: bool,

    // --- MDCT (AAC-SIN) ---
    mdct: &'static Mdct,
    mdct_prev: Vec<f32>, // previous hop (N mono samples)
    mdct_fill: Vec<f32>, // current hop being filled
    mdct_have_prev: bool,
    mdct_hop: u64,
    mdct_scratch: Vec<f32>, // N coefficients
    mdct_frames: u32,
    mdct_dead_frames: u32,
    mdct_cutoff_ratio_sum: f64,
    mdct_dead_db_sum: f64,

    // --- Re-quantization segment buffers (L/R as f64) ---
    req_early_l: Vec<f64>,
    req_early_r: Vec<f64>,
    req_main_l: Vec<f64>,
    req_main_r: Vec<f64>,

    // --- dynamics (DR) ---
    dyn_block_sumsq: f64, // running sum of per-frame mean squares
    dyn_block_frames: u64,
    dyn_blocks: Vec<f64>, // mean square of each completed block
}

impl StreamAnalyzer {
    pub fn new(channels: usize) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let hann: Vec<f32> = (0..FFT_SIZE)
            .map(|n| {
                let x = std::f32::consts::PI * n as f32 / (FFT_SIZE as f32 - 1.0);
                x.sin().powi(2) // Hann window == sin^2
            })
            .collect();
        Self {
            channels: channels.max(1),
            fft,
            hann,
            frame_buf: Vec::with_capacity(FFT_SIZE),
            power_acc: vec![0.0; FFT_SIZE / 2 + 1],
            window_count: 0,
            clip_state: clipping::ClipState::new(CLIP_THRESHOLD),
            true_peak: TruePeak::new(channels.max(1)),
            diff_energy: 0.0,
            l_energy: 0.0,
            r_energy: 0.0,
            identical_frames: 0,
            total_frames: 0,
            int_or_mask: 0,
            saw_integer: false,
            mdct: Mdct::shared(),
            mdct_prev: Vec::with_capacity(AAC_N),
            mdct_fill: Vec::with_capacity(AAC_N),
            mdct_have_prev: false,
            mdct_hop: 0,
            mdct_scratch: vec![0.0; AAC_N],
            mdct_frames: 0,
            mdct_dead_frames: 0,
            mdct_cutoff_ratio_sum: 0.0,
            mdct_dead_db_sum: 0.0,
            req_early_l: Vec::with_capacity(requant::SEGMENT_LEN),
            req_early_r: Vec::with_capacity(requant::SEGMENT_LEN),
            req_main_l: Vec::with_capacity(requant::SEGMENT_LEN),
            req_main_r: Vec::with_capacity(requant::SEGMENT_LEN),
            dyn_block_sumsq: 0.0,
            dyn_block_frames: 0,
            dyn_blocks: Vec::new(),
        }
    }

    /// Push one frame of normalized-float samples (`samples.len() == channels`),
    /// optionally accompanied by the raw integer sample values for the same
    /// frame (used for effective bit-depth estimation).
    pub fn push_frame(&mut self, samples: &[f32], int_samples: Option<&[i32]>) {
        if samples.is_empty() {
            return;
        }
        let idx = self.total_frames; // sample index of this frame
        self.total_frames += 1;

        // Capture the re-quantization segments (first two channels, f64).
        // Both segment starts are multiples of 1024, preserving AAC frame
        // alignment modulo the block length.
        let l = samples[0] as f64;
        let r = if samples.len() >= 2 { samples[1] as f64 } else { l };
        if self.req_early_l.len() < requant::SEGMENT_LEN {
            self.req_early_l.push(l);
            if self.channels >= 2 {
                self.req_early_r.push(r);
            }
        }
        if idx >= REQ_MAIN_START && self.req_main_l.len() < requant::SEGMENT_LEN {
            self.req_main_l.push(l);
            if self.channels >= 2 {
                self.req_main_r.push(r);
            }
        }

        // Clipping + peak (per channel), and the frame's mean square for the
        // dynamics blocks.
        let mut frame_sumsq = 0.0f64;
        for &s in samples {
            self.clip_state.push(s);
            frame_sumsq += (s as f64) * (s as f64);
        }
        self.dyn_block_sumsq += frame_sumsq / samples.len() as f64;
        self.dyn_block_frames += 1;
        self.true_peak.push_frame(samples);
        if self.dyn_block_frames == DYN_BLOCK_FRAMES {
            self.dyn_blocks
                .push(self.dyn_block_sumsq / self.dyn_block_frames as f64);
            self.dyn_block_sumsq = 0.0;
            self.dyn_block_frames = 0;
        }

        // Stereo difference energy (first two channels).
        if self.channels >= 2 && samples.len() >= 2 {
            let l = samples[0] as f64;
            let r = samples[1] as f64;
            let d = l - r;
            self.diff_energy += d * d;
            self.l_energy += l * l;
            self.r_energy += r * r;
            if (l - r).abs() < 1e-9 {
                self.identical_frames += 1;
            }
        }

        // Integer OR mask for bit-depth estimation. Raw two's-complement value;
        // sign extension is handled later by masking to the declared width.
        if let Some(ints) = int_samples {
            self.saw_integer = true;
            for &v in ints {
                self.int_or_mask |= v as u32;
            }
        }

        // Mono downmix into the FFT buffer.
        let mut mono = 0.0f32;
        for &s in samples {
            mono += s;
        }
        mono /= samples.len().max(1) as f32;
        self.frame_buf.push(mono);
        if self.frame_buf.len() == FFT_SIZE {
            self.process_window();
            self.frame_buf.clear();
        }

        // Feed the same mono sample to the MDCT pipeline (frame = 2N, hop = N).
        self.mdct_fill.push(mono);
        if self.mdct_fill.len() == AAC_N {
            self.mdct_hop += 1;
            if self.mdct_have_prev
                && self.mdct_frames < MDCT_MAX_FRAMES
                && self.mdct_hop % MDCT_STRIDE == 0
            {
                self.process_mdct();
            }
            std::mem::swap(&mut self.mdct_prev, &mut self.mdct_fill);
            self.mdct_fill.clear();
            self.mdct_have_prev = true;
        }
    }

    /// Analyze one overlapping MDCT frame (previous hop + current hop) for the
    /// AAC-SIN transcode signature: a flat, sharply-bounded high-frequency dead
    /// zone in the sine-window MDCT domain.
    fn process_mdct(&mut self) {
        let n = AAC_N;
        let mut frame = Vec::with_capacity(2 * n);
        frame.extend_from_slice(&self.mdct_prev);
        frame.extend_from_slice(&self.mdct_fill);

        let m = self.mdct; // &'static, cheap to copy
        m.forward(&frame, &mut self.mdct_scratch);

        let mut peak = 0.0f32;
        for &c in &self.mdct_scratch {
            let a = c.abs();
            if a > peak {
                peak = a;
            }
        }
        self.mdct_frames += 1;
        if peak < 1e-7 {
            return; // silent frame
        }

        let thr = peak * 1e-4; // -80 dB relative to the frame peak
        let mut cutoff = 0usize;
        for k in (0..n).rev() {
            if self.mdct_scratch[k].abs() > thr {
                cutoff = k;
                break;
            }
        }

        let top = (MDCT_DEAD_TOP_RATIO * n as f32) as usize;
        if cutoff < top {
            let start = (cutoff + 8).min(n);
            if start < n {
                let mut sum = 0.0f64;
                let mut cnt = 0u32;
                for k in start..n {
                    let d = 20.0 * (self.mdct_scratch[k].abs() / peak).max(1e-12).log10();
                    sum += d as f64;
                    cnt += 1;
                }
                if cnt > 0 {
                    self.mdct_dead_db_sum += sum / cnt as f64;
                    self.mdct_cutoff_ratio_sum += cutoff as f64 / n as f64;
                    self.mdct_dead_frames += 1;
                }
            }
        }
    }

    fn process_window(&mut self) {
        let mut buf: Vec<Complex<f32>> = self
            .frame_buf
            .iter()
            .zip(self.hann.iter())
            .map(|(&s, &w)| Complex {
                re: s * w,
                im: 0.0,
            })
            .collect();
        self.fft.process(&mut buf);
        for (bin, c) in buf.iter().take(self.power_acc.len()).enumerate() {
            self.power_acc[bin] += (c.re as f64).powi(2) + (c.im as f64).powi(2);
        }
        self.window_count += 1;
    }

    /// Finalize and compute all summary metrics for a stream at `sample_rate`.
    /// `declared_bits` is the container's stated integer bit depth (if any) and
    /// is used to bound the effective bit-depth estimate.
    pub fn finish(mut self, sample_rate: u32, declared_bits: Option<u32>) -> AnalysisSummary {
        // Flush a trailing partial window (zero padded) so short files still
        // produce a spectrum.
        if self.window_count == 0 && !self.frame_buf.is_empty() {
            self.frame_buf.resize(FFT_SIZE, 0.0);
            self.process_window();
        }

        let spectrum_db = spectrum::average_to_db(&self.power_acc, self.window_count);
        let (cutoff_hz, cutoff_ratio) =
            spectrum::detect_cutoff(&spectrum_db, sample_rate, FFT_SIZE);
        let (cliff_db, above_db) =
            spectrum::cutoff_context(&spectrum_db, sample_rate, FFT_SIZE, cutoff_hz);

        // Dynamics: flush a meaningful trailing partial block (>= 1/4 length),
        // then compare the peak with the RMS of the loudest 20% of blocks.
        if self.dyn_block_frames >= DYN_BLOCK_FRAMES / 4 {
            self.dyn_blocks
                .push(self.dyn_block_sumsq / self.dyn_block_frames as f64);
        }
        let mut clipping = self.clip_state.finish(self.channels, self.total_frames);
        // True peak from the 4x-oversampled stream. It can legitimately sit a
        // hair above the sample peak on any material, and above 1.0 (positive
        // dBTP) on loud masters — that's the inter-sample clipping signal.
        clipping.true_peak = self.true_peak.peak();
        clipping.true_peak_dbtp = self.true_peak.peak_dbtp();
        let dr_db = {
            let mut blocks = self.dyn_blocks.clone();
            blocks.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
            let top = ((blocks.len() as f64 * DYN_TOP_FRACTION).ceil() as usize).max(1);
            let loud: Vec<f64> = blocks.into_iter().take(top).collect();
            if loud.is_empty() {
                None
            } else {
                let rms = (loud.iter().sum::<f64>() / loud.len() as f64).sqrt();
                if rms > 1e-9 && clipping.peak > 0.0 {
                    Some((20.0 * (clipping.peak as f64 / rms).log10()) as f32)
                } else {
                    None
                }
            }
        };

        let fake_stereo = if self.channels >= 2 {
            crate::stereo::is_fake(
                self.diff_energy,
                self.l_energy,
                self.r_energy,
                self.identical_frames,
                self.total_frames,
            )
        } else {
            false
        };

        let real_bit_depth = match (self.saw_integer, declared_bits) {
            (true, Some(declared)) => Some(bitdepth::effective_bits(self.int_or_mask, declared)),
            _ => None,
        };

        // AAC re-quantization check — only meaningful at the AAC frame rates
        // covered by the 44.1/48 kHz scale-factor band table.
        let requant_rate = if sample_rate == 44_100 || sample_rate == 48_000 {
            let (seg_l, seg_r) = if self.req_main_l.len() >= requant::SEGMENT_LEN {
                (&self.req_main_l, &self.req_main_r)
            } else {
                (&self.req_early_l, &self.req_early_r)
            };
            let right = if self.channels >= 2 && seg_r.len() >= requant::SEGMENT_LEN {
                Some(seg_r.as_slice())
            } else {
                None
            };
            requant::analyze_segment(seg_l, right).map(|r| r.rate)
        } else {
            None
        };

        let (mdct_cutoff_ratio, mdct_dead_db, mdct_dead_fraction) = if self.mdct_frames > 0 {
            let frac = self.mdct_dead_frames as f32 / self.mdct_frames as f32;
            if self.mdct_dead_frames > 0 {
                (
                    Some(self.mdct_cutoff_ratio_sum / self.mdct_dead_frames as f64),
                    Some((self.mdct_dead_db_sum / self.mdct_dead_frames as f64) as f32),
                    Some(frac),
                )
            } else {
                (None, None, Some(frac))
            }
        } else {
            (None, None, None)
        };

        AnalysisSummary {
            cutoff_hz,
            cutoff_ratio,
            cliff_db,
            above_db,
            spectrum_db,
            clipping,
            fake_stereo,
            real_bit_depth,
            dr_db,
            mdct_cutoff_ratio,
            mdct_dead_db,
            mdct_dead_fraction,
            requant_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pure sine has a crest factor of exactly sqrt(2) == 3.01 dB; the DR
    /// estimate on a steady full-scale sine must land there.
    #[test]
    fn dr_of_a_pure_sine_is_3db() {
        let mut a = StreamAnalyzer::new(1);
        let rate = 44_100.0f64;
        for n in 0..(DYN_BLOCK_FRAMES * 2) {
            let s = (2.0 * std::f64::consts::PI * 1000.0 * n as f64 / rate).sin() as f32 * 0.9;
            a.push_frame(&[s], None);
        }
        let summary = a.finish(44_100, None);
        let dr = summary.dr_db.expect("dr computed");
        assert!((dr - 3.01).abs() < 0.1, "dr = {dr}");
    }

    /// A hard-clipped ("brickwalled") sine approaches a square wave whose
    /// crest factor tends to 0 dB — the loudness-war signature.
    #[test]
    fn dr_of_a_squashed_sine_is_low() {
        let mut a = StreamAnalyzer::new(1);
        let rate = 44_100.0f64;
        for n in 0..(DYN_BLOCK_FRAMES * 2) {
            let raw = (2.0 * std::f64::consts::PI * 1000.0 * n as f64 / rate).sin() * 8.0;
            let s = raw.clamp(-0.98, 0.98) as f32;
            a.push_frame(&[s], None);
        }
        let summary = a.finish(44_100, None);
        let dr = summary.dr_db.expect("dr computed");
        assert!(dr < 1.0, "dr = {dr}");
    }

    /// Silence yields no DR value rather than a bogus number.
    #[test]
    fn dr_of_silence_is_none() {
        let mut a = StreamAnalyzer::new(1);
        for _ in 0..(DYN_BLOCK_FRAMES + 10) {
            a.push_frame(&[0.0], None);
        }
        let summary = a.finish(44_100, None);
        assert!(summary.dr_db.is_none());
    }
}
