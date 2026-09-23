use super::*;

#[test]
fn only_the_three_tabulated_rates_are_supported() {
    assert_eq!(supported_rate_khz(44_100), Some(44));
    assert_eq!(supported_rate_khz(48_000), Some(48));
    assert_eq!(supported_rate_khz(32_000), Some(32));
    // Hi-res: a band limit here is upsampling, not a lossy source.
    assert_eq!(supported_rate_khz(96_000), None);
    assert_eq!(supported_rate_khz(88_200), None);
    // Odd rates from games or old hardware have no tabulated bands.
    assert_eq!(supported_rate_khz(22_050), None);
    assert_eq!(supported_rate_khz(0), None);
}
