//! Every Tauri command the frontend can invoke, one file per domain.
//!
//! Commands are deliberately thin (see AGENTS.md): validate the arguments,
//! call a domain service, map errors to a `String` the frontend can show.
//! Analysis belongs in `core`, editing in `services`, and app-specific
//! integrations in separate backend modules, all testable without a GUI.
//!
//! * [`analysis`] — run the detector over a set of targets.
//! * [`batch`] — the cancellation flag and progress plumbing those long jobs
//!   share, plus target expansion.
//! * [`convert`] — re-encode the conversion panel's imported tracks to
//!   another format, optionally copying everything else in their folders
//!   alongside. The other command here (besides [`tags`]) that writes files.
//! * [`spectrograms`] — render a PNG per track through ffmpeg.
//! * [`report`] — write CSV/JSON reports and M3U playlists, and read a report
//!   back in.
//! * [`tags`] — read/write tags and cover art. The only commands here that
//!   modify the user's audio files, and only on an explicit Save.
//! * [`files`] — reveal a file or open a folder in the OS file browser.
//! * [`rename`] — rename a file on disk, keeping its extension. The only
//!   command that changes where a file lives rather than what's in it.
//! * [`lookup`] — thin wrappers over the online providers in [`crate::lookup`].
//! * [`credentials`] — status, migration and explicit changes in the OS vault.
//! * [`player`] — start/stop/pause/seek the preview player, and its volume.

pub mod analysis;
pub mod batch;
pub mod convert;
pub mod credentials;
pub mod files;
pub mod lookup;
pub mod player;
pub mod rename;
pub mod report;
pub mod spectrograms;
pub mod tags;

use std::path::Path;

/// A path's final component, or an empty string for a path that has none.
/// Used in progress events and per-file error messages, where the full path
/// would be unreadable.
pub(crate) fn file_name(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
}
