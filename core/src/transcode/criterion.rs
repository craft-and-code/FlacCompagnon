//! The statistical detection criterion, shared by every codec detector.
//!
//! Two things live here, both codec-independent because both come from the
//! quantizer model rather than from any particular filterbank:
//!
//! * the per-subband decision thresholds `τ(s)` (the paper's §1.3), and
//! * the measurement they are compared against — scale the coefficients,
//!   round them, and see how far they moved (§1.2).
//!
//! # The idea in one paragraph
//!
//! Under the lossless null model, the rounding error of each coefficient behaves
//! like a uniform random variable on `[-½, ½]`: mean `1/12`, variance
//! `1/180`. Averaged over a subband of `K` coefficients the mean stays `1/12`
//! and the variance falls to `1/(180K)`, and by the central limit theorem the
//! average is close to Gaussian. On transcoded audio the coefficients are
//! near the encoder's lattice, so the error piles up near zero instead.
//! Tonal and near-silent lossless inputs can violate the uniform-error
//! assumption too; the model is not a provenance guarantee.
//! `τ(s)` is placed in the left tail of the *lossless* distribution, at a
//! point reached with probability `P` under that model — so counting how
//! often the measurement falls below it separates the two cases.
//!
//! # A note on normalisation
//!
//! The paper writes `E(s)` as a **sum** over the subband (eq. 5) with
//! `μ = K/12` and `σ² = K/180`. This module uses the **mean** instead, with
//! `μ = 1/12` and `σ² = 1/(180K)`. The two are equivalent — working through
//! eq. (8) under the mean convention yields a threshold exactly `K` times
//! smaller, matching the `K` times smaller statistic — and the mean is used
//! here because it keeps every subband's numbers in the same range regardless
//! of width, which makes them comparable when reading a debug dump.

/// Target probability `P` in eq. (8): how often a *genuine lossless* subband
/// is allowed to fall below its own threshold by chance.
///
/// 0.01, from the paper's §1.3 ("Here, we set P = 0.01"). This is not the
/// false-positive rate of the detector: a single subband dipping below τ
/// proves nothing, and the verdict comes from the rate across many subbands,
/// frames and scalefactors compared against a separate significance
/// threshold.
pub const TARGET_PROBABILITY: f64 = 0.01;

/// Per-subband decision thresholds `τ(s)`, one per entry of `widths`.
///
/// Implements eq. (8) under this module's mean normalisation:
///
/// ```text
/// τ(s) = μ − √2·σ·erfinv( erf( μ / (√2·σ) ) − 2P )
/// with  μ = 1/12,  σ = √(1 / (180·K(s)))
/// ```
///
/// which is the value τ solving `prob{E(s) < τ} = P` for eq. (7)'s
/// distribution. Computed once per file and cached by the caller: it depends
/// only on the subband widths, which come from the standard's tables.
///
/// A zero-width subband would divide by zero, so it gets a threshold of `0` —
/// nothing can fall below it, which is the right answer for a band containing
/// no coefficients.
pub fn subband_thresholds(widths: &[usize], target_probability: f64) -> Vec<f64> {
    const MU: f64 = 1.0 / 12.0;
    widths
        .iter()
        .map(|&k| {
            if k == 0 {
                return 0.0;
            }
            let sigma = (1.0 / (180.0 * k as f64)).sqrt();
            let scale = std::f64::consts::SQRT_2 * sigma;
            MU - scale * erfinv(erf(MU / scale) - 2.0 * target_probability)
        })
        .collect()
}

