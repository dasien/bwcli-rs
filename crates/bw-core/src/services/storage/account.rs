//! Account management for TypeScript CLI compatibility
//!
//! Manages the account registry and active account resolution.
//! The TypeScript CLI stores accounts in `global_account_accounts` and
//! tracks the active account in `global_account_activeAccountId`.

use super::{JsonFileStorage, Storage, keys::StorageKey};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Account information stored in the global accounts registry
///
/// This matches the TypeScript CLI's account storage format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    /// User's email address
    pub email: String,

    /// Whether the email has been verified
    #[serde(default)]
    pub email_verified: bool,

    /// User's display name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Manages account registry and active account resolution
///
/// Provides a clean API for:
/// - Getting/setting the active user ID
/// - Registering accounts in the global registry
/// - Clearing active account on logout
pub struct AccountManager {
    storage: Arc<Mutex<JsonFileStorage>>,
}

impl AccountManager {
    /// Create a new AccountManager
    pub fn new(storage: Arc<Mutex<JsonFileStorage>>) -> Self {
        Self { storage }
    }

    /// Get the active user ID, if any
    ///
    /// Returns `Ok(None)` if no user is logged in.
    /// The active account ID is stored at `global_account_activeAccountId`.
    pub async fn get_active_user_id(&self) -> Result<Option<String>> {
        let storage = self.storage.lock().await;
        let key = StorageKey::GlobalActiveAccountId.format(None);

        // The value might be null (JSON null) or a string
        let value: Option<serde_json::Value> = storage.get(&key)?;

        match value {
            Some(serde_json::Value::String(id)) if !id.is_empty() => Ok(Some(id)),
            _ => Ok(None),
        }
    }

    /// Set the active user ID
    ///
    /// This marks the specified user as the currently active account.
    pub async fn set_active_user_id(&self, user_id: &str) -> Result<()> {
        let mut storage = self.storage.lock().await;
        let key = StorageKey::GlobalActiveAccountId.format(None);
        storage.set(&key, &user_id.to_string()).await?;
        storage.flush().await?;
        Ok(())
    }

    /// Clear the active account (on logout)
    ///
    /// Sets the active account ID to null but preserves the account in the registry.
    /// This allows the TypeScript CLI to re-login without re-entering the email.
    pub async fn clear_active_account(&self) -> Result<()> {
        let mut storage = self.storage.lock().await;
        let key = StorageKey::GlobalActiveAccountId.format(None);
        storage.set(&key, &serde_json::Value::Null).await?;
        storage.flush().await?;
        Ok(())
    }

    /// Register an account in the global accounts registry
    ///
    /// Adds or updates the account entry. The registry is stored at
    /// `global_account_accounts` as a map of user ID to AccountInfo.
    pub async fn register_account(&self, user_id: &str, email: &str) -> Result<()> {
        let mut storage = self.storage.lock().await;
        let key = StorageKey::GlobalAccounts.format(None);

        // Get existing accounts or create new map
        let mut accounts: HashMap<String, AccountInfo> = storage.get(&key)?.unwrap_or_default();

        // Add/update this account
        accounts.insert(
            user_id.to_string(),
            AccountInfo {
                email: email.to_string(),
                email_verified: true,
                name: None,
            },
        );

        storage.set(&key, &accounts).await?;
        storage.flush().await?;
        Ok(())
    }

    /// Get account info for a specific user
    pub async fn get_account(&self, user_id: &str) -> Result<Option<AccountInfo>> {
        let storage = self.storage.lock().await;
        let key = StorageKey::GlobalAccounts.format(None);

        let accounts: Option<HashMap<String, AccountInfo>> = storage.get(&key)?;

        Ok(accounts.and_then(|a| a.get(user_id).cloned()))
    }

    /// Get all registered accounts
    pub async fn get_all_accounts(&self) -> Result<HashMap<String, AccountInfo>> {
        let storage = self.storage.lock().await;
        let key = StorageKey::GlobalAccounts.format(None);

        Ok(storage.get(&key)?.unwrap_or_default())
    }

