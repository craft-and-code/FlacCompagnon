//! Thin Tauri event/command adapter over aede-audio's minimal local player.
//! The dedicated control thread retains stream ownership across WebView reloads.
//! No normalization, EQ, crossfade, bit-perfect negotiation or account state.
use aede_audio::output::{Gain, MinimalPlayer};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
enum Cmd {
    Play(PathBuf, u64, mpsc::Sender<Result<(), String>>),
    Stop(u64),
    Pause(u64, mpsc::Sender<Result<(), String>>),
    Resume(u64, mpsc::Sender<Result<(), String>>),
    Seek(u64, f64, mpsc::Sender<Result<(), String>>),
}
static SENDER: OnceLock<mpsc::Sender<Cmd>> = OnceLock::new();
static REQUEST_SEQ: PlaybackIntent = PlaybackIntent(AtomicU64::new(0));
static GAIN: OnceLock<Arc<Gain>> = OnceLock::new();
fn gain() -> Arc<Gain> {
    GAIN.get_or_init(|| Arc::new(Gain::default())).clone()
}
struct PlaybackIntent(AtomicU64);
impl PlaybackIntent {
    fn next(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst).wrapping_add(1)
    }
    fn is_current(&self, id: u64) -> bool {
        self.0.load(Ordering::SeqCst) == id
    }
    fn admit(&self, id: u64) -> Result<(), String> {
        if self.is_current(id) {
            Ok(())
        } else {
            Err("Playback request was superseded.".into())
        }
    }
}
/// A queued start result, waited off the Tauri async runtime.
pub struct PlayRequest {
    request_id: u64,
    reply: mpsc::Receiver<Result<(), String>>,
}
impl PlayRequest {
    /// Return the accepted event identity or an explicit start/supersession error.
    pub fn wait(self) -> Result<u64, String> {
        self.reply
            .recv()
            .map_err(|_| "Playback engine did not respond.".to_string())??;
        Ok(self.request_id)
    }
}
/// Natural completion for an exact playback request.
#[derive(Clone, Serialize)]
pub struct Finished {
    pub request_id: u64,
}
/// Rough post-volume RMS level for the current request.
#[derive(Clone, Serialize)]
pub struct Level {
    pub request_id: u64,
    pub level: f32,
}
/// Actual device-consumed source position, including a requested seek prefix.
#[derive(Clone, Serialize)]
pub struct Position {
    pub request_id: u64,
    pub position_secs: f64,
}
/// Decode/device failure is distinct from EOF and must not auto-advance the queue.
#[derive(Clone, Serialize)]
pub struct Failure {
    pub request_id: u64,
    pub message: String,
}
const EMIT_INTERVAL: Duration = Duration::from_millis(60);
/// Initialize the one stream-owning control thread. Does not start playback.
pub fn init(app: AppHandle) {
    let (tx, rx) = mpsc::channel();
    if SENDER.set(tx).is_err() {
        return;
    }
    thread::spawn(move || audio_thread(rx, app));
}
fn clear_current<P>(current: &mut Option<(P, u64)>, intent: &PlaybackIntent, stop: u64) {
    // A Stop sender can be preempted while a newer Play has already been queued.
    if intent.is_current(stop) {
        *current = None;
    }
}
fn control_current<P>(
    current: &mut Option<(P, u64)>,
    intent: &PlaybackIntent,
    owner: u64,
    work: impl FnOnce(&mut P) -> Result<(), String>,
) -> Result<(), String> {
    if !intent.is_current(owner) {
        return Err("Playback request was superseded.".into());
    }
    match current.as_mut() {
        Some((player, id)) if *id == owner => work(player),
        Some(_) => Err("Playback request was superseded.".into()),
        None => Err("Nothing is playing.".into()),
    }
}
fn audio_thread(rx: mpsc::Receiver<Cmd>, app: AppHandle) {
    let mut current: Option<(MinimalPlayer, u64)> = None;
    let mut last_emit = Instant::now();
    loop {
        match rx.recv_timeout(EMIT_INTERVAL) {
            Ok(Cmd::Stop(stop)) => {
                clear_current(&mut current, &REQUEST_SEQ, stop);
            }
            Ok(Cmd::Pause(owner, reply)) => {
                let _ = reply.send(control_current(&mut current, &REQUEST_SEQ, owner, |p| {
                    p.pause();
                    Ok(())
                }));
            }
            Ok(Cmd::Resume(owner, reply)) => {
                let _ = reply.send(control_current(&mut current, &REQUEST_SEQ, owner, |p| {
                    p.resume();
                    Ok(())
                }));
            }
            Ok(Cmd::Seek(owner, seconds, reply)) => {
                let _ = reply.send(control_current(&mut current, &REQUEST_SEQ, owner, |p| {
                    p.seek_admitted(seconds, || REQUEST_SEQ.admit(owner))
                }));
            }
            Ok(Cmd::Play(path, id, reply)) => {
                if !REQUEST_SEQ.is_current(id) {
                    let _ = reply.send(Err("Playback request was superseded.".into()));
                    continue;
                }
                current = None;
                let result = MinimalPlayer::open_admitted(&path, gain(), || REQUEST_SEQ.admit(id))
                    .and_then(|player| {
                        if !REQUEST_SEQ.is_current(id) {
                            return Err("Playback request was superseded.".into());
                        }
                        player.resume();
                        current = Some((player, id));
                        Ok(())
                    });
                let _ = reply.send(result);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if last_emit.elapsed() < EMIT_INTERVAL {
            continue;
        }
        last_emit = Instant::now();
        if let Some((player, id)) = &current {
            if !REQUEST_SEQ.is_current(*id) {
                current = None;
                continue;
            }
            let snapshot = player.snapshot();
            let _ = app.emit(
                "playback://level",
                Level {
                    request_id: *id,
                    level: snapshot.level,
                },
            );
            let _ = app.emit(
                "playback://position",
                Position {
                    request_id: *id,
                    position_secs: snapshot.position_secs,
                },
            );
            if let Some(message) = snapshot.error {
                let _ = app.emit(
                    "playback://error",
                    Failure {
                        request_id: *id,
                        message,
                    },
                );
                current = None;
            } else if snapshot.finished {
                let _ = app.emit("playback://finished", Finished { request_id: *id });
                current = None;
            }
        }
    }
}
/// Queue before scheduling the caller's blocking wait, preserving Stop/Play order.
pub fn request_play(path: PathBuf) -> Result<PlayRequest, String> {
    let sender = SENDER.get().ok_or("Playback engine not started.")?;
    let request_id = REQUEST_SEQ.next();
    let (tx, reply) = mpsc::channel();
    sender
        .send(Cmd::Play(path, request_id, tx))
        .map_err(|_| "Playback engine is not running.".to_string())?;
    Ok(PlayRequest { request_id, reply })
}
/// Stop retires pending starts immediately as well as stopping the active stream.
pub fn stop() -> Result<(), String> {
    let sender = SENDER.get().ok_or("Playback engine not started.")?;
    let stop = REQUEST_SEQ.next();
    sender
        .send(Cmd::Stop(stop))
        .map_err(|_| "Playback engine is not running.".into())
}
/// A control queued on the calling thread; only waiting may run in a blocking task.
pub struct ControlRequest {
    reply: mpsc::Receiver<Result<(), String>>,
}
impl ControlRequest {
    /// Await the exact queued control, retaining its captured playback identity.
    pub fn wait(self) -> Result<(), String> {
        self.reply
            .recv()
            .map_err(|_| "Playback engine did not respond.".to_string())?
    }
}
fn enqueue_control(
    sender: &mpsc::Sender<Cmd>,
    intent: &PlaybackIntent,
    owner: u64,
    command: impl FnOnce(mpsc::Sender<Result<(), String>>) -> Cmd,
) -> Result<ControlRequest, String> {
    if !intent.is_current(owner) {
        return Err("Playback request was superseded.".into());
    }
    let (tx, reply) = mpsc::channel();
    sender
        .send(command(tx))
        .map_err(|_| "Playback engine is not running.".to_string())?;
    Ok(ControlRequest { reply })
}
fn request_control(
    owner: u64,
    command: impl FnOnce(mpsc::Sender<Result<(), String>>) -> Cmd,
) -> Result<ControlRequest, String> {
    let sender = SENDER.get().ok_or("Playback engine not started.")?;
    enqueue_control(sender, &REQUEST_SEQ, owner, command)
}
/// Queue Pause for the exact request shown by the caller, before scheduling its wait.
pub fn request_pause(owner: u64) -> Result<ControlRequest, String> {
    request_control(owner, |tx| Cmd::Pause(owner, tx))
}
/// Queue Resume for the exact request shown by the caller.
pub fn request_resume(owner: u64) -> Result<ControlRequest, String> {
    request_control(owner, |tx| Cmd::Resume(owner, tx))
}
/// Queue a seek for the captured track; never reinterpret it as a successor's position.
pub fn request_seek(owner: u64, seconds: f64) -> Result<ControlRequest, String> {
    aede_audio::output::validate_position(seconds)?;
    request_control(owner, |tx| Cmd::Seek(owner, seconds, tx))
}
/// Select preview volume without modifying analysis samples or results.
pub fn set_volume(volume: f32) {
    gain().set_volume(volume);
}
/// Mute independently of the retained preview volume.
pub fn set_muted(muted: bool) {
    gain().set_muted(muted);
}
#[cfg(test)]
#[path = "../tests/unit/playback.rs"]
mod tests;
