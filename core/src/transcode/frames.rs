//! Choosing *where* in the file to look, and *which* channel signal to look
//! at.
//!
//! Both decisions are codec-independent, both come from the paper's §1.4, and
//! both exist for the same reason: the detector is expensive, so it is pointed
//! at the parts of the file most likely to answer.
//!
//! # Why the loudest frames
//!
//! §1.3 puts it plainly — "detection can be improved by selecting only
//! high-energy frames". A quiet passage gives the encoder little to quantize:
//! most coefficients land near zero, the lattice is coarse relative to the
//! signal, and the rounding error carries almost no information either way.
//! Loud frames exercise the quantizer across its range, so that is where the
//! grid is visible.

/// Energy of every analysis frame, in dB.
///
/// Frames overlap by half: frame `f` spans `f·frame_len` to
/// `(f+2)·frame_len`, matching the codec's own 50%-overlapped transform. The
/// last two frames' worth of samples are dropped rather than zero-padded —
/// a padded frame's energy is not comparable to a full one's, and it would
/// compete for a place in the top list on false terms.
///
/// Returns an empty vector for a signal shorter than three frames, which is
/// the caller's signal that the file is too short to analyze.
pub fn frame_energies_db(signal: &[f64], frame_len: usize) -> Vec<f64> {
    if frame_len == 0 {
        return Vec::new();
    }
    let count = (signal.len() / frame_len).saturating_sub(2);
    (0..count)
        .map(|f| {
            let start = f * frame_len;
            let energy: f64 = signal
                .get(start..start + 2 * frame_len)
                .map(|w| w.iter().map(|s| s * s).sum())
                .unwrap_or(0.0);
            // dB purely so the values stay comparable when printed; the
            // ordering is the same either way since log is monotonic. Silence
            // gives -inf, which sorts last — correct, it is the least useful
            // frame there is.
            10.0 * energy.log10()
        })
        .collect()
}

