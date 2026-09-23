use super::*;

#[test]
fn year_prefix_takes_only_a_four_digit_head() {
    assert_eq!(year_prefix("1997").as_deref(), Some("1997"));
    assert_eq!(year_prefix("1997-06-16").as_deref(), Some("1997"));
    assert_eq!(year_prefix(""), None);
    assert_eq!(year_prefix("199"), None);
    assert_eq!(year_prefix("unknown"), None);
}

/// A provider returning a multi-byte character where a date was expected
/// must not panic on the byte slice — this is the crash `s[..4]` would
/// have caused on, for example, "±1997".
#[test]
fn year_prefix_survives_non_ascii() {
    assert_eq!(year_prefix("±1997"), None);
    assert_eq!(year_prefix("日本語です"), None);
}
