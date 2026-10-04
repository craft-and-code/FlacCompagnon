//! Discogs credentials persisted exclusively in the operating system's vault.
//!
//! Only status crosses back to the webview. Reads and changes run off-runtime;
//! one lock serializes migration, replacement and deletion without caching a
//! secret. Platform errors are sanitized because some contain secret bytes.

use std::sync::{Arc, Mutex};

use keyring::v1::{Entry, Error};
use serde::Serialize;

const SERVICE: &str = "com.flaccompagnon.app";
const ACCOUNT: &str = "discogs-personal-token";
// Credential Manager permits more than this; a personal API token needs far
// less. Bound inputs before sending them to a native credential API.
const MAX_TOKEN_BYTES: usize = 1024;

/// Presence of a saved credential, never its contents.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct DiscogsCredentialStatus {
    /// Whether Discogs can use a token from the system credential store.
    pub configured: bool,
}

trait Vault: Send {
    fn read(&self) -> Result<Option<String>, String>;
    fn write(&self, token: &str) -> Result<(), String>;
    fn delete(&self) -> Result<(), String>;
}

struct NativeVault;

impl NativeVault {
    fn entry(&self) -> Result<Entry, String> {
        Entry::new(SERVICE, ACCOUNT).map_err(vault_error)
    }
}

impl Vault for NativeVault {
    fn read(&self) -> Result<Option<String>, String> {
        match self.entry()?.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(Error::NoEntry) => Ok(None),
            Err(error) => Err(vault_error(error)),
        }
    }

    fn write(&self, token: &str) -> Result<(), String> {
        self.entry()?.set_password(token).map_err(vault_error)
    }

    fn delete(&self) -> Result<(), String> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(error) => Err(vault_error(error)),
        }
    }
}

fn vault_error(error: Error) -> String {
    match error {
        Error::NoStorageAccess(_) => {
            "The system credential store is locked or access was denied. Unlock it and try again."
        }
        Error::NoDefaultStore | Error::NotSupportedByStore(_) => {
            "The system credential store is unavailable. Enable Keychain, Credential Manager or Secret Service, then restart the app."
        }
        _ => "The system credential store operation failed. No plaintext fallback was used.",
    }
    .to_string()
}

fn validate_token(token: &str) -> Result<&str, String> {
    let token = token.trim();
    if token.is_empty()
        || token.len() > MAX_TOKEN_BYTES
        || !token.bytes().all(|b| b.is_ascii_graphic())
    {
        return Err(
            "Discogs token must be nonempty ASCII text, without spaces, and at most 1024 bytes."
                .into(),
        );
    }
    Ok(token)
}

/// Managed access to one application credential, shared by blocking workers.
#[derive(Clone)]
pub struct CredentialStore(Arc<Mutex<Box<dyn Vault>>>);

impl Default for CredentialStore {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Box::new(NativeVault))))
    }
}

impl CredentialStore {
    fn with_vault<T>(
        &self,
        operation: impl FnOnce(&dyn Vault) -> Result<T, String>,
    ) -> Result<T, String> {
        let vault = self
            .0
            .lock()
            .map_err(|_| "Credential store access was interrupted. Restart the app.".to_string())?;
        operation(vault.as_ref())
    }

    /// Read presence while keeping the token entirely in the backend.
    pub fn status(&self) -> Result<DiscogsCredentialStatus, String> {
        self.with_vault(|vault| {
            Ok(DiscogsCredentialStatus {
                configured: vault.read()?.is_some(),
            })
        })
    }

    /// Explicitly replace a saved token in the native store.
    pub fn save(&self, token: &str) -> Result<DiscogsCredentialStatus, String> {
        let token = validate_token(token)?;
        self.with_vault(|vault| {
            vault.write(token)?;
            Ok(DiscogsCredentialStatus { configured: true })
        })
    }

    /// Import a legacy token once, preserving any newer native credential.
    pub fn migrate(&self, token: &str) -> Result<DiscogsCredentialStatus, String> {
        self.with_vault(|vault| {
            if vault.read()?.is_none() {
                vault.write(validate_token(token)?)?;
            }
            Ok(DiscogsCredentialStatus { configured: true })
        })
    }

    /// Forget the saved token; an already absent entry is also success.
    pub fn delete(&self) -> Result<DiscogsCredentialStatus, String> {
        self.with_vault(|vault| {
            vault.delete()?;
            Ok(DiscogsCredentialStatus { configured: false })
        })
    }

    /// Read and validate a token only when an explicit Discogs request needs it.
    pub fn token_for_request(&self) -> Result<String, String> {
        self.with_vault(|vault| {
            let token = vault.read()?.ok_or_else(|| {
                "No Discogs token is saved. Save one in the search settings first.".to_string()
            })?;
            Ok(validate_token(&token)?.to_string())
        })
    }
}

#[cfg(test)]
#[path = "../tests/unit/credentials.rs"]
mod tests;
