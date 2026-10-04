use super::*;

#[test]
fn an_unknown_length_body_cannot_grow_past_the_limit() {
    let mut bytes = Vec::new();
    append_bounded(&mut bytes, b"123", 5).unwrap();
    assert!(append_bounded(&mut bytes, b"456", 5).is_err());
    assert_eq!(bytes, b"123");
}

#[test]
fn a_body_at_the_exact_limit_is_accepted() {
    let mut bytes = Vec::new();
    append_bounded(&mut bytes, b"12345", 5).unwrap();
    append_bounded(&mut bytes, b"", 5).unwrap();
    assert_eq!(bytes, b"12345");
}