/// Indices of the `wanted` highest-energy frames.
///
/// Ties are broken by frame index so the choice is reproducible: a file
/// containing a long stretch of identical frames (digital silence, a held
/// tone) would otherwise give a different answer per run depending on the
/// sort's internal ordering, and a detector whose verdict moves between runs
/// on the same file is worse than a slightly less optimal one.
///
/// Fewer than `wanted` frames are returned when the file is short; the caller
/// decides whether that is still enough to judge.
pub fn top_frames(energies_db: &[f64], wanted: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..energies_db.len()).collect();
    order.sort_by(|&a, &b| {
        energies_db[b]
            .partial_cmp(&energies_db[a])
            // NaN cannot arise from a sum of squares, but `partial_cmp` has
            // to be handled and falling back on the index keeps the order
            // total rather than letting `sort_by` see an inconsistent
            // comparator.
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    order.truncate(wanted);
    order
}

/// Select exactly the calibrated number of frames, with enough trailing
/// samples for the transform at every alignment. `context_frames` is 2 for
/// AAC and 3 for MP3 (which also needs the preceding PQMF granule).
///
/// Returning a shorter selection would silently apply a threshold calibrated
/// for a different number of observations. Select once on M in stereo and
/// pass the same indices to every channel sweep, as in the MATLAB reference.
pub fn select_frames(
    signal: &[f64],
    frame_len: usize,
    context_frames: usize,
    wanted: usize,
) -> Option<Vec<usize>> {
    if frame_len == 0 || wanted == 0 {
        return None;
    }
    let mut energies = frame_energies_db(signal, frame_len);
    energies.truncate((signal.len() / frame_len).saturating_sub(context_frames));
    if energies.len() < wanted {
        return None;
    }
    Some(top_frames(&energies, wanted))
}

/// Mean energy, in dB, over the given frames.
///
/// The mean of the decibel values rather than the decibels of the mean
/// energy: those differ, and this one is what the reference uses to decide
/// which of two channels to test. Returns `-inf` for an empty selection, so
/// a channel with nothing to measure always loses the comparison.
pub fn mean_energy_db(energies_db: &[f64], frames: &[usize]) -> f64 {
    if frames.is_empty() {
        return f64::NEG_INFINITY;
    }
    let sum: f64 = frames.iter().filter_map(|&f| energies_db.get(f)).sum();
    sum / frames.len() as f64
}

/// Mid and side signals, `(l + r)·k` and `(l − r)·k`.
///
/// `k` is 1 for AAC and ½ for MP3 — the reference implementations use different matrices, and it is not a detail that can be normalised away here: the
/// scale feeds straight into `φ^dz` (eq. 2), which is a logarithm of the
/// coefficient peak, so a factor of two shifts every scalefactor the search
/// tries by exactly 4 steps.
///
/// Channels of unequal length are truncated to the shorter, rather than
/// refused: a decoder that returns a partial last frame on one channel should
/// not sink the whole analysis.
pub fn mid_side(left: &[f64], right: &[f64], k: f64) -> (Vec<f64>, Vec<f64>) {
    let n = left.len().min(right.len());
    let mut mid = Vec::with_capacity(n);
    let mut side = Vec::with_capacity(n);
    for i in 0..n {
        mid.push((left[i] + right[i]) * k);
        side.push((left[i] - right[i]) * k);
    }
    (mid, side)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_files_cannot_shrink_the_calibrated_population() {
        for (context, wanted) in [(2, 64), (3, 8)] {
            let hop = 16;
            let minimum = (wanted + context) * hop;
            assert!(select_frames(&vec![1.0; minimum - 1], hop, context, wanted).is_none());
            let selected = select_frames(&vec![1.0; minimum], hop, context, wanted).unwrap();
            assert_eq!(selected.len(), wanted);
            for frame in selected {
                // Last alignment plus complete AAC window / MP3 context.
                assert!(frame * hop + (hop - 1) + context * hop <= minimum);
            }
        }
    }

    #[test]
    fn frames_overlap_by_half_and_drop_the_incomplete_tail() {
        // 10 frames' worth of samples gives 8 analysis frames: the last two
        // starts would run past the end of the signal.
        let signal = vec![1.0; 10 * 64];
        assert_eq!(frame_energies_db(&signal, 64).len(), 8);

        // Too short to analyze at all, at every boundary.
        for n in [0usize, 63, 64, 127, 128, 191] {
            assert!(
                frame_energies_db(&vec![1.0; n], 64).len() <= 1,
                "n={n} produced too many frames"
            );
        }
        assert!(
            frame_energies_db(&[1.0; 100], 0).is_empty(),
            "zero-length frame"
        );
    }

    /// The energy of a frame must cover *two* frame lengths — using one would
    /// silently measure the wrong window and pick the wrong frames.
    #[test]
    fn energy_covers_two_frame_lengths() {
        let mut signal = vec![0.0; 4 * 8];
        // Put energy only in the second half of frame 0's span.
        for s in signal.iter_mut().take(16).skip(8) {
            *s = 1.0;
        }
        let db = frame_energies_db(&signal, 8);
        // Frame 0 spans samples 0..16, so it must see those eight ones.
        assert!((db[0] - 10.0 * 8.0f64.log10()).abs() < 1e-12, "{db:?}");
    }

    #[test]
    fn top_frames_picks_the_loudest_and_breaks_ties_by_index() {
        let db = [1.0, 9.0, 5.0, 9.0, 2.0];
        assert_eq!(top_frames(&db, 3), vec![1, 3, 2]);
        // Asking for more than exist yields what exists, in order.
        assert_eq!(top_frames(&db, 99).len(), 5);
        assert_eq!(top_frames(&[], 4), Vec::<usize>::new());
        assert_eq!(top_frames(&db, 0), Vec::<usize>::new());

        // Identical frames must give the same answer every time.
        let flat = [3.0; 6];
        assert_eq!(top_frames(&flat, 3), vec![0, 1, 2]);
        assert_eq!(top_frames(&flat, 3), top_frames(&flat, 3));
    }

    #[test]
    fn mean_energy_ignores_missing_frames_and_empty_selections() {
        let db = [10.0, 20.0, 30.0];
        assert!((mean_energy_db(&db, &[0, 2]) - 20.0).abs() < 1e-12);
        assert_eq!(mean_energy_db(&db, &[]), f64::NEG_INFINITY);
        // An out-of-range index must not panic — it is dropped.
        assert!((mean_energy_db(&db, &[1, 99]) - 10.0).abs() < 1e-12);
    }

    #[test]
    fn mid_side_applies_its_scale_and_tolerates_ragged_channels() {
        let (m, s) = mid_side(&[1.0, 2.0], &[3.0, 4.0], 1.0);
        assert_eq!(m, vec![4.0, 6.0]);
        assert_eq!(s, vec![-2.0, -2.0]);

        let (m_half, s_half) = mid_side(&[1.0, 2.0], &[3.0, 4.0], 0.5);
        assert_eq!(m_half, vec![2.0, 3.0]);
        assert_eq!(s_half, vec![-1.0, -1.0]);

        let (m2, s2) = mid_side(&[1.0, 2.0, 3.0], &[1.0], 1.0);
        assert_eq!(m2.len(), 1);
        assert_eq!(s2.len(), 1);
        let (m3, s3) = mid_side(&[], &[1.0], 1.0);
        assert!(m3.is_empty() && s3.is_empty());
    }
}
