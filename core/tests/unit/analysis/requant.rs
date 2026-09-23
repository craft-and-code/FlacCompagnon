use super::*;

#[test]
fn bessel_matches_reference() {
    // I0(0)=1; I0(1)≈1.2660658; I0(12.566)≈ large — check monotonicity too.
    assert!((bessel_i0(0.0) - 1.0).abs() < 1e-15);
    assert!((bessel_i0(1.0) - 1.2660658777520084).abs() < 1e-12);
    assert!(bessel_i0(4.0 * std::f64::consts::PI) > bessel_i0(10.0));
}

#[test]
fn windows_have_expected_shape() {
    let s = sine_window();
    let k = kbd_window();
    assert_eq!(s.len(), L);
    assert_eq!(k.len(), L);
    // Princen–Bradley: w[n]^2 + w[n+N]^2 == 1 for both AAC windows.
    for n in 0..N {
        assert!((s[n] * s[n] + s[n + N] * s[n + N] - 1.0).abs() < 1e-9);
        assert!((k[n] * k[n] + k[n + N] * k[n + N] - 1.0).abs() < 1e-9);
    }
    // Same property for the short-block windows.
    let ss = sine_window_of(LSHORT);
    let ks = kbd_window_of(NS);
    assert_eq!(ss.len(), LSHORT);
    assert_eq!(ks.len(), LSHORT);
    for n in 0..NS {
        assert!((ss[n] * ss[n] + ss[n + NS] * ss[n + NS] - 1.0).abs() < 1e-9);
        assert!((ks[n] * ks[n] + ks[n + NS] * ks[n + NS] - 1.0).abs() < 1e-9);
    }
}

/// The short (256-point) MDCT must match the direct definition, exactly as
/// the long one does — same twiddle machinery, different size.
#[test]
fn short_mdct_matches_direct() {
    let mdct = Mdct::short();
    let win = sine_window_of(LSHORT);
    let mut state = 0xC0FFEEu64;
    let frame: Vec<f64> = (0..LSHORT)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((state >> 33) as f64 / (1u64 << 31) as f64) - 1.0
        })
        .collect();
    let mut fast = vec![0.0; NS];
    mdct.forward(&frame, &win, &mut fast);
    let n0 = NS as f64 / 2.0 + 0.5;
    for &k in &[0usize, 1, 17, 64, 127] {
        let mut acc = 0.0;
        for n in 0..LSHORT {
            acc += frame[n]
                * win[n]
                * (std::f64::consts::PI / NS as f64 * (n as f64 + n0) * (k as f64 + 0.5)).cos();
        }
        assert!(
            (acc - fast[k]).abs() < 1e-9 * acc.abs().max(1.0),
            "short bin {k}: direct {acc} vs fft {}",
            fast[k]
        );
    }
}

#[test]
fn fft_mdct_matches_direct() {
    let mdct = Mdct::new();
    let win = kbd_window();
    // Deterministic pseudo-random frame.
    let mut state = 0x12345678u64;
    let frame: Vec<f64> = (0..L)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((state >> 33) as f64 / (1u64 << 31) as f64) - 1.0
        })
        .collect();
    let mut fast = vec![0.0; N];
    mdct.forward(&frame, &win, &mut fast);
    // Direct definition.
    let n0 = N as f64 / 2.0 + 0.5;
    for &k in &[0usize, 1, 17, 500, 1023] {
        let mut acc = 0.0;
        for n in 0..L {
            acc += frame[n]
                * win[n]
                * (std::f64::consts::PI / N as f64 * (n as f64 + n0) * (k as f64 + 0.5)).cos();
        }
        assert!(
            (acc - fast[k]).abs() < 1e-9 * acc.abs().max(1.0),
            "bin {k}: direct {acc} vs fft {}",
            fast[k]
        );
    }
}
