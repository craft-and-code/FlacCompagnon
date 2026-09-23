use super::*;

#[test]
fn tone_energy_concentrates_near_its_bin() {
    // A mid-band sinusoid at the centre frequency of bin k0 should place the
    // largest MDCT coefficient at (or adjacent to) k0.
    let n = 256;
    let mdct = Mdct::new(n);
    let len = 2 * n;
    let k0 = 60usize;
    // MDCT bin k corresponds to normalized frequency (k + 0.5) / (2N).
    let f = (k0 as f32 + 0.5) / (len as f32);
    let frame: Vec<f32> = (0..len)
        .map(|t| (2.0 * std::f32::consts::PI * f * t as f32).cos())
        .collect();
    let mut out = vec![0.0f32; n];
    mdct.forward(&frame, &mut out);

    let mut argmax = 0usize;
    let mut max = 0.0f32;
    for (k, &v) in out.iter().enumerate() {
        if v.abs() > max {
            max = v.abs();
            argmax = k;
        }
    }
    assert!(
        (argmax as isize - k0 as isize).abs() <= 2,
        "argmax {argmax} not near {k0}"
    );
}

#[test]
fn silence_transforms_to_zero() {
    let n = 128;
    let mdct = Mdct::new(n);
    let frame = vec![0.0f32; 2 * n];
    let mut out = vec![9.0f32; n];
    mdct.forward(&frame, &mut out);
    assert!(out.iter().all(|&v| v.abs() < 1e-6));
}
