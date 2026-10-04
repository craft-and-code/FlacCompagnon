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

#[test]
fn quiet_right_channel_does_not_interrupt_left_channel_clipping() {
    let mut state = ClipState::new(0.9997);
    for _ in 0..5 {
        state.push_frame(&[1.0, 0.2]);
    }
    let info = state.finish(2, 5);
    assert_eq!(info.clip_events, 1);
    assert_eq!(info.clipped_samples, 5);
}

#[test]
fn adjacent_channels_do_not_create_a_clipping_run() {
    let mut state = ClipState::new(0.9997);
    state.push_frame(&[1.0, 1.0]);
    state.push_frame(&[1.0, 0.2]);
    state.push_frame(&[0.2, 1.0]);
    let info = state.finish(2, 3);
    assert_eq!(info.clip_events, 0);
    assert_eq!(info.clipped_samples, 4);
}

#[test]
fn long_clipping_runs_are_counted_once_per_channel() {
    let mut state = ClipState::new(0.9997);
    for _ in 0..100_000 {
        state.push_frame(&[1.0, -1.0]);
    }
    let info = state.finish(2, 100_000);
    assert_eq!(info.clip_events, 2);
    assert_eq!(info.clipped_samples, 200_000);
}
