use super::*;

/// A pure sine has a crest factor of exactly sqrt(2) == 3.01 dB; the DR
/// estimate on a steady full-scale sine must land there.
#[test]
fn dr_of_a_pure_sine_is_3db() {
    let mut a = StreamAnalyzer::new(1, 44_100);
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
    let mut a = StreamAnalyzer::new(1, 44_100);
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
    let mut a = StreamAnalyzer::new(1, 44_100);
    for _ in 0..(DYN_BLOCK_FRAMES + 10) {
        a.push_frame(&[0.0], None);
    }
    let summary = a.finish(44_100, None);
    assert!(summary.dr_db.is_none());
}

#[test]
fn float_dr_is_gain_invariant_even_above_full_scale() {
    let selection = crate::AnalysisSelection::from_names(["dynamics"]).expect("known analysis");
    let mut readings = Vec::new();
    for peak in [0.9f32, 1.8] {
        let mut analyzer = StreamAnalyzer::new_selected(1, 48_000, selection);
        // A quarter-rate sine has samples [0, A, 0, -A]. Complete periods
        // give exactly RMS=A/sqrt(2), independently of the implementation.
        for frame in 0..131_072 * 2 {
            let sample = match frame % 4 {
                1 => peak,
                3 => -peak,
                _ => 0.0,
            };
            analyzer.push_frame(&[sample], None);
        }
        let summary = analyzer.finish(48_000, None);
        let dr = summary.dr_db.expect("crest factor");
        let expected = (20.0 * std::f64::consts::SQRT_2.log10()) as f32;
        assert!((dr - expected).abs() < 0.0001, "peak={peak}, DR={dr}");
        // Preserve the existing clipping payload used by authenticity.
        assert_eq!(summary.clipping.peak, peak.min(1.0));
        readings.push(dr);
    }
    assert!((readings[0] - readings[1]).abs() < 0.0001);
}

#[test]
fn dr_uses_the_loudest_twenty_percent_of_multiple_blocks() {
    let selection = crate::AnalysisSelection::from_names(["dynamics"]).expect("known analysis");
    let mut analyzer = StreamAnalyzer::new_selected(1, 44_100, selection);
    // Six specified equal-length blocks: ceil(6*20%)=2. The two greatest
    // powers are 0.8² and 0.6²; their mean is 0.5 and the global peak is 0.8.
    for amplitude in [0.1f32, 0.8, 0.2, 0.4, 0.6, 0.3] {
        for _ in 0..131_072 {
            analyzer.push_frame(&[amplitude], None);
        }
    }
    let dr = analyzer
        .finish(44_100, None)
        .dr_db
        .expect("loud-passage crest factor");
    let expected = (20.0 * (0.8f64 / 0.5f64.sqrt()).log10()) as f32;
    assert!(
        (dr - expected).abs() < 0.0001,
        "DR={dr}, expected={expected}"
    );
}

#[test]
fn stereo_silence_has_no_fake_stereo_finding() {
    let selection = crate::AnalysisSelection::from_names(["stereo"]).expect("known analysis");
    let mut analyzer = StreamAnalyzer::new_selected(2, 44_100, selection);
    for _ in 0..1000 {
        analyzer.push_frame(&[0.0, 0.0], None);
    }
    assert!(!analyzer.finish(44_100, None).fake_stereo);
}

#[test]
fn stereo_clipping_and_final_true_peak_reach_the_analysis_summary() {
    let selection = crate::AnalysisSelection::from_names(["clipping"]).expect("known analysis");
    let mut analyzer = StreamAnalyzer::new_selected(2, 48_000, selection);
    for _ in 0..3 {
        analyzer.push_frame(&[1.0, 0.2], None);
    }
    // Give the final two-sample pulse an independent FIR history.
    for _ in 0..12 {
        analyzer.push_frame(&[0.0, 0.0], None);
    }
    analyzer.push_frame(&[0.0, 0.9], None);
    analyzer.push_frame(&[0.0, 0.9], None);
    let summary = analyzer.finish(48_000, None);
    assert_eq!(summary.clipping.clip_events, 1);
    assert!(summary.clipping.true_peak > 1.09);
}

#[test]
fn loudness_with_md5_constructs_and_runs_only_the_loudness_meter() {
    let selection = crate::AnalysisSelection::from_names(["loudness", "flac-md5"]).expect("names");
    let mut analyzer = StreamAnalyzer::new_selected(2, 44_100, selection);
    assert!(analyzer.loudness.is_some());
    assert!(analyzer.fft.is_none() && analyzer.hann.is_empty() && analyzer.power_acc.is_empty());
    assert!(analyzer.mdct.is_none() && analyzer.mdct_scratch.is_empty());
    assert!(analyzer.true_peak.is_none() && analyzer.bit_depth.is_none());
    assert!(analyzer.discontinuities.is_none() && analyzer.local_phase.is_none());
    assert!(analyzer.high_frequency_stereo.is_none() && analyzer.dc_offset.is_none());
    for i in 0..176_400 {
        let sample = (2.0 * std::f32::consts::PI * 1_000.0 * i as f32 / 44_100.0).sin() * 0.1;
        analyzer.push_frame(&[sample, sample], Some(&[123, 123]));
    }
    assert_eq!(analyzer.window_count, 0);
    assert_eq!(analyzer.mdct_hop, 0);
    assert_eq!(analyzer.dyn_block_frames, 0);
    assert_eq!(analyzer.l_energy, 0.0);
    let summary = analyzer.finish(44_100, Some(16));
    assert!(summary.integrated_lufs.is_some());
    assert!(summary.spectrum_db.is_empty());
    assert!(summary.real_bit_depth.is_none());
    assert_eq!(summary.clipping.peak, 0.0);
}

#[test]
fn three_measurements_do_not_construct_other_meters() {
    let selection = crate::AnalysisSelection::from_names(["bit-depth", "phase", "fingerprints"])
        .expect("names");
    let analyzer = StreamAnalyzer::new_selected(2, 44_100, selection);
    assert!(analyzer.bit_depth.is_some() && analyzer.local_phase.is_some());
    assert!(analyzer.loudness.is_none() && analyzer.true_peak.is_none());
    assert!(analyzer.fft.is_none() && analyzer.mdct.is_none());
    assert!(analyzer.high_frequency_stereo.is_none() && analyzer.discontinuities.is_none());
}
