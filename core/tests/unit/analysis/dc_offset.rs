use super::*;

#[test]
fn opposite_channel_offsets_do_not_cancel_in_a_downmix() {
    let mut meter = DcOffsetMeter::new(3).unwrap();
    // Alternating AC has an exact zero mean. Binary fractions make the
    // independently added bias exact even after normalization to f32.
    for n in 0..10_000 {
        let ac = if n % 2 == 0 { 0.25 } else { -0.25 };
        meter.push_frame(&[ac + 0.125, ac - 0.125, ac + 0.0625]);
    }
    let result = meter.finish().unwrap();
    assert_eq!(result.channel_means, vec![0.125, -0.125, 0.0625]);
    assert_eq!(result.max_abs, 0.125);
}

#[test]
fn centered_silence_and_empty_input_have_distinct_results() {
    for channels in [1, 2, 32] {
        let mut meter = DcOffsetMeter::new(channels).unwrap();
        meter.push_frame(&vec![0.0; channels]);
        let measured = meter.finish().unwrap();
        assert_eq!(measured.channel_means, vec![0.0; channels]);
        assert_eq!(measured.max_abs, 0.0);
    }
    assert!(DcOffsetMeter::new(1).unwrap().finish().is_none());
    assert!(DcOffsetMeter::new(0).is_none());
    assert!(DcOffsetMeter::new(33).is_none());
    assert!(DcOffsetMeter::new(usize::MAX).is_none());
}

#[test]
fn waveform_asymmetry_alone_is_not_dc_offset() {
    // Unequal positive and negative peaks, but 0.75 - 3*0.25 = 0.
    let mut meter = DcOffsetMeter::new(1).unwrap();
    for sample in [0.75, -0.25, -0.25, -0.25].repeat(1_000) {
        meter.push_frame(&[sample]);
    }
    assert_eq!(meter.finish().unwrap().max_abs, 0.0);
}

#[test]
fn mean_includes_silence_and_partial_periods() {
    let mut meter = DcOffsetMeter::new(1).unwrap();
    for sample in [0.25, 0.25, 0.0, 0.0] {
        meter.push_frame(&[sample]);
    }
    assert_eq!(meter.finish().unwrap().channel_means, vec![0.125]);
    let mut short = DcOffsetMeter::new(1).unwrap();
    short.push_frame(&[-0.5]);
    assert_eq!(short.finish().unwrap().channel_means, vec![-0.5]);
}

#[test]
fn large_samples_do_not_erase_small_bias_or_get_clamped() {
    let mut meter = DcOffsetMeter::new(2).unwrap();
    for x in [f32::MAX, 0.375, -f32::MAX] {
        meter.push_frame(&[x, 2.0]);
    }
    let measured = meter.finish().unwrap();
    assert_eq!(measured.channel_means, vec![0.125, 2.0]);
    assert_eq!(measured.max_abs, 2.0);
}

#[test]
fn malformed_tail_withholds_the_mean_of_the_valid_prefix() {
    for bad in [
        vec![],
        vec![0.0],
        vec![0.0; 3],
        vec![f32::NAN, 0.0],
        vec![0.0, f32::INFINITY],
    ] {
        let mut meter = DcOffsetMeter::new(2).unwrap();
        meter.push_frame(&[0.125, -0.125]);
        meter.push_frame(&bad);
        meter.push_frame(&[0.125, -0.125]);
        assert!(meter.finish().is_none());
    }
}
