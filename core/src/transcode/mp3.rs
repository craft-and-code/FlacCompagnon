//! The MP3 detector: does this signal sit on an MPEG-1 Layer III lattice?
//!
//! Same search as [`super::aac`] — every alignment, every window shape, a grid
//! of scalefactors, both stereo modes, keep the best — over a completely
//! different transform. See [`super::pqmf`] for the first stage and
//! [`super::mp3_tables`] for why all three are needed.
//!
//! # What differs from AAC, beyond the filterbank
//!
//! | | AAC | MP3 |
//! |---|---|---|
//! | alignments | 1024 | 576 (one granule) |
//! | scalefactor bias | 60 | 80 |
//! | δ range | 0.3 … 0.7 | 0.5 … 0.7 |
//! | stereo matrix | `L ± R` | `½(L ± R)` |
//! | top long band | skipped | **measured** |
//!
//! None of those are free choices. The bias and the matrix come from the
//! standards; the δ range and the significance threshold were tuned by the
//! author for this codec; and the last long band is measured here because
//! MP3's band table already stops well below the full spectrum — there is no
//! wide empty remainder at the top to skip, unlike AAC's.
//!
//! # Why this file is over the size ceiling
//!
//! It is one algorithm, and the pieces only mean anything in sequence: the
//! sweep drives a per-frame transform, that transform is three chained stages
//! whose intermediate layout the band-counting then depends on, and all of it
//! shares one set of scratch buffers sized from the same constants. Splitting
//! it would put the loop in one file and the thing it loops over in another
//! — a split by layer, not by responsibility — and the reader would have to
//! hold the interleaved short-transform layout in their head across a file
//! boundary to see why a band is a contiguous slice.
//!
//! What *was* separable has been separated: the window shapes live in
//! [`super::mp3_tables`] with the other constants of the standard, and the
//! filterbank in [`super::pqmf`].

use super::criterion::{scalefactor_grid, subband_thresholds, PreparedSubband, TARGET_PROBABILITY};
// No `mean_energy_db` here, unlike the AAC path: that one compares the two
// signals of each stereo pair and sweeps only the louder. This one sweeps all
// four, so there is nothing to compare.
use super::frames::{mid_side, select_frames};
use super::mdct::Mdct;
use super::mp3_tables::{
    butterfly_coefficients, long_bands, short_bands, Windows, GRANULE_LEN, GRANULE_SIZE, N_LONG,
    N_SHORT, PQMF_BANDS, SF_BIAS, SHORT_COUNT, SHORT_OFFSET,
};
use super::pqmf::{flip_odd_subbands, Pqmf};
use super::{supported_rate_khz, Band, TranscodeEvidence};

/// Subband samples spanning the two granules one frame analyses: 18 + 18.
const STEPS: usize = 2 * GRANULE_SIZE;
/// Coefficients each short transform produces.
const SHORT_OUT: usize = N_SHORT / 2;

/// Search settings, defaulting to the author's own for this codec.
#[derive(Debug, Clone, Copy)]
pub struct Mp3Params {
    /// `Nf` — high-energy frames to measure.
    pub frames: usize,
    /// `Nsf` — scalefactors tried per band.
    pub scalefactors: usize,
    /// Lower relative scalefactor bound. 0.5 for MP3, against AAC's 0.3.
    pub delta_min: f64,
    /// Upper relative scalefactor bound.
    pub delta_max: f64,
    /// `λ` — the likelihood above which the verdict is "transcoded". Pairs
    /// with `frames`/`scalefactors` and cannot be moved independently of them.
    ///
    /// The MATLAB MP3 reference uses 0.025. This port retains its provisional
    /// 0.031 setting until a broader MP3 calibration is available. The
    /// 0.025–0.032 range in the JAES article concerns AAC, not MP3; it does
    /// not validate this choice. Raising the threshold trades sensitivity
    /// for fewer positives, and the dead-zone guard needs validation too.
    pub significance: f64,
    /// Sample alignments to try. Always [`GRANULE_LEN`] in production; a
    /// field only so tests can run without the full sweep.
    pub alignments: usize,
}

