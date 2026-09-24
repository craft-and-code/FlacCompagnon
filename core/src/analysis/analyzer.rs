//! Streaming analyzer.
//!
//! The decoder feeds audio one frame at a time (a frame = one sample per
//! channel). Storing an entire album in memory would be wasteful, so every
//! metric is accumulated incrementally:
//!
//! * spectrum      -> Hann-windowed FFT of a mono downmix, averaged over windows
//! * clipping      -> full-scale sample counting with run detection
//! * stereo        -> L-R difference, L/R correlation and mono cancellation
//! * loudness      -> K-weighted integrated LUFS and loudness range
//! * real bitdepth -> exact unused bits and persistent lower-depth integer grids
//!
//! Nothing here depends on a specific file format; [`decode`](crate::decode)
//! adapts each codec to the [`StreamAnalyzer::push_frame`] interface.
//!
//! # Size
//!
//! Over CLAUDE.md's 300-line ceiling, deliberately. Each metric already lives
//! in its own module ([`spectrum`], [`clipping`], [`stereo`](super::stereo),
//! [`bitdepth`], [`loudness`](super::loudness), [`mdct`](super::mdct)); what is left here is the single hot
//! loop that feeds them all from one pass over the samples,
//! plus the state that loop carries. That single pass *is* the design — the
//! whole reason this type exists rather than five independent analyzers is
//! that an album must be read once, not five times. Splitting the loop would
//! hide exactly the thing a reviewer needs to see whole, and any split would
//! be by metric, which is the axis already covered by those modules.

use std::sync::Arc;

use rustfft::{num_complex::Complex, Fft, FftPlanner};

use super::dc_offset::{DcOffset, DcOffsetMeter};
use super::discontinuities::{DiscontinuityAnalysis, DiscontinuityDetector};
use super::intensity_stereo::{HighFrequencyStereo, HighFrequencyStereoMeter};
use super::local_phase::{LocalPhase, LocalPhaseMeter};
use super::loudness_peaks::LoudnessPeaks;
use super::mdct::{Mdct, AAC_N};
use super::truepeak::TruePeak;
use super::{bitdepth, clipping, loudness::LoudnessMeter, spectrum};
use crate::ClippingInfo;

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
    /// Estimated frequency-response cutoff, in Hz.
    pub cutoff_hz: f64,
    /// [`cutoff_hz`](Self::cutoff_hz) as a fraction of Nyquist (0..1).
    pub cutoff_ratio: f64,
    /// How sharply the level drops at the cutoff (dB). Large == a brick wall.
    pub cliff_db: f32,
    /// Mean level just above the cutoff (dB rel. peak) — the dead-zone depth.
    pub above_db: f32,
    /// Averaged magnitude spectrum in dB, one entry per FFT bin (0..=N/2).
    pub spectrum_db: Vec<f32>,
    /// Clipping / true-peak-ish statistics accumulated over the stream.
    pub clipping: ClippingInfo,
    /// `true` when the left and right channels were identical (or near
    /// enough) for long enough to suggest a mono source duplicated to stereo.
    pub fake_stereo: bool,
    /// Whole-stream correlation of the first two channels when both carry audio.
    pub phase_correlation: Option<f32>,
    /// True when the two channels are strongly opposed across the stream.
    pub phase_inverted: bool,
    /// Unweighted RMS balance, available only for two-channel streams.
    pub stereo_balance: Option<super::stereo::StereoBalance>,
    /// High-frequency Side/Mid measurement for qualifying stereo streams.
    pub high_frequency_stereo: Option<HighFrequencyStereo>,
    /// Whole-stream mean measured independently on each channel.
    pub dc_offset: Option<DcOffset>,
    /// Time-local correlation and frequency-band evidence for stereo streams.
    pub local_phase: Option<LocalPhase>,
    /// The bit depth actually used by the samples, when it could be
    /// determined from an integer PCM source (`None` for float sources).
    pub real_bit_depth: Option<u32>,
    /// Exact occupied depth and whether the effective depth is a grid estimate.
    pub bit_depth_evidence: Option<bitdepth::BitDepthEvidence>,

    /// Dynamic-range estimate in dB: peak level vs the RMS of the loudest 20%
    /// of ~3 s blocks (crest factor of the loud passages, DR-meter style).
    /// High values (>= 12 dB) indicate a dynamic master; low values (< 8 dB)
    /// a loudness-war master. `None` for silent or extremely short streams.
    pub dr_db: Option<f32>,
    /// EBU R 128 integrated loudness, in LUFS; absent when unmeasurable.
    pub integrated_lufs: Option<f32>,
    /// Maximum momentary and short-term loudness over real-audio windows.
    pub loudness_peaks: Option<LoudnessPeaks>,
    /// EBU Tech 3342 loudness range, in LU; absent when unmeasurable.
    pub loudness_range_lu: Option<f32>,
    /// Suspected short pulses and digital dropouts; absent if unmeasurable.
    pub discontinuities: Option<DiscontinuityAnalysis>,

    // --- MDCT (AAC-SIN) transcode evidence ---
    /// Mean per-frame MDCT cutoff as a fraction of Nyquist (dead-zone frames).
    pub mdct_cutoff_ratio: Option<f64>,
    /// Mean level (dB rel. frame peak) of the MDCT dead zone above the cutoff.
    pub mdct_dead_db: Option<f32>,
    /// Fraction of analyzed MDCT frames that showed a high-frequency dead zone.
    pub mdct_dead_fraction: Option<f32>,

    /// Legacy detector result, retained for API compatibility. Always `None`;
    /// transcoding is now evaluated by `crate::transcode` in the pipeline.
    pub requant_rate: Option<f32>,
}

