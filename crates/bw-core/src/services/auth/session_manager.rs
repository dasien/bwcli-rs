use crate::models::auth::{SessionKey, SessionKeyError};
use crate::services::storage::{JsonFileStorage, Storage};
use anyhow::Result;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Session manager for handling BW_SESSION session keys
///
/// Manages the lifecycle of session keys used to encrypt local storage.
/// Session keys can come from:
/// 1. BW_SESSION environment variable (highest priority)
/// 2. Storage (persisted from previous session)
/// 3. Newly generated (on login/unlock)
pub struct SessionManager {
    storage: Arc<Mutex<JsonFileStorage>>,
}

impl SessionManager {
    /// Create new session manager
    pub fn new(storage: Arc<Mutex<JsonFileStorage>>) -> Self {
        Self { storage }
    }

    /// Generate a new session key
    ///
    /// Uses cryptographically secure random number generator (OsRng)
    pub fn generate_session_key() -> SessionKey {
        SessionKey::generate()
    }

    /// Save session key to storage
    ///
    /// Note: The session key itself is not stored - we only generate it.
    /// Users are expected to export it to BW_SESSION environment variable.
    /// This method is kept for potential future use.
    pub async fn save_session_key(&self, key: &SessionKey) -> Result<()> {
        let mut storage = self.storage.lock().await;
        storage.set("sessionKeyHint", &key.to_base64()).await?;
        storage.flush().await?;
        Ok(())
    }

    /// Load session key from BW_SESSION environment variable
    ///
    /// # Returns
    /// - `Ok(Some(key))` if BW_SESSION is set and valid
    /// - `Ok(None)` if BW_SESSION is not set
    /// - `Err` if BW_SESSION is set but invalid
    pub async fn load_session_key(&self) -> Result<Option<SessionKey>> {
        // Check environment variable first (highest priority)
        if let Ok(session_str) = env::var("BW_SESSION") {
            if !session_str.is_empty() {
                let key = SessionKey::from_base64(&session_str)
                    .map_err(|e| anyhow::anyhow!("Invalid BW_SESSION: {}", e))?;
                return Ok(Some(key));
            }
        }

        // No session key available
        Ok(None)
    }

    /// Clear session key from storage
    ///
    /// Note: This only clears the storage hint. Users must manually unset
    /// the BW_SESSION environment variable.
    pub async fn clear_session_key(&self) -> Result<()> {
        let mut storage = self.storage.lock().await;
        storage.remove("sessionKeyHint").await?;
        storage.flush().await?;
        Ok(())
    }

    /// Format session key for export to environment variable
    ///
    /// Returns the command that users should run to set BW_SESSION
    pub fn format_for_export(key: &SessionKey) -> String {
        key.to_base64()
    }

    /// Validate session key format
    ///
    /// Used to validate --session flag input
    pub fn validate_session_key(key_str: &str) -> Result<SessionKey, SessionKeyError> {
        SessionKey::from_base64(key_str)
    }

    /// Check if user is logged in (has auth state)
    pub async fn is_logged_in(&self) -> Result<bool> {
        let storage = self.storage.lock().await;
        let access_token: Option<String> = storage.get_secure("accessToken").await?;
        Ok(access_token.is_some())
    }

    /// Get device ID from storage or generate new one
    pub async fn get_or_create_device_id(&self) -> Result<String> {
        let mut storage = self.storage.lock().await;

        // Try to load existing device ID
        if let Some(device_id) = storage.get::<String>("deviceId")? {
            return Ok(device_id);
        }

        // Generate new device ID
        let device_id = uuid::Uuid::new_v4().to_string();
        storage.set("deviceId", &device_id).await?;
        storage.flush().await?;

        Ok(device_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_session_key() {
        let key1 = SessionManager::generate_session_key();
        let key2 = SessionManager::generate_session_key();

        // Keys should be unique
        assert_ne!(key1.to_base64(), key2.to_base64());
    }

    #[test]
    fn test_format_for_export() {
        let key = SessionManager::generate_session_key();
        let exported = SessionManager::format_for_export(&key);

        // Should be valid base64
        assert!(!exported.is_empty());

        // Should be able to parse back
        let parsed = SessionManager::validate_session_key(&exported);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_validate_session_key_invalid() {
        let result = SessionManager::validate_session_key("not-valid-base64!");
        assert!(result.is_err());
    }

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