/// One subband's coefficients, prepared for repeated scalefactor trials.
///
/// The scaling in eq. (1) is `(|X(k)| · 2^(−φ/4))^(3/4)`, which expands to
/// `|X(k)|^(3/4) · 2^(−3φ/16)`. The expensive part — a fractional power per
/// coefficient — therefore does not depend on the scalefactor at all, and is
/// hoisted out of the scalefactor loop here. With 64 scalefactors tried per
/// subband that is the difference between one `powf` per coefficient and
/// sixty-four, over a sweep that runs the loop hundreds of millions of times.
pub struct PreparedSubband {
    /// `|X(k)|^(3/4)` for each coefficient.
    mag34: Vec<f64>,
    /// The dead-zone bound `φ^dz` of eq. (2), plus the reference's normalization
    /// bias — see [`PreparedSubband::prepare`].
    sf_max: f64,
    peak34: f64,
}

impl PreparedSubband {
    /// Prepare an empty subband; fill it with [`PreparedSubband::prepare`].
    pub fn new() -> Self {
        Self {
            mag34: Vec::new(),
            sf_max: 0.0,
            peak34: 0.0,
        }
    }

    /// Load `coeffs` and precompute what does not vary with the scalefactor.
    ///
    /// The MATLAB reference searches `φ = δ·(φ^dz + bias) − bias`, with
    /// bias 60 for AAC and 80 for MP3. This equals the paper's unbiased
    /// formula applied to coefficients scaled by `2^(bias/4)`. Preserve that
    /// convention; changing the units changes the finite search grid.
    ///
    /// Returns `false` for a subband whose coefficients are all zero. Silence
    /// has no lattice to sit on: `φ^dz` involves `log2(0)`, and every
    /// downstream quantity becomes meaningless. (The reference implementation
    /// lets this run and produces `NaN`, which then fails its `< τ` test and
    /// contributes nothing — the same outcome, reached by arithmetic on
    /// non-numbers rather than by saying so.)
    pub fn prepare(&mut self, coeffs: &[f64], bias: f64) -> bool {
        // Written as an explicit loop rather than `fold(0.0, f64::max)`
        // because that would be silently wrong here: `f64::max` *ignores* a
        // NaN operand and returns the other one, so a band holding one NaN
        // from a broken decode would sail through with a plausible peak and
        // poison every coefficient afterwards.
        let mut peak = 0.0f64;
        for c in coeffs {
            if !c.is_finite() {
                self.mag34.clear();
                return false;
            }
            let a = c.abs();
            if a > peak {
                peak = a;
            }
        }
        if peak <= 0.0 {
            self.mag34.clear();
            return false;
        }
        // Eq. (2): φ^dz = 16/3 + 4·log2(max|X(k)|), the scalefactor above
        // which every coefficient in the band rounds to zero.
        self.sf_max = 4.0 * peak.log2() + 16.0 / 3.0 + bias;
        self.peak34 = peak.powf(0.75);

        self.mag34.clear();
        self.mag34.extend(coeffs.iter().map(|c| c.abs().powf(0.75)));
        true
    }

    /// How many of `deltas` produce a mean squared rounding error below `tau`.
    ///
    /// This is the inner count `c` of the paper's §1.4 pseudocode, for one
    /// subband of one frame at one alignment.
    pub fn on_grid_count(&self, deltas: &[f64], bias: f64, tau: f64) -> usize {
        if self.mag34.is_empty() {
            return 0;
        }
        let inv_n = 1.0 / self.mag34.len() as f64;
        deltas
            .iter()
            .filter(|&&delta| {
                // (1): the scalefactor-dependent half of the scaling, now a
                // single multiplier since the power was taken in `prepare`.
                let factor = (-3.0 / 16.0 * (delta * self.sf_max - bias)).exp2();
                // A trial wholly inside the zero quantization cell carries no
                // evidence of a codec. This explicit departure from MATLAB
                // prevents almost-silent Side channels from scoring as a grid.
                // Callers retain these rejected trials in the denominator.
                if !factor.is_finite() || self.peak34 * factor < 0.5 {
                    return false;
                }
                // (4) and (5): round, take the residual, average its square.
                let energy: f64 = self
                    .mag34
                    .iter()
                    .map(|m| {
                        let scaled = m * factor;
                        let e = scaled.round() - scaled;
                        e * e
                    })
                    .sum();
                energy * inv_n < tau
            })
            .count()
    }
}

