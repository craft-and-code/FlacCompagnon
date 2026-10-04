//! Thin command wrappers over the online providers in [`crate::lookup`].
//!
//! These are the only commands that reach the network, and only ever from an
//! explicit "Search online" click. They hold no logic — the providers do —
//! but they are what makes the surface visible in one place: if a command
//! that talks to the internet is ever added, it belongs here and nowhere else.

use crate::credentials::CredentialStore;
use crate::lookup::{LookupCandidate, LookupRelease};

async fn saved_token(store: tauri::State<'_, CredentialStore>) -> Result<String, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || store.token_for_request())
        .await
        .map_err(|_| "Credential worker failed. Restart the app.".to_string())?
}

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

/// Search Discogs with a token read from the system credential store.
/// The secret is never supplied by or returned to the lookup frontend.
#[tauri::command]
pub async fn lookup_discogs(
    query: String,
    store: tauri::State<'_, CredentialStore>,
) -> Result<Vec<LookupCandidate>, String> {
    let token = saved_token(store).await?;
    crate::lookup::discogs_search(query, token).await
}

/// Full track list (and cover art, if any) for a Discogs release chosen from
/// [`lookup_discogs`]'s results.
#[tauri::command]
pub async fn lookup_discogs_detail(
    id: String,
    store: tauri::State<'_, CredentialStore>,
) -> Result<LookupRelease, String> {
    let token = saved_token(store).await?;
    crate::lookup::discogs_detail(id, token).await
}
