//! The MPEG-AAC detector: does this signal sit on an AAC quantization
//! lattice?
//!
//! Implements §1.4 of the paper. The shape of the search is a stack of loops,
//! and every level exists because the detector does not know something the
//! encoder did:
//!
//! | Parameter to infer | Search strategy |
//! |---|---|
//! | where the encoder's frames began | try all 1024 sample alignments |
//! | which window shape each frame used | try all four, keep the best |
//! | what scalefactor each band carried | try a grid of them |
//! | which stereo mode was coded | try L/R and M/S, keep the best |
//!
//! The best shape's counts are pooled across frames; the final score is
//! maximized over alignments and the selected stereo signals. This follows
//! the MATLAB aggregation, whose relation to the paper's pseudocode still
//! needs author confirmation. A maximum does not rule out false positives.
//!
//! This file is one algorithm read top to bottom — the sweep, the per-frame
//! measurement it drives, and the buffers both need. Splitting it would put
//! the loop in one file and the thing it loops over in another, which is a
//! split by layer, not by responsibility.

use crate::transcode::aac_tables::{
    long_bands, short_bands, Windows, LONG_LEN, SF_BIAS, SHORT_COUNT, SHORT_LEN, SHORT_OFFSET,
};
use crate::transcode::criterion::{
    scalefactor_grid, subband_thresholds, PreparedSubband, TARGET_PROBABILITY,
};
use crate::transcode::frames::{frame_energies_db, mean_energy_db, mid_side, select_frames};
use crate::transcode::mdct::Mdct;
use crate::transcode::{supported_rate_khz, Band, TranscodeEvidence};

/// Search settings.
///
/// The defaults are the paper's own: `δ ∈ [0.3, 0.7]` from §2, and 64 frames
/// by 64 scalefactors, the most thorough setting it evaluates.
#[derive(Debug, Clone, Copy)]
pub struct AacParams {
    /// `Nf` — how many high-energy frames to measure.
    pub frames: usize,
    /// `Nsf` — how many scalefactors to try per band.
    pub scalefactors: usize,
    /// Lower relative scalefactor bound `δmin`.
    pub delta_min: f64,
    /// Upper relative scalefactor bound `δmax`.
    pub delta_max: f64,
    /// `λ` — the likelihood above which the verdict is "transcoded".
    ///
    /// This pairs with `frames`/`scalefactors` and cannot be moved
    /// independently of them. These thresholds were calibrated empirically
    /// for each complete configuration. The author's
    /// reference lists 0.031 for 8×8, 0.019 for 16×16, 0.0145 for 32×32 and
    /// 0.0125 for 64×64.
    pub significance: f64,
    /// How many sample alignments to try, from 0 upwards.
    ///
    /// Always [`LONG_LEN`] in production — the encoder's frame boundary can
    /// sit anywhere in a transform's span, and testing fewer alignments means
    /// simply missing the files whose boundary falls outside the range. It is
    /// a field only so the unit tests can exercise the plumbing without
    /// running the full sweep, which takes minutes by design.
    pub alignments: usize,
}

impl Default for AacParams {
    fn default() -> Self {
        Self {
            frames: 64,
            scalefactors: 64,
            delta_min: 0.3,
            delta_max: 0.7,
            significance: 0.0125,
            alignments: LONG_LEN,
        }
    }
}

