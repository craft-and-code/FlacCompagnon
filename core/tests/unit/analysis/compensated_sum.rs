use super::*;

#[test]
fn exactly_representable_small_terms_survive_both_cancellation_orders() {
    for values in [
        [2f64.powi(80), 0.375, -2f64.powi(80)],
        [0.375, 2f64.powi(80), -2f64.powi(80)],
        [2f64.powi(80), -2f64.powi(80), 0.375],
    ] {
        let mut sum = CompensatedSum::default();
        for value in values {
            sum.push(value);
        }
        assert_eq!(sum.total(), 0.375);
    }
}

#[test]
fn a_quiet_window_is_not_lost_when_subtracting_large_prefixes() {
    let mut sum = CompensatedSum::default();
    sum.push(2f64.powi(80));
    let before = sum;
    // Exact dyadic ground truth: 240 * (1/8)^2 = 15/4.
    for _ in 0..240 {
        sum.push(0.125 * 0.125);
    }
    assert_eq!(sum.total() - before.total(), 0.0);
    assert_eq!(sum.difference(before), 3.75);
    assert_eq!(before.difference(sum), -3.75);
}

#[test]
fn adding_and_removing_separately_preserves_a_small_rolling_term() {
    let large = f64::from(f32::MAX).powi(2);
    let mut sum = CompensatedSum::default();
    sum.push(large);
    sum.push(0.015625);
    sum.push(-large);
    assert_eq!(sum.total(), 0.015625);
}
