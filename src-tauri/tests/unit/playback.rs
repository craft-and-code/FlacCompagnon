use super::*;

#[test]
fn queued_play_cannot_become_current_after_a_later_stop() {
    let intent = PlaybackIntent(AtomicU64::new(0));
    let old_play = intent.next();
    let stop = intent.next();
    assert!(!intent.is_current(old_play));
    assert!(intent.is_current(stop));
    let newer_play = intent.next();
    assert!(!intent.is_current(old_play));
    assert!(intent.is_current(newer_play));
}

#[test]
fn an_extreme_seek_returns_silence_without_overflowing() {
    let source = SampleSource::Static(Arc::new(vec![0.25, -0.25]));
    let mut output = [1.0; 8];
    assert_eq!(source.read(usize::MAX, &mut output), (0, true));
    assert_eq!(output, [0.0; 8]);
}

#[test]
fn an_extreme_seek_in_a_stream_waits_for_decode_without_panicking() {
    let source = SampleSource::Streaming(Arc::new(StreamBuffer {
        samples: Mutex::new(vec![0.25, -0.25]),
        done: AtomicBool::new(false),
        cancel: Arc::new(AtomicBool::new(false)),
    }));
    let mut output = [1.0; 8];
    assert_eq!(source.read(usize::MAX, &mut output), (0, false));
    assert_eq!(output, [0.0; 8]);
}
