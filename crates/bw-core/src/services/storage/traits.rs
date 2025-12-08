use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Abstract storage interface for configuration and state persistence
///
/// Implementations must provide:
/// - Type-safe get/set operations with JSON serialization
/// - Secure storage for sensitive values (encrypted at rest)
/// - Atomic operations to prevent corruption
/// - Thread-safe access for potential concurrent operations
#[async_trait]
pub trait Storage: Send + Sync {
    /// Retrieve a value by key
    ///
    /// Returns None if key doesn't exist, error if deserialization fails
    fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>;

    /// Store a value by key
    ///
    /// Overwrites existing value if present
    async fn set<T>(&mut self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize + Send + Sync;

    /// Remove a value by key
    ///
    /// Returns true if value existed and was removed, false if key not found
    async fn remove(&mut self, key: &str) -> Result<bool>;

    /// Check if a key exists
    fn has(&self, key: &str) -> Result<bool>;

    /// Retrieve a secure (encrypted) value
    ///
    /// Requires BW_SESSION environment variable to decrypt
    /// Returns None if key doesn't exist
    /// Returns error if BW_SESSION missing or decryption fails
    async fn get_secure(&self, key: &str) -> Result<Option<String>>;

    /// Store a secure (encrypted) value
    ///
    /// Encrypts value using BW_SESSION before storing
    /// Stores with __PROTECTED__ key prefix
    /// Requires BW_SESSION environment variable
    async fn set_secure(&mut self, key: &str, value: &str) -> Result<()>;

    /// Remove a secure value by key
    ///
    /// Automatically adds __PROTECTED__ prefix if not present
    async fn remove_secure(&mut self, key: &str) -> Result<bool>;

    /// Persist all pending changes to disk
    ///
    /// Called automatically by set/remove operations
    /// Can be called explicitly to ensure durability
    async fn flush(&mut self) -> Result<()>;
}