impl Default for Mp3Params {
    fn default() -> Self {
        Self {
            frames: 8,
            scalefactors: 8,
            delta_min: 0.5,
            delta_max: 0.7,
            significance: 0.031,
            alignments: GRANULE_LEN,
        }
    }
}

/// Run the detector over one file's decoded channels.
///
/// Unlike the AAC path, which picks the louder of each stereo pair, this
/// sweeps all four signals (L, R, M, S) and keeps the best — the reference
/// does the same for this codec, and MP3's halved matrix makes M and S
/// quieter than their AAC counterparts, so discarding one on energy alone is
/// less safe here.
pub fn detect(
    channels: &[Vec<f64>],
    sample_rate: u32,
    params: &Mp3Params,
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
        let picked = select_frames(first, GRANULE_LEN, 3, params.frames)?;
        best = sweep(first, &picked, &bands, params, cancelled)?;
    } else {
        let left = first.as_slice();
        let right = channels[1].as_slice();
        // Layer III's matrix is halved: M = ½(L + R), S = ½(L − R).
        let (mid, side) = mid_side(left, right, 0.5);
        let picked = select_frames(&mid, GRANULE_LEN, 3, params.frames)?;
        for signal in [left, right, mid.as_slice(), side.as_slice()] {
            let evidence = sweep(signal, &picked, &bands, params, cancelled)?;
            if evidence.likelihood > best.likelihood {
                best = evidence;
            }
        }
    }

    best.detected = best.likelihood > params.significance;
    Some(best)
}

/// Band layouts, thresholds and the scalefactor grid, built once per file.
struct BandSet {
    long: Vec<Band>,
    short: Vec<Band>,
    tau_long: Vec<f64>,
    tau_short: Vec<f64>,
    deltas: Vec<f64>,
}

