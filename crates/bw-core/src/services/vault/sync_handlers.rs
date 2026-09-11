//! `SyncHandler` implementations for the parts of a sync the SDK does not cover.
//!
//! [`bitwarden_sync::SyncClient`] owns the sync *run* — the lock, the
//! revision-date short-circuit, `last_sync` bookkeeping, error dispatch — and
//! hands the raw response to registered handlers. The SDK ships three:
//! `CryptoSyncHandler`, `FolderSyncHandler` and `SendSyncHandler`. Its own
//! `new_with_sync` still carries a `// TODO: Add more sync handlers here!`, so
//! the rest is ours to supply.
//!
//! What is here, and why each one cannot be the SDK's:
//!
//! | Handler | Why it is ours |
//! |---|---|
//! | [`CipherSyncHandler`] | there is no `CipherSyncHandler` in the SDK yet |
//! | [`CollectionSyncHandler`] | `Collection` is not a registered repository item |
//! | [`OrganizationSyncHandler`] | the SDK has no `Organization` domain type at all |
//!
//! Delete [`CipherSyncHandler`] the moment the SDK grows one; it deliberately
//! mirrors `FolderSyncHandler` line for line so the diff is obvious.
//!
//! Handlers persist and must not fail for reasons the user can do nothing about:
//! a single undeserializable record is logged and skipped rather than failing
//! the whole sync, matching what the SDK's handlers do. An error here aborts the
//! run and `SyncClient` leaves `last_sync` untouched, so the next invocation
//! retries.

