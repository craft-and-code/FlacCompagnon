use super::*;

#[derive(Default)]
struct MemoryVault {
    token: Mutex<Option<String>>,
    fail_writes: bool,
}

impl Vault for Arc<MemoryVault> {
    fn read(&self) -> Result<Option<String>, String> {
        Ok(self.token.lock().unwrap().clone())
    }

    fn write(&self, token: &str) -> Result<(), String> {
        if self.fail_writes {
            return Err("Store locked".into());
        }
        *self.token.lock().unwrap() = Some(token.into());
        Ok(())
    }

    fn delete(&self) -> Result<(), String> {
        *self.token.lock().unwrap() = None;
        Ok(())
    }
}

fn store(vault: &Arc<MemoryVault>) -> CredentialStore {
    CredentialStore(Arc::new(Mutex::new(Box::new(vault.clone()))))
}

#[test]
fn saved_status_never_serializes_the_secret() {
    let vault = Arc::new(MemoryVault::default());
    let store = store(&vault);
    assert!(!store.status().unwrap().configured);
    let status = store.save("  synthetic-token  ").unwrap();
    assert_eq!(
        serde_json::to_value(status).unwrap(),
        serde_json::json!({"configured": true})
    );
    assert_eq!(store.token_for_request().unwrap(), "synthetic-token");
    assert_eq!(
        vault.token.lock().unwrap().as_deref(),
        Some("synthetic-token")
    );
}

#[test]
fn deleting_a_token_prevents_future_discogs_requests() {
    let store = store(&Arc::new(MemoryVault::default()));
    store.save("synthetic-token").unwrap();
    assert!(!store.delete().unwrap().configured);
    assert!(store.token_for_request().is_err());
    assert!(!store.delete().unwrap().configured);
}

#[test]
fn native_tokens_with_outer_spaces_are_normalized_before_authorization() {
    let vault = Arc::new(MemoryVault {
        token: Mutex::new(Some("  synthetic-token  ".into())),
        ..Default::default()
    });
    assert_eq!(
        store(&vault).token_for_request().unwrap(),
        "synthetic-token"
    );
}

#[test]
fn legacy_migration_cannot_replace_a_newer_saved_token() {
    let store = store(&Arc::new(MemoryVault::default()));
    store.save("new-token").unwrap();
    store.migrate("old-token").unwrap();
    assert_eq!(store.token_for_request().unwrap(), "new-token");
}

#[test]
fn malformed_legacy_tokens_do_not_disable_an_existing_native_credential() {
    let store = store(&Arc::new(MemoryVault::default()));
    store.save("new-token").unwrap();
    for legacy in ["", "old token", "é", "token\r\nInjected: value"] {
        assert!(store.migrate(legacy).unwrap().configured);
        assert_eq!(store.token_for_request().unwrap(), "new-token");
    }
    assert!(
        store
            .migrate(&"a".repeat(MAX_TOKEN_BYTES + 1))
            .unwrap()
            .configured
    );
    assert_eq!(store.token_for_request().unwrap(), "new-token");
}

#[test]
fn repeated_legacy_migrations_only_import_the_first_token() {
    let store = store(&Arc::new(MemoryVault::default()));
    store.migrate("old-token").unwrap();
    store.migrate("different-old-token").unwrap();
    assert_eq!(store.token_for_request().unwrap(), "old-token");
}

#[test]
fn concurrent_legacy_imports_cannot_overwrite_each_other() {
    let store = store(&Arc::new(MemoryVault::default()));
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let handles: Vec<_> = ["first-token", "second-token"]
        .into_iter()
        .map(|token| {
            let store = store.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store.migrate(token).unwrap();
            })
        })
        .collect();
    barrier.wait();
    for handle in handles {
        handle.join().unwrap();
    }
    let winner = store.token_for_request().unwrap();
    assert!(matches!(winner.as_str(), "first-token" | "second-token"));
    store.migrate("later-token").unwrap();
    assert_eq!(store.token_for_request().unwrap(), winner);
}

#[test]
fn locked_vault_never_claims_a_token_was_saved() {
    let vault = Arc::new(MemoryVault {
        fail_writes: true,
        ..Default::default()
    });
    let store = store(&vault);
    assert!(store.save("synthetic-token").is_err());
    assert!(store.migrate("synthetic-token").is_err());
    assert!(!store.status().unwrap().configured);
    assert!(vault.token.lock().unwrap().is_none());
}

#[test]
fn invalid_token_is_rejected_before_writing_native_storage() {
    let vault = Arc::new(MemoryVault::default());
    let store = store(&vault);
    for invalid in ["", " ", "token\r\nInjected: value", "to ken", "é", "a\0b"] {
        assert!(store.save(invalid).is_err());
        assert!(store.migrate(invalid).is_err());
    }
    assert!(store.save(&"a".repeat(MAX_TOKEN_BYTES + 1)).is_err());
    assert!(vault.token.lock().unwrap().is_none());
}

#[test]
fn platform_errors_cannot_expose_secret_bytes() {
    let secret = b"synthetic-token".to_vec();
    assert!(!vault_error(Error::BadEncoding(secret.clone())).contains("synthetic-token"));
    assert!(!vault_error(Error::BadDataFormat(
        secret,
        Box::new(std::io::Error::other("synthetic-token"))
    ))
    .contains("synthetic-token"));
}

#[test]
#[ignore = "Writes and deletes an isolated synthetic OS-vault entry; requires an unlocked credential store"]
fn native_vault_round_trip_and_cleanup() {
    let unique = tempfile::tempdir().unwrap();
    let service = format!(
        "{SERVICE}.test.{}",
        unique.path().file_name().unwrap().to_string_lossy()
    );
    let entry = Entry::new(&service, "synthetic-test").unwrap();
    struct Cleanup(Entry);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.delete_credential();
        }
    }
    let cleanup = Cleanup(entry);
    cleanup.0.set_password("synthetic-token").unwrap();
    assert_eq!(cleanup.0.get_password().unwrap(), "synthetic-token");
    cleanup.0.delete_credential().unwrap();
    assert!(matches!(cleanup.0.get_password(), Err(Error::NoEntry)));
}
