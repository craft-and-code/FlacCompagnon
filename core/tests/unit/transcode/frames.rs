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
