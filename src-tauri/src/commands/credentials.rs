//! Status and explicit changes to the Discogs system credential entry.

use crate::credentials::{CredentialStore, DiscogsCredentialStatus};

async fn run(
    store: tauri::State<'_, CredentialStore>,
    operation: impl FnOnce(CredentialStore) -> Result<DiscogsCredentialStatus, String> + Send + 'static,
) -> Result<DiscogsCredentialStatus, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || operation(store))
        .await
        .map_err(|_| "Credential worker failed. Restart the app.".to_string())?
}

/// Report whether a native credential exists without returning the secret.
#[tauri::command]
pub async fn discogs_credential_status(
    store: tauri::State<'_, CredentialStore>,
) -> Result<DiscogsCredentialStatus, String> {
    run(store, |store| store.status()).await
}

/// Save a token after the user explicitly selects Save token.
#[tauri::command]
pub async fn save_discogs_token(
    store: tauri::State<'_, CredentialStore>,
    token: String,
) -> Result<DiscogsCredentialStatus, String> {
    run(store, move |store| store.save(&token)).await
}

/// Remove the application credential from the native store.
#[tauri::command]
pub async fn delete_discogs_token(
    store: tauri::State<'_, CredentialStore>,
) -> Result<DiscogsCredentialStatus, String> {
    run(store, |store| store.delete()).await
}

/// Move an old frontend token to native storage without replacing a newer one.
#[tauri::command]
pub async fn migrate_discogs_token(
    store: tauri::State<'_, CredentialStore>,
    token: String,
) -> Result<DiscogsCredentialStatus, String> {
    run(store, move |store| store.migrate(&token)).await
}
