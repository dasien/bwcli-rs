//! `Repository<Send>` backed by the CLI's own state file.
//!
//! The SDK's send CRUD (`SendClient::{list, get, create, edit, delete}`) reads
//! and writes through a `Repository<Send>` obtained from the state registry.
//! Registering this adapter as *client-managed* keeps `data.json` the single
//! source of truth; without it the SDK falls back to an in-memory database that
//! is empty at the start of every CLI invocation, so `bw send list` would
//! always come back empty.
//!
//! Sends are stored under the TypeScript CLI's key, `user_{id}_send_sends`, as
//! a `HashMap<id, Send>`.

use crate::services::storage::{AccountManager, JsonFileStorage, Storage, StorageKey};
use bitwarden_send::{Send, SendId};
use bitwarden_state::repository::{Repository, RepositoryError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct JsonSendRepository {
    storage: Arc<Mutex<JsonFileStorage>>,
    account_manager: Arc<AccountManager>,
}

impl JsonSendRepository {
    pub fn new(storage: Arc<Mutex<JsonFileStorage>>, account_manager: Arc<AccountManager>) -> Self {
        Self {
            storage,
            account_manager,
        }
    }

    async fn key(&self) -> Result<String, RepositoryError> {
        let user_id = self
            .account_manager
            .get_active_user_id()
            .await
            .map_err(|e| RepositoryError::Internal(e.to_string()))?
            .ok_or_else(|| RepositoryError::Internal("no active user".to_string()))?;

        Ok(StorageKey::UserSends.format(Some(&user_id)))
    }

    async fn load(&self) -> Result<HashMap<String, Send>, RepositoryError> {
        let key = self.key().await?;
        let storage = self.storage.lock().await;

        Ok(storage
            .get::<HashMap<String, Send>>(&key)
            .map_err(|e| RepositoryError::Internal(e.to_string()))?
            .unwrap_or_default())
    }

    async fn store(&self, sends: HashMap<String, Send>) -> Result<(), RepositoryError> {
        let key = self.key().await?;
        let mut storage = self.storage.lock().await;

        storage
            .set(&key, &sends)
            .await
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;
        storage
            .flush()
            .await
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl Repository<Send> for JsonSendRepository {
    async fn get(&self, key: SendId) -> Result<Option<Send>, RepositoryError> {
        Ok(self.load().await?.remove(&key.to_string()))
    }

    async fn list(&self) -> Result<Vec<Send>, RepositoryError> {
        Ok(self.load().await?.into_values().collect())
    }

    async fn set(&self, key: SendId, value: Send) -> Result<(), RepositoryError> {
        let mut sends = self.load().await?;
        sends.insert(key.to_string(), value);
        self.store(sends).await
    }

    async fn set_bulk(&self, values: Vec<(SendId, Send)>) -> Result<(), RepositoryError> {
        let mut sends = self.load().await?;
        for (key, value) in values {
            sends.insert(key.to_string(), value);
        }
        self.store(sends).await
    }

    async fn remove(&self, key: SendId) -> Result<(), RepositoryError> {
        let mut sends = self.load().await?;
        sends.remove(&key.to_string());
        self.store(sends).await
    }

    async fn remove_bulk(&self, keys: Vec<SendId>) -> Result<(), RepositoryError> {
        let mut sends = self.load().await?;
        for key in keys {
            sends.remove(&key.to_string());
        }
        self.store(sends).await
    }

    async fn remove_all(&self) -> Result<(), RepositoryError> {
        self.store(HashMap::new()).await
    }
}
