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
fn an_extreme_seek_returns_an_error_without_overflowing() {
    assert!(aede_audio::output::validate_position(f64::MAX).is_err());
}
#[test]
fn a_non_finite_seek_is_rejected_before_a_decoder_or_device_opens() {
    for seconds in [f64::NAN, f64::INFINITY, -1.] {
        assert!(aede_audio::output::validate_position(seconds).is_err());
    }
}

#[test]
fn a_delayed_stop_does_not_destroy_a_newer_play_request() {
    let intent = PlaybackIntent(AtomicU64::new(0));
    let _first = intent.next();
    // Stop obtains its generation, but its sender is preempted before enqueueing.
    let delayed_stop = intent.next();
    let next = intent.next();
    let mut current = Some(("successor stream", next));
    clear_current(&mut current, &intent, delayed_stop);
    assert_eq!(current, Some(("successor stream", next)));
    let current_stop = intent.next();
    clear_current(&mut current, &intent, current_stop);
    assert!(current.is_none());
}
#[test]
fn delayed_pause_resume_and_seek_do_not_change_the_successor_stream() {
    for action in ["pause", "resume", "seek"] {
        let intent = PlaybackIntent(AtomicU64::new(0));
        let owner = intent.next();
        let next = intent.next();
        let mut current = Some((Vec::new(), next));
        let result = control_current(&mut current, &intent, owner, |player| {
            player.push(action);
            Ok(())
        });
        assert!(result.is_err(), "obsolete {action} was accepted");
        assert!(
            current.as_ref().unwrap().0.is_empty(),
            "obsolete {action} changed successor"
        );
        control_current(&mut current, &intent, next, |player| {
            player.push(action);
            Ok(())
        })
        .unwrap();
        assert_eq!(current.unwrap().0, vec![action]);
    }
}

#[test]
fn queued_controls_keep_their_owner_before_the_blocking_wait_is_scheduled() {
    let intent = PlaybackIntent(AtomicU64::new(0));
    let (sender, commands) = mpsc::channel();
    let owner = intent.next();
    let request = enqueue_control(&sender, &intent, owner, |reply| {
        Cmd::Seek(owner, 12., reply)
    })
    .unwrap();
    // A newer Play is accepted before a blocking waiter or the control thread runs.
    let next = intent.next();
    let mut current = Some((0., next));
    let Cmd::Seek(captured, seconds, reply) = commands.recv().unwrap() else {
        panic!("wrong queued control")
    };
    assert_eq!(captured, owner);
    reply
        .send(control_current(
            &mut current,
            &intent,
            captured,
            |position| {
                *position = seconds;
                Ok(())
            },
        ))
        .unwrap();
    assert!(request.wait().is_err());
    assert_eq!(current, Some((0., next)));
    assert!(enqueue_control(&sender, &intent, owner, |reply| Cmd::Pause(owner, reply)).is_err());
    assert!(commands.try_recv().is_err());
}

#[test]
fn native_owner_replacement_after_a_real_header_stops_before_device_open() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("owner-retirement.wav");
    // One complete stereo PCM16 frame; no encoder dependency in the native adapter suite.
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(40_u32.to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(2_u16.to_le_bytes());
    bytes.extend(48_000_u32.to_le_bytes());
    bytes.extend(192_000_u32.to_le_bytes());
    bytes.extend(4_u16.to_le_bytes());
    bytes.extend(16_u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(4_u32.to_le_bytes());
    bytes.extend(8192_i16.to_le_bytes());
    bytes.extend((-8192_i16).to_le_bytes());
    std::fs::write(&path, bytes).unwrap();
    let intent = PlaybackIntent(AtomicU64::new(0));
    let owner = intent.next();
    let checks = std::cell::Cell::new(0);
    let result = MinimalPlayer::open_admitted(&path, Arc::new(Gain::default()), || {
        checks.set(checks.get() + 1);
        if checks.get() == 2 {
            intent.next();
        }
        intent.admit(owner)
    });
    assert_eq!(
        result.err().expect("retired native preparation"),
        "Playback request was superseded."
    );
    assert_eq!(checks.get(), 2);
}
