use crate::services::storage::{JsonFileStorage, Storage, StorageKey};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// The device identifier, which is neither session nor vault state.
///
/// Session keys are minted and consumed by `bitwarden-unlock` (see
/// `services::sdk_session`), and tokens and login state belong to the SDK's
/// state database, so this is all that is left.
pub struct SessionManager {
    storage: Arc<Mutex<JsonFileStorage>>,
}

impl SessionManager {
    /// Create new session manager
    pub fn new(storage: Arc<Mutex<JsonFileStorage>>) -> Self {
        Self { storage }
    }







    /// Get device ID from storage or generate new one
    pub async fn get_or_create_device_id(&self) -> Result<String> {
        let mut storage = self.storage.lock().await;

        // Try to load existing device ID from new key format
        let device_key = StorageKey::DeviceId.format(None);
        if let Some(device_id) = storage.get::<String>(&device_key)? {
            return Ok(device_id);
        }

        // Fall back to legacy key
        if let Some(device_id) = storage.get::<String>("deviceId")? {
            // Migrate to new key format
            storage.set(&device_key, &device_id).await?;
            return Ok(device_id);
        }

        // Generate new device ID
        let device_id = uuid::Uuid::new_v4().to_string();
        storage.set(&device_key, &device_id).await?;
        storage.flush().await?;

        Ok(device_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;




    #[tokio::test]
    async fn test_device_id_persistence() {
        let temp_dir = tempdir().unwrap();
        let storage = Arc::new(Mutex::new(
            JsonFileStorage::new(Some(temp_dir.path().to_path_buf())).unwrap(),
        ));
        let session_mgr = SessionManager::new(storage);

        // First call should generate new ID
        let id1 = session_mgr.get_or_create_device_id().await.unwrap();
        assert!(!id1.is_empty());

        // Second call should return same ID
        let id2 = session_mgr.get_or_create_device_id().await.unwrap();
        assert_eq!(id1, id2);
    }
}