impl Default for PreparedSubband {
    fn default() -> Self {
        Self::new()
    }
}

/// The scalefactor values to try, `nb` of them spread evenly over
/// `[min, max]`.
///
/// The bounds are the paper's §2 result: `δ ∈ [0.3, 0.7]` covers 90% of the
/// relative scalefactors observed in real AAC bitstreams, tuned on the hardest
/// case in the author's corpus rather than on the average one.
pub fn scalefactor_grid(min: f64, max: f64, nb: usize) -> Vec<f64> {
    if nb == 0 {
        return Vec::new();
    }
    if nb == 1 {
        return vec![min];
    }
    let step = (max - min) / (nb - 1) as f64;
    (0..nb).map(|i| min + step * i as f64).collect()
}

/// The error function.
///
/// Rational approximation with fractional error below 1.2e-7 everywhere
/// (Numerical Recipes §6.2, after W. J. Cody). Far more accuracy than the
/// thresholds need — they are compared against a measured average that is
/// itself noisy — and it avoids a dependency for two functions.
fn erf(x: f64) -> f64 {
    1.0 - erfc(x)
}

/// The complementary error function, `1 − erf(x)`.
///
/// Plain Horner evaluation in `t = 2/(2+|x|)` of the nine-term fit given as
/// `erfcc` in Numerical Recipes §6.2. (An earlier draft of this function
/// mixed these coefficients into the Chebyshev recurrence belonging to a
/// *different* routine on the same page — the two are not interchangeable,
/// and the result was wrong by percent-level amounts. Hence the unit tests
/// checking against published table values rather than against each other.)
fn erfc(x: f64) -> f64 {
    let z = x.abs();
    let t = 2.0 / (2.0 + z);
    let poly = -1.265_512_23
        + t * (1.000_023_68
            + t * (0.374_091_96
                + t * (0.096_784_18
                    + t * (-0.186_288_06
                        + t * (0.278_868_07
                            + t * (-1.135_203_98
                                + t * (1.488_515_87 + t * (-0.822_152_23 + t * 0.170_872_77))))))));
    let ans = t * (-z * z + poly).exp();

    if x >= 0.0 {
        ans
    } else {
        2.0 - ans
    }
}

/// The inverse error function.
///
/// Solved numerically rather than approximated: bisection to bracket, then
/// Newton steps using `erf'(x) = 2/√π · e^(−x²)`. This runs a few dozen times
/// per file — once per subband, at setup — so its cost is irrelevant, and
/// inverting the same `erf` used everywhere else guarantees the two agree
/// with each other rather than merely each being close to the truth.
fn erfinv(y: f64) -> f64 {
    if y <= -1.0 {
        return f64::NEG_INFINITY;
    }
    if y >= 1.0 {
        return f64::INFINITY;
    }
    if y == 0.0 {
        return 0.0;
    }

    // erf saturates past |x| ≈ 6, so this brackets every representable input.
    let (mut lo, mut hi) = (-6.0f64, 6.0f64);
    for _ in 0..64 {
        let mid = 0.5 * (lo + hi);
        if erf(mid) < y {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mut x = 0.5 * (lo + hi);

    // Polish. Newton converges quadratically here; a handful of steps reaches
    // the accuracy floor set by `erf` itself. The derivative of erf is
    // `2/√π · e^(−x²)`, and 2/√π is `FRAC_2_SQRT_PI` in the standard library —
    // no reason to spell the digits out and risk mistyping one.
    for _ in 0..4 {
        let d = std::f64::consts::FRAC_2_SQRT_PI * (-x * x).exp();
        if d.abs() < 1e-300 {
            break;
        }
        x -= (erf(x) - y) / d;
    }
    x
}

#[cfg(test)]
#[path = "../../tests/unit/transcode/criterion.rs"]
mod tests;
