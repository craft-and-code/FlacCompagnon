//! Start/stop/pause/seek the preview player, and its volume.
//!
//! Play and stop are queued before scheduling a blocking worker so a delayed
//! play cannot arrive after a newer stop. Waiting for playback, pause, resume,
//! or seek happens off the async runtime. Volume and mute are a pair of plain
//! atomics the output callback reads directly (see `playback::set_volume`'s
//! doc comment) — nothing to wait on, so those two stay synchronous.

use std::path::PathBuf;

use crate::playback;

/// Play `path` (a file already listed in the results table) through the
/// system's default audio output. Stops whatever was playing first.
///
/// Returns a request id the frontend matches against `playback://finished`,
/// `playback://level` and `playback://position` events, so a stale
/// notification from a track that was already superseded can be ignored.
#[tauri::command]
pub async fn play_track(path: String) -> Result<u64, String> {
    let request = playback::request_play(PathBuf::from(path))?;
    tauri::async_runtime::spawn_blocking(move || request.wait())
        .await
        .map_err(|e| e.to_string())?
}

/// Stop the currently playing track, if any.
#[tauri::command]
pub fn stop_playback() -> Result<(), String> {
    playback::stop()
}

/// Pause the currently playing track in place — the footer's play/pause
/// button, as opposed to a row's play button (which stops outright, see
/// `stop_playback`).
#[tauri::command]
pub async fn pause_playback() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(playback::pause)
        .await
        .map_err(|e| e.to_string())?
}

/// Resume a track paused with [`pause_playback`].
#[tauri::command]
pub async fn resume_playback() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(playback::resume)
        .await
        .map_err(|e| e.to_string())?
}

/// Move the currently playing track's playhead, in seconds from its start —
/// the footer's seek bar, dragged or clicked.
#[tauri::command]
pub async fn seek_playback(seconds: f64) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || playback::seek(seconds))
        .await
        .map_err(|e| e.to_string())?
}

/// Set the playback volume (0.0–1.0, clamped). Synchronous: this is a plain
/// atomic store, never blocks, and cannot fail.
#[tauri::command]
pub fn set_volume(volume: f32) {
    playback::set_volume(volume);
}

/// Mute or unmute without discarding the volume level underneath — see
/// `playback::set_muted`'s doc comment.
#[tauri::command]
pub fn set_muted(muted: bool) {
    playback::set_muted(muted);
}