/// Run the detector over one file's decoded channels.
///
/// `channels` holds one slice per channel. Mono is analyzed directly; stereo
/// is analyzed as L/R and as M/S and the higher score wins, because an
/// encoder chooses between those two modes per frame and nothing in the
/// decoded signal records which it picked. More than two channels: the first
/// two are used, which is where a stereo master's content lives.
///
/// `cancelled` is polled once per alignment so a long sweep can be stopped.
/// Returns `None` when it fires, when the file cannot supply all requested frames,
/// or when its sample rate has no tabulated bands.
pub fn detect(
    channels: &[Vec<f64>],
    sample_rate: u32,
    params: &AacParams,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Option<TranscodeEvidence> {
    let rate_khz = supported_rate_khz(sample_rate)?;
    let bands = BandSet::new(rate_khz, params)?;
    let first = channels.first()?;

    let mut best = TranscodeEvidence {
        likelihood: 0.0,
        offset: 0,
        detected: false,
    };

    if channels.len() == 1 {
        let picked = select_frames(first, LONG_LEN, 2, params.frames)?;
        best = sweep(first, &picked, &bands, params, cancelled)?;
    } else {
        let left = first.as_slice();
        let right = channels[1].as_slice();
        // AAC's stereo matrix is unscaled: M = L + R, S = L − R.
        let (mid, side) = mid_side(left, right, 1.0);

        // Frames are chosen on the mid channel and then reused to compare the
        // four signals, so all four are judged over the same stretches of
        // music rather than each over its own loudest moments.
        let picked = select_frames(&mid, LONG_LEN, 2, params.frames)?;
        let mean_of = |x: &[f64]| mean_energy_db(&frame_energies_db(x, LONG_LEN), &picked);

        // Only the louder of each pair is swept: the quieter one carries the
        // same lattice more faintly, so it costs a full sweep to learn
        // nothing new.
        let lr: &[f64] = if mean_of(left) >= mean_of(right) {
            left
        } else {
            right
        };
        let ms: &[f64] = if mean_of(&mid) >= mean_of(&side) {
            &mid
        } else {
            &side
        };

        for signal in [lr, ms] {
            let evidence = sweep(signal, &picked, &bands, params, cancelled)?;
            if evidence.likelihood > best.likelihood {
                best = evidence;
            }
        }
    }

    best.detected = best.likelihood > params.significance;
    Some(best)
}

/// Band layouts and everything derived from them, built once per file.
struct BandSet {
    long: Vec<Band>,
    short: Vec<Band>,
    tau_long: Vec<f64>,
    tau_short: Vec<f64>,
    deltas: Vec<f64>,
}

impl BandSet {
    fn new(rate_khz: u32, params: &AacParams) -> Option<Self> {
        let long = long_bands(rate_khz)?;
        let short = short_bands(rate_khz)?;
        let widths_long: Vec<usize> = long.iter().map(|b| b.width()).collect();
        // A short band is measured across all eight short transforms at once,
        // so its statistic averages over eight times as many coefficients and
        // its threshold has to be computed for that width, not the band's.
        let widths_short: Vec<usize> = short.iter().map(|b| b.width() * SHORT_COUNT).collect();
        Some(Self {
            tau_long: subband_thresholds(&widths_long, TARGET_PROBABILITY),
            tau_short: subband_thresholds(&widths_short, TARGET_PROBABILITY),
            long,
            short,
            deltas: scalefactor_grid(params.delta_min, params.delta_max, params.scalefactors),
        })
    }
}

/// Running best across the four window shapes of one frame.
///
/// A plain struct rather than a closure over local variables: the comparison
/// has to read and write the same state the caller keeps afterwards, and a
/// closure capturing it mutably would lock the caller out of it.
struct BestShape {
    hits: usize,
    trials: usize,
    ratio: f64,
}

impl BestShape {
    fn new() -> Self {
        // −1 so the first offer always wins, even one that scored zero.
        Self {
            hits: 0,
            trials: 0,
            ratio: -1.0,
        }
    }

    fn offer(&mut self, hits: usize, trials: usize) {
        let ratio = if trials == 0 {
            0.0
        } else {
            hits as f64 / trials as f64
        };
        if ratio > self.ratio {
            self.ratio = ratio;
            self.hits = hits;
            self.trials = trials;
        }
    }
}

/// Count on-grid trials across a spectrum's bands.
///
/// A free function taking `sub` separately rather than a method: the caller
/// holds both the scratch quantizer and the spectrum as fields of one struct,
/// and a method would borrow the whole struct twice over.
///
/// The topmost band is skipped, as in the reference. It is the wide remainder
/// up to Nyquist, and in a transcoded file it is precisely the region the
/// encoder threw away — near-silent, no lattice left in it, and its emptiness
/// would dilute the ratio identically for every file.
///
/// `trials` counts every scalefactor offered, including those in bands too
/// silent to test. That is deliberate and matches the reference: a file whose
/// bands are mostly empty should score *low*, and crediting it by shrinking
/// the denominator would do the opposite.
fn count_bands(
    sub: &mut PreparedSubband,
    spectrum: &[f64],
    bands: &[Band],
    taus: &[f64],
    deltas: &[f64],
) -> (usize, usize) {
    let mut hits = 0;
    let mut trials = 0;
    let n = bands.len().saturating_sub(1);
    for (b, &tau) in bands.iter().zip(taus).take(n) {
        let Some(slice) = spectrum.get(b.start..b.end) else {
            continue;
        };
        trials += deltas.len();
        if sub.prepare(slice, SF_BIAS) {
            hits += sub.on_grid_count(deltas, SF_BIAS, tau);
        }
    }
    (hits, trials)
}

/// Everything one thread needs, allocated once and reused for every alignment
/// it handles.
///
/// A sweep runs on the order of a million transforms; allocating per
/// transform would cost more than the transforms do. The long and short
/// windowing buffers are separate fields rather than one shared buffer so
/// that borrowing one never conflicts with borrowing the other.
struct Worker {
    mdct_long: Mdct,
    mdct_short: Mdct,
    windowed_long: Vec<f64>,
    windowed_short: Vec<f64>,
    spectrum: Vec<f64>,
    short_spectra: Vec<f64>,
    gather: Vec<f64>,
    sub: PreparedSubband,
}

impl Worker {
    fn new() -> Self {
        Self {
            mdct_long: Mdct::new(LONG_LEN),
            mdct_short: Mdct::new(SHORT_LEN),
            windowed_long: vec![0.0; 2 * LONG_LEN],
            windowed_short: vec![0.0; 2 * SHORT_LEN],
            spectrum: vec![0.0; LONG_LEN],
            short_spectra: vec![0.0; SHORT_COUNT * SHORT_LEN],
            gather: vec![0.0; SHORT_COUNT * SHORT_LEN],
            sub: PreparedSubband::new(),
        }
    }

    /// Measure one 2048-sample frame under all four window shapes and return
    /// the best as a `(hits, trials)` pair.
    fn measure_frame(&mut self, win: &[f64], bands: &BandSet, w: &Windows) -> (usize, usize) {
        let mut best = BestShape::new();

        for shape in [&w.long, &w.start, &w.stop] {
            for (dst, (s, sh)) in self
                .windowed_long
                .iter_mut()
                .zip(win.iter().zip(shape.iter()))
            {
                *dst = s * sh;
            }
            if !self
                .mdct_long
                .transform(&self.windowed_long, &mut self.spectrum)
            {
                continue;
            }
            let (h, t) = count_bands(
                &mut self.sub,
                &self.spectrum,
                &bands.long,
                &bands.tau_long,
                &bands.deltas,
            );
            best.offer(h, t);
        }

        if self.transform_short_block(win, w) {
            let (h, t) = self.count_short_bands(bands);
            best.offer(h, t);
        }

        (best.hits, best.trials)
    }

    /// The eight short transforms of one frame, written into
    /// [`Worker::short_spectra`]. Returns `false` if the frame is too short
    /// to hold them, which leaves the short shape simply unmeasured.
    fn transform_short_block(&mut self, win: &[f64], w: &Windows) -> bool {
        for k in 0..SHORT_COUNT {
            let from = k * SHORT_LEN + SHORT_OFFSET;
            let Some(src) = win.get(from..from + 2 * SHORT_LEN) else {
                return false;
            };
            for (dst, (s, sh)) in self
                .windowed_short
                .iter_mut()
                .zip(src.iter().zip(w.short.iter()))
            {
                *dst = s * sh;
            }
            let out = &mut self.short_spectra[k * SHORT_LEN..(k + 1) * SHORT_LEN];
            if !self.mdct_short.transform(&self.windowed_short, out) {
                return false;
            }
        }
        true
    }

    /// One short band spans the same frequency range in all eight transforms.
    /// The reference measures those eight stretches as a single population
    /// rather than as eight small ones — which is why they are gathered into
    /// a contiguous buffer here before being handed to the quantizer.
    fn count_short_bands(&mut self, bands: &BandSet) -> (usize, usize) {
        let mut hits = 0;
        let mut trials = 0;
        let n = bands.short.len().saturating_sub(1);

        for (b, &tau) in bands.short.iter().zip(&bands.tau_short).take(n) {
            let width = b.width();
            let mut filled = 0;
            for k in 0..SHORT_COUNT {
                let base = k * SHORT_LEN;
                if base + b.end > self.short_spectra.len() || filled + width > self.gather.len() {
                    break;
                }
                self.gather[filled..filled + width]
                    .copy_from_slice(&self.short_spectra[base + b.start..base + b.end]);
                filled += width;
            }
            trials += bands.deltas.len();
            if self.sub.prepare(&self.gather[..filled], SF_BIAS) {
                hits += self.sub.on_grid_count(&bands.deltas, SF_BIAS, tau);
            }
        }
        (hits, trials)
    }
}

/// Sweep every alignment of one signal, returning the best likelihood found.
///
/// Alignments are split across threads because they are perfectly
/// independent — each one re-measures the same frames from a different
/// starting sample and shares nothing with its neighbours. `std::thread`
/// rather than a work-stealing pool: the work is uniform, so a fixed split is
/// as good as any, and it keeps the crate free of a parallelism dependency.
fn sweep(
    signal: &[f64],
    picked: &[usize],
    bands: &BandSet,
    params: &AacParams,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Option<TranscodeEvidence> {
    if picked.len() != params.frames || picked.is_empty() || bands.deltas.is_empty() {
        return None;
    }

    let alignments = params.alignments.clamp(1, LONG_LEN);
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(alignments);
    let per = alignments.div_ceil(threads);

    let results: Vec<Option<(f64, usize)>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let lo = t * per;
                let hi = ((t + 1) * per).min(alignments);
                scope.spawn(move || {
                    let windows = Windows::new();
                    let mut worker = Worker::new();
                    let mut best = (0.0f64, lo);
                    for offset in lo..hi {
                        if cancelled() {
                            return None;
                        }
                        let mut hits = 0usize;
                        let mut trials = 0usize;
                        for &f in picked {
                            let start = f * LONG_LEN + offset;
                            let win = signal.get(start..start + 2 * LONG_LEN)?;
                            let (h, t) = worker.measure_frame(win, bands, &windows);
                            hits += h;
                            trials += t;
                        }
                        let ratio = if trials == 0 {
                            0.0
                        } else {
                            hits as f64 / trials as f64
                        };
                        if ratio > best.0 {
                            best = (ratio, offset);
                        }
                    }
                    Some(best)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| match h.join() {
                Ok(v) => v,
                // A worker panicked. `.ok()` used to turn that into `None`,
                // which the caller reads as "no answer" and the UI as
                // **clean** — a crash silently rebranded as a verdict, with
                // nothing printed anywhere. Re-raising it makes the bug
                // impossible to miss instead of impossible to see. This is a
                // programming fault by construction: every index in the sweep
                // is derived from constants, never from the file.
                Err(payload) => std::panic::resume_unwind(payload),
            })
            .collect()
    });

    let mut best = (0.0f64, 0usize);
    for result in results {
        let (ratio, offset) = result?;
        if ratio > best.0 {
            best = (ratio, offset);
        }
    }
    Some(TranscodeEvidence {
        likelihood: best.0,
        offset: best.1,
        detected: best.0 > params.significance,
    })
}

#[cfg(test)]
#[path = "../../tests/unit/transcode/aac.rs"]
mod tests;