impl BandSet {
    fn new(rate_khz: u32, params: &Mp3Params) -> Option<Self> {
        let long = long_bands(rate_khz)?;
        let short = short_bands(rate_khz)?;
        let widths_long: Vec<usize> = long.iter().map(|b| b.width()).collect();
        // As in AAC: a short band is one population across all three short
        // transforms, so its threshold is computed for the wider count.
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

/// Running best across one frame's four window shapes.
struct BestShape {
    hits: usize,
    trials: usize,
    ratio: f64,
}

impl BestShape {
    fn new() -> Self {
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

/// Per-thread scratch, allocated once.
struct Worker {
    pqmf: Pqmf,
    mdct_long: Mdct,
    mdct_short: Mdct,
    /// 32 × 36 subband samples, band-major.
    subband: Vec<f64>,
    /// One granule's worth of PQMF output, band-major.
    granule: Vec<f64>,
    windowed: Vec<f64>,
    spectrum: Vec<f64>,
    scratch: Vec<f64>,
    sub: PreparedSubband,
}

impl Worker {
    fn new() -> Self {
        Self {
            pqmf: Pqmf::new(),
            mdct_long: Mdct::new(N_LONG / 2),
            mdct_short: Mdct::new(SHORT_OUT),
            subband: vec![0.0; PQMF_BANDS * STEPS],
            granule: vec![0.0; PQMF_BANDS * GRANULE_SIZE],
            windowed: vec![0.0; N_LONG],
            spectrum: vec![0.0; GRANULE_LEN],
            scratch: vec![0.0; GRANULE_LEN],
            sub: PreparedSubband::new(),
        }
    }

    /// Fill [`Worker::subband`] from three consecutive granules.
    fn analyse(&mut self, g1: &[f64], g2: &[f64], g3: &[f64]) -> bool {
        for (which, (prev, cur)) in [(g1, g2), (g2, g3)].into_iter().enumerate() {
            if !self.pqmf.granule(prev, cur, &mut self.granule) {
                return false;
            }
            for band in 0..PQMF_BANDS {
                let src = &self.granule[band * GRANULE_SIZE..(band + 1) * GRANULE_SIZE];
                let at = band * STEPS + which * GRANULE_SIZE;
                self.subband[at..at + GRANULE_SIZE].copy_from_slice(src);
            }
        }
        flip_odd_subbands(&mut self.subband, STEPS);
        true
    }

    /// One long-family shape: an 18-point MDCT per subband, laid out in
    /// frequency order.
    fn long_shape(&mut self, window: &[f64]) -> bool {
        for band in 0..PQMF_BANDS {
            let src = &self.subband[band * STEPS..band * STEPS + N_LONG];
            for (d, (s, w)) in self.windowed.iter_mut().zip(src.iter().zip(window)) {
                *d = s * w;
            }
            let out = &mut self.spectrum[band * (N_LONG / 2)..(band + 1) * (N_LONG / 2)];
            if !self.mdct_long.transform(&self.windowed, out) {
                return false;
            }
        }
        true
    }

    /// The short shape: three 6-coefficient transforms per subband.
    ///
    /// They are stored **interleaved by transform** — `spectrum[c * 3 + w]`
    /// holds coefficient `c` of short window `w`. That is not a stylistic
    /// choice: it is the layout the reference's column-major reshape
    /// produces, and it is what makes a scalefactor band contiguous
    /// afterwards. A band spanning coefficients `s..e` across all three
    /// transforms is exactly the slice `spectrum[s * 3 .. e * 3]`, which is
    /// why [`Worker::count_short_bands`] needs no gather buffer at all.
    fn short_shape(&mut self, window: &[f64]) -> bool {
        for band in 0..PQMF_BANDS {
            for w in 0..SHORT_COUNT {
                let from = band * STEPS + w * SHORT_OUT + SHORT_OFFSET;
                let Some(src) = self.subband.get(from..from + N_SHORT) else {
                    return false;
                };
                let buf = &mut self.windowed[..N_SHORT];
                for (d, (s, win)) in buf.iter_mut().zip(src.iter().zip(window)) {
                    *d = s * win;
                }
                if !self
                    .mdct_short
                    .transform(&self.windowed[..N_SHORT], &mut self.scratch[..SHORT_OUT])
                {
                    return false;
                }
                for c in 0..SHORT_OUT {
                    self.spectrum[(band * SHORT_OUT + c) * SHORT_COUNT + w] = self.scratch[c];
                }
            }
        }
        true
    }

    /// Alias-reduction butterflies across every subband boundary.
    ///
    /// Not optional cosmetics: the encoder applies these, so the lattice
    /// exists only after them. Skipping the stage leaves coefficients of
    /// plausible magnitude sitting nowhere in particular, and the detector
    /// would report every file clean without anything looking wrong.
    fn butterflies(&mut self) {
        let (cs, ca) = butterfly_coefficients();
        let half = N_LONG / 2;
        self.scratch.copy_from_slice(&self.spectrum);
        for sb in 1..PQMF_BANDS {
            for i in 0..cs.len() {
                let lo = sb * half - i - 1;
                let hi = sb * half + i;
                let (a, b) = (self.scratch[lo], self.scratch[hi]);
                self.spectrum[lo] = cs[i] * a - ca[i] * b;
                self.spectrum[hi] = cs[i] * b + ca[i] * a;
            }
        }
    }

    /// Count on-grid trials over the long bands. **All** of them, including
    /// the topmost — see this module's header.
    fn count_long_bands(&mut self, bands: &BandSet) -> (usize, usize) {
        let mut hits = 0;
        let mut trials = 0;
        for (b, &tau) in bands.long.iter().zip(&bands.tau_long) {
            let Some(slice) = self.spectrum.get(b.start..b.end) else {
                continue;
            };
            trials += bands.deltas.len();
            if self.sub.prepare(slice, SF_BIAS) {
                hits += self.sub.on_grid_count(&bands.deltas, SF_BIAS, tau);
            }
        }
        (hits, trials)
    }

    /// Count on-grid trials over the short bands, skipping the topmost.
    fn count_short_bands(&mut self, bands: &BandSet) -> (usize, usize) {
        let mut hits = 0;
        let mut trials = 0;
        let n = bands.short.len().saturating_sub(1);
        for (b, &tau) in bands.short.iter().zip(&bands.tau_short).take(n) {
            // Contiguous thanks to the interleaved layout — see `short_shape`.
            let Some(slice) = self
                .spectrum
                .get(b.start * SHORT_COUNT..b.end * SHORT_COUNT)
            else {
                continue;
            };
            trials += bands.deltas.len();
            if self.sub.prepare(slice, SF_BIAS) {
                hits += self.sub.on_grid_count(&bands.deltas, SF_BIAS, tau);
            }
        }
        (hits, trials)
    }

    /// Measure one frame under all four shapes, best wins.
    fn measure(
        &mut self,
        g1: &[f64],
        g2: &[f64],
        g3: &[f64],
        bands: &BandSet,
        w: &Windows,
    ) -> (usize, usize) {
        if !self.analyse(g1, g2, g3) {
            return (0, 0);
        }
        let mut best = BestShape::new();
        for shape in [&w.long, &w.start, &w.stop] {
            if self.long_shape(shape) {
                self.butterflies();
                let (h, t) = self.count_long_bands(bands);
                best.offer(h, t);
            }
        }
        if self.short_shape(&w.short) {
            self.butterflies();
            let (h, t) = self.count_short_bands(bands);
            best.offer(h, t);
        }
        (best.hits, best.trials)
    }
}

/// Sweep every alignment of one signal.
fn sweep(
    signal: &[f64],
    picked: &[usize],
    bands: &BandSet,
    params: &Mp3Params,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Option<TranscodeEvidence> {
    if picked.len() != params.frames || picked.is_empty() || bands.deltas.is_empty() {
        return None;
    }

    let alignments = params.alignments.clamp(1, GRANULE_LEN);
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
                            let at = f * GRANULE_LEN + offset;
                            let three = signal.get(at..at + 3 * GRANULE_LEN)?;
                            let (h, t) = worker.measure(
                                &three[..GRANULE_LEN],
                                &three[GRANULE_LEN..2 * GRANULE_LEN],
                                &three[2 * GRANULE_LEN..],
                                bands,
                                &windows,
                            );
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
mod tests {
    use super::*;

    fn params_fast() -> Mp3Params {
        // `alignments` is the one parameter safe to reduce alone: fewer
        // alignments can only lower the score, never raise it, so a clean
        // verdict stays clean. `frames`/`scalefactors`/`significance` are left
        // at the 8×8 pairing, with this port's own 0.031 threshold — see
        // `Mp3Params::significance` for why it is not the reference's 0.025.
        Mp3Params {
            alignments: 12,
            ..Mp3Params::default()
        }
    }

    fn noise(n: usize) -> Vec<f64> {
        let mut state: u32 = 0x2545_F491;
        (0..n)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                (state as f64 / u32::MAX as f64) * 0.8 - 0.4
            })
            .collect()
    }

    #[test]
    fn the_windows_have_the_right_lengths_and_edges() {
        let w = Windows::new();
        assert_eq!(w.long.len(), N_LONG);
        assert_eq!(w.start.len(), N_LONG);
        assert_eq!(w.stop.len(), N_LONG);
        assert_eq!(w.short.len(), N_SHORT);
        // The transition shapes are asymmetric: `start` ends in silence,
        // `stop` begins in it.
        assert!(w.start[N_LONG - 1].abs() < 1e-12);
        assert!(w.stop[0].abs() < 1e-12);
        for (name, shape) in [("long", &w.long), ("start", &w.start), ("stop", &w.stop)] {
            assert!(
                shape.iter().all(|v| (0.0..=1.0).contains(v)),
                "{name} leaves [0,1]"
            );
        }
    }

    /// The layout claim `short_shape` relies on: one band across all three
    /// short transforms is a contiguous slice. Checked by filling the
    /// spectrum with its own indices and reading a band back.
    #[test]
    fn a_short_band_is_contiguous_in_the_interleaved_layout() {
        let mut worker = Worker::new();
        for (i, v) in worker.spectrum.iter_mut().enumerate() {
            *v = i as f64;
        }
        // Band covering coefficients 4..9 of each of the three transforms.
        let (s, e) = (4usize, 9usize);
        let slice = &worker.spectrum[s * SHORT_COUNT..e * SHORT_COUNT];
        assert_eq!(slice.len(), (e - s) * SHORT_COUNT);
        // Every (coefficient, transform) pair in the band, and nothing else.
        for c in s..e {
            for w in 0..SHORT_COUNT {
                assert!(
                    slice.contains(&((c * SHORT_COUNT + w) as f64)),
                    "c={c} w={w}"
                );
            }
        }
    }

    #[test]
    fn untabulated_rates_and_short_files_are_refused() {
        let ch = vec![noise(GRANULE_LEN * 12)];
        for rate in [96_000u32, 88_200, 22_050, 0] {
            assert!(
                detect(&ch, rate, &params_fast(), &|| false).is_none(),
                "{rate}"
            );
        }
        for n in [0usize, 1, GRANULE_LEN, GRANULE_LEN * 4] {
            assert!(
                detect(&[noise(n)], 44_100, &params_fast(), &|| false).is_none(),
                "n={n}"
            );
        }
        assert!(detect(&[], 44_100, &params_fast(), &|| false).is_none());
    }

    #[test]
    fn cancellation_stops_the_sweep() {
        let ch = vec![noise(GRANULE_LEN * 12)];
        assert!(detect(&ch, 44_100, &params_fast(), &|| true).is_none());
    }

    /// A golden value: the whole chain, end to end, against a number computed
    /// outside this codebase.
    ///
    /// Every other test here checks a property — "clean audio is not flagged",
    /// "the windows are 36 long". Properties are satisfied by a great many
    /// wrong implementations, and one of them bit hard: on a real 128 kbps
    /// transcode an independent reference measured 0.0796 while this code
    /// reported the file clean, with every property test green.
    ///
    /// So this one pins the *number*. The input is deterministic to the bit
    /// (xorshift32 from a fixed seed), the search is fixed at 12 alignments,
    /// and 0.020270 is what an independent implementation of the same chain
    /// produces on that exact input. A mismatch localises the fault
    /// immediately: the transform, the filterbank, the butterflies and the
    /// criterion all feed this single figure, and none of them can drift
    /// without moving it.
    #[test]
    fn the_whole_chain_reproduces_an_independently_computed_value() {
        let ch = vec![noise(GRANULE_LEN * 16)];
        let params = Mp3Params {
            alignments: 12,
            ..Mp3Params::default()
        };
        let e = detect(&ch, 44_100, &params, &|| false).expect("should run");
        assert!(
            (e.likelihood - 0.020_270).abs() < 5e-5,
            "expected 0.020270, got {:.6} — the chain has drifted",
            e.likelihood
        );
    }

    /// The false-positive direction, the one a user cannot forgive.
    #[test]
    fn lattice_free_audio_is_not_flagged() {
        let ch = vec![noise(GRANULE_LEN * 16)];
        let e = detect(&ch, 44_100, &params_fast(), &|| false).expect("should run");
        assert!(
            !e.detected,
            "noise scored {} against λ={}",
            e.likelihood,
            params_fast().significance
        );
        assert!((0.0..=1.0).contains(&e.likelihood));
    }

    #[test]
    fn silence_is_not_flagged() {
        let ch = vec![vec![0.0; GRANULE_LEN * 16]];
        let e = detect(&ch, 44_100, &params_fast(), &|| false).expect("should run");
        assert_eq!(e.likelihood, 0.0);
        assert!(!e.detected);
    }

    #[test]
    fn stereo_is_accepted() {
        let l = noise(GRANULE_LEN * 16);
        let r: Vec<f64> = l.iter().map(|v| v * 0.7).collect();
        let e = detect(&[l, r], 44_100, &params_fast(), &|| false).expect("stereo");
        assert!(!e.detected, "{}", e.likelihood);
    }
}
