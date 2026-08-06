//! Every Tauri command the frontend can invoke, one file per domain.
//!
//! Commands are deliberately thin (see CLAUDE.md): validate the arguments,
//! call into `flaccompagnon_core`, map the error to a `String` the frontend
//! can show. Anything with real logic belongs in `core`, where it can be
//! tested without a GUI.
//!
//! * [`analysis`] — run the detector over a set of targets.
//! * [`batch`] — the cancellation flag and progress plumbing those long jobs
//!   share, plus target expansion.
//! * [`spectrograms`] — render a PNG per track through ffmpeg.
//! * [`report`] — write CSV/JSON reports and M3U playlists, and read a report
//!   back in.
//! * [`tags`] — read/write tags and cover art. The only commands here that
//!   modify the user's audio files, and only on an explicit Save.
//! * [`files`] — reveal a file or open a folder in the OS file browser.
//! * [`lookup`] — thin wrappers over the online providers in [`crate::lookup`].
//! * [`player`] — start/stop the preview player.

pub mod analysis;
pub mod batch;
pub mod files;
pub mod lookup;
pub mod player;
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