impl AnalysisSummary {
    /// The FFT size [`spectrum_db`](Self::spectrum_db) was produced with.
    ///
    /// The spectrum holds bins `0..=N/2` (a real FFT's non-redundant half), so
    /// `N` is recovered as `(len - 1) * 2`. Callers need it to turn a bin index
    /// back into a frequency; it lives here rather than being re-derived at
    /// each call site, where the off-by-one is easy to get wrong and silently
    /// shifts every frequency band that depends on it.
    pub fn fft_size(&self) -> usize {
        self.spectrum_db.len().saturating_sub(1) * 2
    }
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
    loudness: Option<LoudnessMeter>,
    discontinuities: Option<DiscontinuityDetector>,
    high_frequency_stereo: Option<HighFrequencyStereoMeter>,
    dc_offset: Option<DcOffsetMeter>,
    local_phase: Option<LocalPhaseMeter>,

    // --- stereo relationship ---
    diff_energy: f64,
    l_energy: f64,
    r_energy: f64,
    cross_energy: f64,
    identical_frames: u64,
    total_frames: u64,

    // --- bit depth ---
    bit_depth: bitdepth::BitDepthAnalyzer,

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

    // --- dynamics (DR) ---
    dyn_block_sumsq: f64, // running sum of per-frame mean squares
    dyn_block_frames: u64,
    dyn_blocks: Vec<f64>, // mean square of each completed block
}

impl StreamAnalyzer {
    /// Start a fresh analysis for a stream with `channels` audio channels.
    pub fn new(channels: usize, sample_rate: u32) -> Self {
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
            loudness: LoudnessMeter::new(sample_rate, channels),
            discontinuities: DiscontinuityDetector::new(sample_rate, channels),
            high_frequency_stereo: HighFrequencyStereoMeter::new(sample_rate, channels),
            dc_offset: DcOffsetMeter::new(channels),
            local_phase: LocalPhaseMeter::new(sample_rate, channels),
            diff_energy: 0.0,
            l_energy: 0.0,
            r_energy: 0.0,
            cross_energy: 0.0,
            identical_frames: 0,
            total_frames: 0,
            bit_depth: bitdepth::BitDepthAnalyzer::new(channels.max(1)),
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
            dyn_block_sumsq: 0.0,
            dyn_block_frames: 0,
            dyn_blocks: Vec::new(),
        }
    }

    /// Push one frame of normalized-float samples (`samples.len() == channels`),
    /// optionally accompanied by the raw integer sample values for the same
    /// frame (used for effective bit-depth estimation).
    pub fn push_frame(&mut self, samples: &[f32], int_samples: Option<&[i32]>) {
        if let Some(loudness) = &mut self.loudness {
            loudness.push_frame(samples);
        }
        if let Some(phase) = &mut self.local_phase {
            phase.push_frame(samples);
        }
        if let Some(dc_offset) = &mut self.dc_offset {
            dc_offset.push_frame(samples);
        }
        if samples.is_empty() {
            return;
        }
        self.total_frames += 1;

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
        if let Some(discontinuities) = &mut self.discontinuities {
            discontinuities.push_frame(samples);
        }
        if let Some(high_frequency_stereo) = &mut self.high_frequency_stereo {
            high_frequency_stereo.push_frame(samples);
        }
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
            self.cross_energy += l * r;
            if (l - r).abs() < 1e-9 {
                self.identical_frames += 1;
            }
        }

        self.bit_depth.push_frame(int_samples);

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
                && self.mdct_hop.is_multiple_of(MDCT_STRIDE)
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
            .map(|(&s, &w)| Complex { re: s * w, im: 0.0 })
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
            super::stereo::is_fake(
                self.diff_energy,
                self.l_energy,
                self.r_energy,
                self.identical_frames,
                self.total_frames,
            )
        } else {
            false
        };
        let phase = super::stereo::analyze_phase(self.l_energy, self.r_energy, self.cross_energy);
        let high_frequency_stereo = self
            .high_frequency_stereo
            .take()
            .and_then(HighFrequencyStereoMeter::finish);

        let measured_depth = self.bit_depth.finish(declared_bits);
        let real_bit_depth = measured_depth.map(|(bits, _)| bits);
        let bit_depth_evidence = measured_depth.map(|(_, evidence)| evidence);

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

        // Capture programme loudness and M/S maxima before LRA appends 1.5 s of
        // analysis-only silence for its centred 3 s tail windows.
        let integrated_lufs = self
            .loudness
            .as_ref()
            .and_then(LoudnessMeter::integrated_lufs);
        let loudness_peaks = self.loudness.as_ref().and_then(LoudnessMeter::peaks);
        let loudness_range_lu = self
            .loudness
            .take()
            .and_then(LoudnessMeter::loudness_range_lu);

        AnalysisSummary {
            cutoff_hz,
            cutoff_ratio,
            cliff_db,
            above_db,
            spectrum_db,
            clipping,
            fake_stereo,
            phase_correlation: phase.correlation,
            phase_inverted: phase.likely_inverted,
            stereo_balance: if self.channels == 2 {
                super::stereo::analyze_balance(self.l_energy, self.r_energy)
            } else {
                None
            },
            high_frequency_stereo,
            dc_offset: self.dc_offset.take().and_then(DcOffsetMeter::finish),
            local_phase: self.local_phase.take().and_then(LocalPhaseMeter::finish),
            real_bit_depth,
            bit_depth_evidence,
            dr_db,
            integrated_lufs,
            loudness_peaks,
            loudness_range_lu,
            discontinuities: self
                .discontinuities
                .take()
                .and_then(DiscontinuityDetector::finish),
            mdct_cutoff_ratio,
            mdct_dead_db,
            mdct_dead_fraction,
            requant_rate: None,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/analysis/analyzer.rs"]
mod tests;