    /// Remove an account from the registry
    ///
    /// This fully removes the account. Use `clear_active_account` for logout.
    pub async fn remove_account(&self, user_id: &str) -> Result<bool> {
        let mut storage = self.storage.lock().await;
        let key = StorageKey::GlobalAccounts.format(None);

        let mut accounts: HashMap<String, AccountInfo> = storage.get(&key)?.unwrap_or_default();

        let removed = accounts.remove(user_id).is_some();

        if removed {
            storage.set(&key, &accounts).await?;
            storage.flush().await?;
        }

        Ok(removed)
    }

    /// Check if we have a logged-in session (active account with tokens)
    ///
    /// This checks:
    /// 1. There is an active account ID
    /// 2. The active account has an access token (not null)
    pub async fn is_logged_in(&self) -> Result<bool> {
        let user_id = match self.get_active_user_id().await? {
            Some(id) => id,
            None => return Ok(false),
        };

        let storage = self.storage.lock().await;
        let token_key = StorageKey::UserAccessToken.format(Some(&user_id));

        // Check for access token (may be null if logged out)
        let token: Option<serde_json::Value> = storage.get(&token_key)?;

        // Token is present and not null
        Ok(matches!(token, Some(serde_json::Value::String(s)) if !s.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_account_manager() -> (AccountManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Mutex::new(
            JsonFileStorage::new(Some(temp_dir.path().to_path_buf())).unwrap(),
        ));
        (AccountManager::new(storage), temp_dir)
    }

    #[tokio::test]
    async fn test_no_active_user_initially() {
        let (manager, _temp) = create_test_account_manager().await;
        let user_id = manager.get_active_user_id().await.unwrap();
        assert!(user_id.is_none());
    }

    #[tokio::test]
    async fn test_set_and_get_active_user() {
        let (manager, _temp) = create_test_account_manager().await;

        manager.set_active_user_id("test-user-123").await.unwrap();

        let user_id = manager.get_active_user_id().await.unwrap();
        assert_eq!(user_id, Some("test-user-123".to_string()));
    }

    #[tokio::test]
    async fn test_clear_active_account() {
        let (manager, _temp) = create_test_account_manager().await;

        manager.set_active_user_id("test-user-123").await.unwrap();
        manager.clear_active_account().await.unwrap();

        let user_id = manager.get_active_user_id().await.unwrap();
        assert!(user_id.is_none());
    }

    #[tokio::test]
    async fn test_register_and_get_account() {
        let (manager, _temp) = create_test_account_manager().await;

        manager
            .register_account("user-123", "test@example.com")
            .await
            .unwrap();

        let account = manager.get_account("user-123").await.unwrap();
        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.email, "test@example.com");
        assert!(account.email_verified);
    }

    #[tokio::test]
    async fn test_get_all_accounts() {
        let (manager, _temp) = create_test_account_manager().await;

        manager
            .register_account("user-1", "user1@example.com")
            .await
            .unwrap();
        manager
            .register_account("user-2", "user2@example.com")
            .await
            .unwrap();

        let accounts = manager.get_all_accounts().await.unwrap();
        assert_eq!(accounts.len(), 2);
        assert!(accounts.contains_key("user-1"));
        assert!(accounts.contains_key("user-2"));
    }

    #[tokio::test]
    async fn test_remove_account() {
        let (manager, _temp) = create_test_account_manager().await;

        manager
            .register_account("user-1", "user1@example.com")
            .await
            .unwrap();

        let removed = manager.remove_account("user-1").await.unwrap();
        assert!(removed);

        let account = manager.get_account("user-1").await.unwrap();
        assert!(account.is_none());
    }

    #[tokio::test]
    async fn test_is_not_logged_in_without_active_account() {
        let (manager, _temp) = create_test_account_manager().await;

        let logged_in = manager.is_logged_in().await.unwrap();
        assert!(!logged_in);
    }
}