use crate::models::vault::Organization;
use crate::services::storage::{AccountManager, JsonFileStorage, Storage, StorageKey};
use bitwarden_api_api::models::SyncResponseModel;
use bitwarden_collections::collection::Collection;
use bitwarden_core::Client;
use bitwarden_core::key_management::crypto::InitOrgCryptoRequest;
use bitwarden_state::repository::Repository;
use bitwarden_sync::{SyncHandler, SyncHandlerError};
use bitwarden_vault::{Cipher, CipherId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Persists ciphers into the SDK's `Repository<Cipher>`.
///
/// The SDK has `FolderSyncHandler` and `SendSyncHandler` but no cipher
/// equivalent, so this fills the gap. `replace_all` mirrors a full sync: the
/// server response is authoritative, and an item deleted server-side must
/// disappear locally rather than linger.
pub struct CipherSyncHandler {
    repository: Arc<dyn Repository<Cipher>>,
}

impl CipherSyncHandler {
    pub fn new(sdk: &Client) -> Result<Self, String> {
        Ok(Self {
            repository: sdk
                .platform()
                .state()
                .get::<Cipher>()
                .map_err(|e| e.to_string())?,
        })
    }
}

#[async_trait::async_trait]
impl SyncHandler for CipherSyncHandler {
    async fn on_sync(&self, response: &SyncResponseModel) -> Result<(), SyncHandlerError> {
        let Some(api_ciphers) = response.ciphers.as_ref() else {
            return Ok(());
        };

        let ciphers: Vec<(CipherId, Cipher)> = api_ciphers
            .iter()
            .filter_map(|c| {
                // One bad record must not cost the user the whole sync; the SDK's
                // own handlers log and skip for the same reason.
                Cipher::try_from(c.clone())
                    .inspect_err(
                        |e| tracing::error!(id = ?c.id, error = ?e, "Failed to deserialize cipher"),
                    )
                    .ok()
                    .and_then(|cipher| {
                        let id = cipher.id.or_else(|| {
                            tracing::error!("Skipping cipher with missing id");
                            None
                        })?;
                        Some((id, cipher))
                    })
            })
            .collect();

        self.repository.replace_all(ciphers).await?;
        Ok(())
    }
}

/// Persists collections to `data.json`.
///
/// `Collection` is not among the SDK's registered repository items (`Cipher`,
/// `Folder`, `SettingItem`, `OrganizationSharedKey`, `Send`), so there is no
/// SDK-managed store to write to. The TypeScript CLI's key is used so the two
/// CLIs can share a profile.
pub struct CollectionSyncHandler {
    storage: Arc<Mutex<JsonFileStorage>>,
    account_manager: Arc<AccountManager>,
}

impl CollectionSyncHandler {
    pub fn new(storage: Arc<Mutex<JsonFileStorage>>, account_manager: Arc<AccountManager>) -> Self {
        Self {
            storage,
            account_manager,
        }
    }
}

#[async_trait::async_trait]
impl SyncHandler for CollectionSyncHandler {
    async fn on_sync(&self, response: &SyncResponseModel) -> Result<(), SyncHandlerError> {
        let Some(api_collections) = response.collections.as_ref() else {
            return Ok(());
        };

        let collections: HashMap<String, Collection> = api_collections
            .iter()
            .filter_map(|c| {
                Collection::try_from(c.clone())
                    .inspect_err(|e| {
                        tracing::error!(id = ?c.id, error = ?e, "Failed to deserialize collection")
                    })
                    .ok()
                    .and_then(|collection| collection.id.map(|id| (id.to_string(), collection)))
            })
            .collect();

        let user_id = require_user_id(&self.account_manager).await?;
        let mut storage = self.storage.lock().await;
        storage
            .set(
                &StorageKey::UserCollections.format(Some(&user_id)),
                &collections,
            )
            .await
            .map_err(|e| -> SyncHandlerError { e.to_string().into() })?;

        Ok(())
    }
}

/// Persists organizations to `data.json` and loads their keys into the key store.
///
/// Two jobs, because they share one input. The SDK has no `Organization` domain
/// type, so the records are ours; the keys, however, go through the SDK's
/// `initialize_org_crypto`.
pub struct OrganizationSyncHandler {
    sdk: Arc<Client>,
    storage: Arc<Mutex<JsonFileStorage>>,
    account_manager: Arc<AccountManager>,
}

impl OrganizationSyncHandler {
    pub fn new(
        sdk: Arc<Client>,
        storage: Arc<Mutex<JsonFileStorage>>,
        account_manager: Arc<AccountManager>,
    ) -> Self {
        Self {
            sdk,
            storage,
            account_manager,
        }
    }
}

#[async_trait::async_trait]
impl SyncHandler for OrganizationSyncHandler {
    async fn on_sync(&self, response: &SyncResponseModel) -> Result<(), SyncHandlerError> {
        // Organizations arrive on the profile, not as a top-level list. The
        // server sends both `organizations` and `organizationsNew`; newer
        // clients prefer the latter and fall back.
        let profile_organizations = response
            .profile
            .as_deref()
            .and_then(|profile| {
                profile
                    .organizations_new
                    .as_ref()
                    .or(profile.organizations.as_ref())
            })
            .map(Vec::as_slice)
            .unwrap_or_default();

        let organizations: HashMap<String, Organization> = profile_organizations
            .iter()
            .filter_map(Organization::from_api)
            .map(|o| (o.id.clone(), o))
            .collect();

        let organization_keys = Organization::keys_from_api(profile_organizations);

        let user_id = require_user_id(&self.account_manager).await?;
        {
            let mut storage = self.storage.lock().await;
            storage
                .set(
                    &StorageKey::UserOrganizations.format(Some(&user_id)),
                    &organizations,
                )
                .await
                .map_err(|e| -> SyncHandlerError { e.to_string().into() })?;
        }

        // Load organization keys into the key store and persist them for later
        // unlocks. Without this, organization-owned items cannot be decrypted and
        // nothing can be shared *into* an organization, because sharing
        // re-encrypts the item under the organization's key.
        //
        // Best-effort, deliberately: unwrapping these needs the user's private
        // key, so an incompletely unlocked vault would otherwise make `sync`
        // fail — and `sync` is how you recover from a bad local state. Failing
        // here leaves organization items undecryptable, which is what they
        // already were; failing the sync would also lose the personal vault.
        if !organization_keys.is_empty()
            && let Err(e) = self
                .sdk
                .crypto()
                .initialize_org_crypto(InitOrgCryptoRequest { organization_keys })
                .await
        {
            tracing::warn!(
                "Could not load organization keys ({e}); organization items will not \
                 decrypt and items cannot be shared into an organization. Unlock the \
                 vault and sync again."
            );
        }

        Ok(())
    }
}

/// The active user id, or a handler error naming why there isn't one.
///
/// Storage keys are user-namespaced, so a handler cannot write anything without
/// this. It should be impossible here — `sync` requires authentication — but
/// failing loudly beats writing to a key with an empty namespace.
async fn require_user_id(account_manager: &AccountManager) -> Result<String, SyncHandlerError> {
    account_manager
        .get_active_user_id()
        .await
        .map_err(|e| -> SyncHandlerError { e.to_string().into() })?
        .ok_or_else(|| -> SyncHandlerError { "no active account".into() })
}
