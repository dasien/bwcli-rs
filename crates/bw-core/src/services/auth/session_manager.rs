use crate::services::storage::{JsonFileStorage, Storage, StorageKey};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Account and device state that is not part of the session itself.
///
/// Session keys are minted and consumed by `bitwarden-unlock` (see
/// `services::sdk_session`); what remains here is the device identifier, token
/// lookup, and login-state checks.
pub struct SessionManager {
    storage: Arc<Mutex<JsonFileStorage>>,
}

impl SessionManager {
    /// Create new session manager
    pub fn new(storage: Arc<Mutex<JsonFileStorage>>) -> Self {
        Self { storage }
    }







    /// Check if user is logged in (has auth state)
    ///
    /// Note: This is a legacy check that doesn't use the new namespaced keys.
    /// Prefer using AccountManager::is_logged_in() for accurate state checking.
    pub async fn is_logged_in(&self) -> Result<bool> {
        let storage = self.storage.lock().await;

        // Check for active account ID first (new format)
        let active_id_key = StorageKey::GlobalActiveAccountId.format(None);
        let active_id: Option<serde_json::Value> = storage.get(&active_id_key)?;

        if let Some(serde_json::Value::String(user_id)) = active_id {
            if !user_id.is_empty() {
                // Check if user has access token
                let token_key = StorageKey::UserAccessToken.format(Some(&user_id));
                let token: Option<serde_json::Value> = storage.get(&token_key)?;
                if matches!(token, Some(serde_json::Value::String(s)) if !s.is_empty()) {
                    return Ok(true);
                }
            }
        }

        // Fall back to legacy format check
        let access_token: Option<String> = storage.get("accessToken")?;
        Ok(access_token.is_some())
    }

    /// Get the active user's access token
    ///
    /// Returns the access token for the currently active user, or None if not logged in.
    pub async fn get_access_token(&self) -> Result<Option<String>> {
        let storage = self.storage.lock().await;

        // Get active user ID
        let active_id_key = StorageKey::GlobalActiveAccountId.format(None);
        let active_id: Option<serde_json::Value> = storage.get(&active_id_key)?;

        let user_id = match active_id {
            Some(serde_json::Value::String(id)) if !id.is_empty() => id,
            _ => return Ok(None),
        };

        // Get access token for this user
        let token_key = StorageKey::UserAccessToken.format(Some(&user_id));
        let token: Option<String> = storage.get(&token_key)?;

        Ok(token)
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
