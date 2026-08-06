//! Thin command wrappers over the online providers in [`crate::lookup`].
//!
//! These are the only commands that reach the network, and only ever from an
//! explicit "Search online" click. They hold no logic — the providers do —
//! but they are what makes the surface visible in one place: if a command
//! that talks to the internet is ever added, it belongs here and nowhere else.

use crate::lookup::{LookupCandidate, LookupRelease};

/// Search MusicBrainz for releases matching free-text `query`.
#[tauri::command]
pub async fn lookup_musicbrainz(query: String) -> Result<Vec<LookupCandidate>, String> {
    crate::lookup::musicbrainz_search(query).await
}

/// Full track list (and cover art, if any) for a MusicBrainz release chosen
/// from [`lookup_musicbrainz`]'s results.
#[tauri::command]
pub async fn lookup_musicbrainz_detail(id: String) -> Result<LookupRelease, String> {
    crate::lookup::musicbrainz_detail(id).await
}

/// Search Discogs for releases matching free-text `query`. `token` is the
/// user's personal Discogs access token, kept in the frontend's
/// `localStorage` — never persisted here.
#[tauri::command]
pub async fn lookup_discogs(query: String, token: String) -> Result<Vec<LookupCandidate>, String> {
    crate::lookup::discogs_search(query, token).await
}

/// Full track list (and cover art, if any) for a Discogs release chosen from
/// [`lookup_discogs`]'s results.
#[tauri::command]
pub async fn lookup_discogs_detail(id: String, token: String) -> Result<LookupRelease, String> {
    crate::lookup::discogs_detail(id, token).await
}
