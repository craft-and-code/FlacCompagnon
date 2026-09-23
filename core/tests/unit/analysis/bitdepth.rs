use super::*;

// The underscore in each literal below sits at the padding boundary, not
// every four digits: `0x1234_00` reads as "the 16-bit sample 0x1234,
// shifted into a 24-bit field, low byte zero", which is the whole point of
// this test. Clippy's suggested regrouping (`0x0012_3400`) is uniform but
// says nothing, so the lint is turned off here rather than obeyed.
#[allow(clippy::unusual_byte_groupings)]
#[test]
fn sixteen_bit_padded_to_24_is_flagged() {
    // 16-bit samples left-shifted into a 24-bit field => low 8 bits zero.
    let mut mask = 0u32;
    for v in [0x1234_00u32, 0x5600_00, 0x00AB_00, 0x7F00_00] {
        mask |= v;
    }
    assert_eq!(effective_bits(mask, 24), 16);
    assert!(is_fake_hires(24, mask));
}

#[test]
fn genuine_24bit_is_not_flagged() {
    let mut mask = 0u32;
    for v in [0x123456u32, 0x000001, 0xABCDEF, 0x000003] {
        mask |= v;
    }
    assert!(effective_bits(mask, 24) > 16);
    assert!(!is_fake_hires(24, mask));
}

#[test]
fn sign_extended_negatives_do_not_inflate() {
    // -256 (0xFFFFFF00) has 8 trailing zero bits; within a 24-bit width the
    // effective depth is 16, not 32.
    let mask = (-256i32) as u32;
    assert_eq!(effective_bits(mask, 24), 16);
}

#[test]
fn silence_reports_one_bit() {
    assert_eq!(effective_bits(0, 24), 1);
}

fn measure(
    bits: u32,
    frames: impl IntoIterator<Item = Vec<i32>>,
    channels: usize,
) -> (u32, BitDepthEvidence) {
    let mut analyzer = BitDepthAnalyzer::new(channels);
    for frame in frames {
        analyzer.push_frame(Some(&frame));
    }
    analyzer
        .finish(Some(bits))
        .expect("complete integer fixture")
}

fn source(n: i32) -> i32 {
    (n * 73 % 60_001) - 30_000
}

#[test]
fn exact_padding_keeps_arbitrary_widths_and_the_silence_convention() {
    for declared in [8, 16, 24, 32] {
        for original in 1..=declared {
            let sample = -1i32 << (declared - original);
            let (bits, e) = measure(declared, [vec![sample, 0]], 2);
            assert_eq!(bits, original);
            assert_eq!(e.method, BitDepthMethod::Stored);
        }
        assert_eq!(measure(declared, [vec![0, 0]], 2).0, 1);
    }
}

#[test]
fn low_level_residual_does_not_hide_a_sustained_lower_depth_grid() {
    // Construct an integer source independently, then add a bounded residue
    // after widening. This includes signed samples on both sides of zero.
    for (original, declared) in [(8, 16), (12, 24), (16, 24), (18, 24), (20, 32), (24, 32)] {
        let shift = declared - original;
        let frames = (0..16_384).map(|n| {
            let range = (1i32 << (original - 1)).min(30_000);
            let s = (n * 73 % (2 * range - 1)) - range;
            let residual = n % 5 - 2;
            vec![(s << shift) + residual]
        });
        let (bits, evidence) = measure(declared, frames, 1);
        assert_eq!(bits, original, "{original} -> {declared}");
        assert_eq!(evidence.stored_bits, declared);
        assert_eq!(evidence.method, BitDepthMethod::NarrowGrid);
    }
}

#[test]
fn a_single_outlier_after_a_long_padded_intro_vetoes_the_grid() {
    let frames =
        (0..20_000).map(|n| vec![(source(n) << 8) + if n == 19_999 { 127 } else { n % 21 - 10 }]);
    let (bits, e) = measure(24, frames, 1);
    assert_eq!((bits, e.method), (24, BitDepthMethod::Stored));
}

#[test]
fn every_channel_must_support_the_lower_depth() {
    for reverse in [false, true] {
        let frames = (0..16_384).map(|n| {
            let mut f = vec![(source(n) << 8) + n % 21 - 10, source(n) * 253 + 1];
            if reverse {
                f.reverse();
            }
            f
        });
        assert_eq!(measure(24, frames, 2).0, 24);
    }
    let frames = (0..16_384).map(|n| vec![0, (source(n) << 8) + n % 21 - 10]);
    assert_eq!(measure(24, frames, 2).0, 16);
}

#[test]
fn different_channel_grids_use_the_highest_required_depth() {
    let frames = (0..16_384).map(|n| vec![(source(n) << 8) + n % 21 - 10, source(n) << 4]);
    let (bits, evidence) = measure(24, frames, 2);
    assert_eq!(bits, 20);
    assert_eq!(evidence.method, BitDepthMethod::NarrowGrid);
}

#[test]
fn anti_phase_channels_cannot_cancel_the_bit_depth_evidence() {
    let frames = (0..16_384).map(|n| {
        let s = (source(n) << 8) + n % 21 - 10;
        vec![s, -s]
    });
    assert_eq!(measure(24, frames, 2).0, 16);
}

#[test]
fn silent_noisy_dc_and_few_level_signals_do_not_establish_a_grid() {
    for kind in 0..4 {
        let frames = (0..16_384).map(|n| {
            let coarse = match kind {
                0 => 0,
                1 => 10_000,
                2 => {
                    if n % 2 == 0 {
                        20_000
                    } else {
                        -20_000
                    }
                }
                _ => {
                    if n % 4096 == 0 {
                        source(n)
                    } else {
                        0
                    }
                }
            };
            vec![(coarse << 8) + n % 21 - 10]
        });
        assert_eq!(measure(24, frames, 1).0, 24, "kind {kind}");
    }
}

#[test]
fn too_little_varying_audio_cannot_be_an_estimated_grid() {
    let frames = (0..20_000).map(|n| {
        vec![if n < 8192 {
            (source(n) << 8) + n % 21 - 10
        } else {
            0
        }]
    });
    assert_eq!(measure(24, frames, 1).0, 24);
}

#[test]
fn native_integer_noise_and_low_level_tones_are_not_lower_depth() {
    for amplitude in [10, 1_000, 30_000, 8_000_000] {
        let mut rng = 0xa17b_924du32;
        let noise = (0..16_384).map(|_| {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            vec![(rng % (2 * amplitude + 1)) as i32 - amplitude as i32]
        });
        assert_eq!(measure(24, noise, 1).0, 24);
        let tone = (0..16_384)
            .map(|n| vec![(amplitude as f64 * (n as f64 * 0.14247586).sin()).round() as i32]);
        assert_eq!(measure(24, tone, 1).0, 24);
    }
}

#[test]
fn empty_float_or_partly_missing_integer_frames_do_not_invent_a_depth() {
    let mut a = BitDepthAnalyzer::new(2);
    assert!(a.finish(Some(24)).is_none());
    a.push_frame(Some(&[256, -256]));
    a.push_frame(None);
    assert!(a.finish(Some(24)).is_none());
    let mut a = BitDepthAnalyzer::new(2);
    a.push_frame(Some(&[256]));
    assert!(a.finish(Some(24)).is_none());
    assert!(a.finish(Some(0)).is_none());
    assert!(a.finish(Some(33)).is_none());
}
