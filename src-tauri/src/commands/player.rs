//! Start/stop the preview player.
//!
//! Both hop onto a blocking thread: the engine in [`crate::playback`] talks to
//! its audio thread over a channel and waits for the reply, which must not
//! happen on the async runtime's thread.

use std::path::PathBuf;

use crate::playback;

/// Play `path` (a file already listed in the results table) through the
/// system's default audio output. Stops whatever was playing first.
///
/// Returns a request id the frontend matches against `playback://finished`
/// and `playback://level` events, so a stale notification from a track that
/// was already superseded can be ignored.
#[tauri::command]
pub async fn play_track(path: String) -> Result<u64, String> {
    tauri::async_runtime::spawn_blocking(move || playback::play(PathBuf::from(path)))
        .await
        .map_err(|e| e.to_string())?
}

/// Stop the currently playing track, if any.
#[tauri::command]
pub async fn stop_playback() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(playback::stop)
        .await
        .map_err(|e| e.to_string())?
}
