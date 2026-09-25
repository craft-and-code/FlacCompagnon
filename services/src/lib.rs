//! Editing and export services used by the FlacCompagnon desktop app.
//!
//! Analysis stays in `flaccompagnon-core`; this crate owns operations that
//! write tags, audio, playlists, or move-related metadata.

#![warn(missing_docs)]

pub mod convert;
pub mod playlist;
pub mod relocate;
pub mod tags;

/// Shared read-only decoder used by conversion.
pub use flaccompagnon_core::decode;
pub use relocate::{match_moved_files, Relocation};
