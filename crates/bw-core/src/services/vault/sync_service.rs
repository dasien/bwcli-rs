//! Vault synchronization.
//!
//! A thin wrapper over the SDK's [`bitwarden_sync::SyncClient`], which owns the
//! sync *run*: a lock so two syncs cannot interleave, the revision-date
//! short-circuit, `last_sync` bookkeeping, and error dispatch. This module only
//! decides *what* to persist, by registering handlers.
//!
//! ## Why this replaced a hand-rolled sync
//!
//! The previous version called `sync_api().get()` directly and did all the
//! bookkeeping itself. Two things it got wrong, both free from `SyncClient`:
//!
//! - **`last_sync` was stamped after the fetch, not before.** `SyncClient`
//!   captures the time *before* any server call, and documents why: a change
//!   committed during the sync window has a revision date earlier than a
//!   finish-time `last_sync`, so the next revision check reports "nothing
//!   changed" and that change is never picked up. Our version had exactly that
//!   race.
//! - **No lock.** Harmless for one-shot CLI invocations, not for `bw serve`.
//!
//! It also means each SDK release *removes* code here rather than diverging
//! from it: folders and sends are now the SDK's own handlers, and a
//! `CipherSyncHandler` upstream would delete another one.
//!
//! ## Where `last_sync` lives
//!
//! `SyncClient` keeps it as an SDK setting, and that is **authoritative** —
//! its revision check reads it, so a second copy could disagree and suppress a
//! needed sync. We additionally *mirror* it into `data.json` under the
//! TypeScript CLI's `lastSync` key, because that file is a shared profile: a TS
//! CLI reading a vault we synced should not re-download it. The mirror is
//! written after the fact and never read back by us.

use super::errors::VaultError;
use super::sync_handlers::{CipherSyncHandler, CollectionSyncHandler, OrganizationSyncHandler};
use crate::services::storage::{AccountManager, JsonFileStorage, Storage, StorageKey};
use bitwarden_core::{Client, FromClient};
use bitwarden_send::SendSyncHandler;
use bitwarden_sync::{SyncClientExt, SyncRequest};
use bitwarden_vault::FolderSyncHandler;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Service for vault synchronization operations
pub struct SyncService {
    storage: Arc<Mutex<JsonFileStorage>>,
    sdk: Arc<Client>,
}

impl SyncService {
    pub fn new(storage: Arc<Mutex<JsonFileStorage>>, sdk: Arc<Client>) -> Self {
        Self { storage, sdk }
    }

    /// Sync vault from server.
    ///
    /// Skips the download when the server's account revision date is no newer
    /// than the last sync, unless `force` is set.
    ///
    /// # Returns
    /// The `last_sync` timestamp as RFC3339, whether a download happened or not.
    pub async fn sync(&self, force: bool) -> Result<String, VaultError> {
        // Authentication means "the SDK has tokens"; an expired access token
        // still counts, because its token handler renews transparently.
        if !crate::services::sdk_session::is_authenticated(&self.sdk).await {
            return Err(VaultError::NotAuthenticated);
        }

        let account_manager = Arc::new(AccountManager::new(Arc::clone(&self.storage)));

        let sync = self.sdk.sync();

        // Handler order is the order registered, and `SyncClient` stops at the
        // first failure. Ciphers and folders first: they are the vault proper,
        // and the most likely thing a user is waiting on.
        sync.register_sync_handler(Arc::new(
            CipherSyncHandler::new(&self.sdk).map_err(VaultError::StorageError)?,
        ));
        // The SDK's own handlers. `SendSyncHandler` writes through
        // `Repository<Send>`, which is our `JsonSendRepository`, so sends still
        // land in `data.json` under the TypeScript CLI's key.
        sync.register_sync_handler(Arc::new(FolderSyncHandler::from_client(&self.sdk)));
        sync.register_sync_handler(Arc::new(SendSyncHandler::from_client(&self.sdk)));
        sync.register_sync_handler(Arc::new(CollectionSyncHandler::new(
            Arc::clone(&self.storage),
            Arc::clone(&account_manager),
        )));
        sync.register_sync_handler(Arc::new(OrganizationSyncHandler::new(
            Arc::clone(&self.sdk),
            Arc::clone(&self.storage),
            Arc::clone(&account_manager),
        )));

        sync.sync(SyncRequest {
            force,
            exclude_subdomains: None,
        })
        .await
        .map_err(|e| VaultError::ApiError(e.to_string()))?;

        let last_sync = sync
            .last_sync()
            .await
            .map(|t| t.to_rfc3339())
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

        self.mirror_last_sync(&account_manager, &last_sync).await;

        Ok(last_sync)
    }

    /// Copy `last_sync` into `data.json` for the TypeScript CLI's benefit.
    ///
    /// Best-effort: the authoritative value is the SDK setting, so a failure
    /// here costs a redundant sync in the *other* CLI and nothing more. Failing
    /// the whole sync over it would be worse.
    async fn mirror_last_sync(&self, account_manager: &AccountManager, last_sync: &str) {
        let Ok(Some(user_id)) = account_manager.get_active_user_id().await else {
            return;
        };

        let mut storage = self.storage.lock().await;
        if let Err(e) = storage
            .set(
                &StorageKey::UserLastSync.format(Some(&user_id)),
                &last_sync.to_string(),
            )
            .await
        {
            tracing::debug!("Could not mirror lastSync into data.json: {e}");
        }
    }

    /// The last sync timestamp, as RFC3339.
    ///
    /// Read from the SDK setting `SyncClient` maintains — the same value its
    /// revision check uses — rather than the `data.json` mirror, so `bw status`
    /// cannot disagree with the sync decision.
    pub async fn get_last_sync(&self) -> Result<Option<String>, VaultError> {
        Ok(self.sdk.sync().last_sync().await.map(|t| t.to_rfc3339()))
    }

    pub fn storage(&self) -> &Arc<Mutex<JsonFileStorage>> {
        &self.storage
    }
}
