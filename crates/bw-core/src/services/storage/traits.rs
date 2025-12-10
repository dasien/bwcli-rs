use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Abstract storage interface for configuration and state persistence
///
/// Implementations must provide:
/// - Type-safe get/set operations with JSON serialization
/// - Atomic operations to prevent corruption
/// - Thread-safe access for potential concurrent operations
///
/// Note: For encrypted storage of sensitive keys (like the user key),
/// use the `protected_storage` module functions directly with the
/// `__PROTECTED__` key prefix convention.
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

    /// Persist all pending changes to disk
    ///
    /// Called automatically by set/remove operations
    /// Can be called explicitly to ensure durability
    async fn flush(&mut self) -> Result<()>;
}
