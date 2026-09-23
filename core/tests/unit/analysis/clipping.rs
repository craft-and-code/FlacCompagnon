use super::*;

#[test]
fn detects_a_clip_run() {
    let mut s = ClipState::new(0.9997);
    for _ in 0..5 {
        s.push(1.0);
    }
    let info = s.finish(1, 5);
    assert!(info.clipped);
    assert_eq!(info.clip_events, 1);
    assert_eq!(info.clipped_samples, 5);
}

#[test]
fn short_touches_are_not_events() {
    let mut s = ClipState::new(0.9997);
    s.push(1.0);
    s.push(0.2);
    s.push(1.0);
    let info = s.finish(1, 3);
    assert!(!info.clipped);
    assert_eq!(info.clip_events, 0);
}
