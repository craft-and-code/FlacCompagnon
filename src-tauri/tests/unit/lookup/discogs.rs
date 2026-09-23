use super::*;

#[test]
fn accepts_a_plain_numeric_release_id() {
    assert!(is_valid_release_id("249504"));
    assert!(is_valid_release_id("1"));
}

/// The id is interpolated into a URL path, so anything that could steer
/// the request elsewhere has to be refused before it reaches the network.
#[test]
fn rejects_ids_that_could_steer_the_request_elsewhere() {
    assert!(!is_valid_release_id(""));
    assert!(!is_valid_release_id("../../etc/passwd"));
    assert!(!is_valid_release_id("249504/../x"));
    assert!(!is_valid_release_id("249504?token=x"));
    assert!(!is_valid_release_id("249504#f"));
    assert!(!is_valid_release_id("249 504"));
    assert!(!is_valid_release_id("-1"));
}
