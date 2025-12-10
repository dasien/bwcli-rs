use super::{
    atomic::AtomicWriter,
    errors::StorageError,
    keys::{SUPPORTED_STATE_VERSION, StorageKey},
    path::StoragePath,
    traits::Storage,
};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::debug;

/// JSON-based file storage with support for secure (encrypted) values
///
/// Storage format is compatible with TypeScript CLI's namespaced format:
/// - Single JSON file (data.json)
/// - Namespaced key patterns (e.g., `global_account_accounts`, `user_{id}_token_accessToken`)
/// - State version tracked at `stateVersion` key (currently 73)
/// - Keys with __PROTECTED__ prefix are encrypted
/// - Values are JSON types (string, number, object, array, etc.)
/// - Unknown keys are preserved when writing to maintain cross-CLI compatibility
pub struct JsonFileStorage {
    /// In-memory cache of storage contents
    /// Uses Mutex for interior mutability (required for get operations)
    data: Arc<Mutex<HashMap<String, Value>>>,

    /// Atomic writer for safe file operations
    writer: AtomicWriter,
}

impl JsonFileStorage {
    /// Create new storage instance
    ///
    /// # Arguments
    /// * `custom_path` - Optional custom storage directory path
    ///   If None, uses platform-specific default
    ///
    /// # Path Resolution Priority
    /// 1. `./bw-data` (relative to executable)
    /// 2. `BITWARDENCLI_APPDATA_DIR` environment variable
    /// 3. Platform default (see `StoragePath::resolve`)
    pub fn new(custom_path: Option<PathBuf>) -> Result<Self> {
        let storage_path = StoragePath::resolve(custom_path)?;
        let file_path = storage_path.join("data.json");

        // Ensure storage directory exists with correct permissions
        StoragePath::ensure_directory_exists(&storage_path)?;

        let writer = AtomicWriter::new(file_path.clone());

        let data = if file_path.exists() {
            Self::load_from_file(&file_path)?
        } else {
            Arc::new(Mutex::new(HashMap::new()))
        };

        Ok(Self { data, writer })
    }

    /// Load storage from file
    fn load_from_file(path: &PathBuf) -> Result<Arc<Mutex<HashMap<String, Value>>>> {
        let contents =
            fs::read_to_string(path).map_err(|e| StorageError::ReadError(e, path.clone()))?;

        if contents.trim().is_empty() {
            return Ok(Arc::new(Mutex::new(HashMap::new())));
        }

        let data: HashMap<String, Value> = serde_json::from_str(&contents)
            .map_err(|e| StorageError::ParseError(e, path.clone()))?;

        // Validate state version if present
        if let Some(version_value) = data.get("stateVersion") {
            if let Some(version) = version_value.as_u64() {
                if version < SUPPORTED_STATE_VERSION {
                    return Err(StorageError::UnsupportedStateVersion {
                        found: version,
                        required: SUPPORTED_STATE_VERSION,
                    }
                    .into());
                }
                debug!("Loaded storage with state version {}", version);
            }
        }

        Ok(Arc::new(Mutex::new(data)))
    }

    /// Ensure state version is set in storage
    ///
    /// Called during login to initialize the state version for new storage
    /// or verify compatibility for existing storage.
    pub async fn ensure_state_version(&mut self) -> Result<()> {
        let has_version = {
            let data = self.acquire_lock()?;
            data.contains_key("stateVersion")
        };

        if !has_version {
            debug!("Initializing state version to {}", SUPPORTED_STATE_VERSION);
            self.set(
                &StorageKey::StateVersion.format(None),
                &SUPPORTED_STATE_VERSION,
            )
            .await?;
        }

        Ok(())
    }

    /// Get the current state version
    pub fn get_state_version(&self) -> Result<Option<u64>> {
        let data = self.acquire_lock()?;
        Ok(data.get("stateVersion").and_then(|v| v.as_u64()))
    }

    /// Get nested value by dot-separated path
    ///
    /// Example: "environmentUrls.api" accesses obj["environmentUrls"]["api"]
    fn get_nested<'a>(data: &'a Value, key: &str) -> Option<&'a Value> {
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = data;

        for part in parts {
            current = current.get(part)?;
        }

        Some(current)
    }

    /// Set nested value by dot-separated path
    ///
    /// Creates intermediate objects as needed
    fn set_nested(data: &mut Value, key: &str, value: Value) {
        let parts: Vec<&str> = key.split('.').collect();

        if parts.len() == 1 {
            // Simple case: direct key
            data[parts[0]] = value;
            return;
        }

        let mut current = data;

        // Navigate to parent, creating intermediate objects as needed
        for part in &parts[..parts.len() - 1] {
            if !current[part].is_object() {
                current[part] = Value::Object(serde_json::Map::new());
            }
            current = &mut current[part];
        }

        // Set the final value
        let last_key = parts[parts.len() - 1];
        current[last_key] = value;
    }

    /// Remove nested value by dot-separated path
    fn remove_nested(data: &mut Value, key: &str) -> bool {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.is_empty() {
            return false;
        }

        let mut current = data;

        // Navigate to parent
        for part in &parts[..parts.len() - 1] {
            if let Some(obj) = current.get_mut(part) {
                current = obj;
            } else {
                return false;
            }
        }

        // Remove final key
        let last_key = parts[parts.len() - 1];
        if let Some(obj) = current.as_object_mut() {
            obj.remove(last_key).is_some()
        } else {
            false
        }
    }

    /// Acquire lock on data, converting poison errors to StorageError
    fn acquire_lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, HashMap<String, Value>>, StorageError> {
        self.data
            .lock()
            .map_err(|e| StorageError::LockError(format!("Mutex poisoned: {}", e)))
    }
}

#[async_trait]
impl Storage for JsonFileStorage {
    fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let data = self.acquire_lock()?;
        let root = Value::Object(data.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

        let value = Self::get_nested(&root, key);

        match value {
            None => Ok(None),
            Some(v) => {
                let deserialized: T = serde_json::from_value(v.clone())
                    .map_err(|e| StorageError::DeserializationError(e, key.to_string()))?;
                Ok(Some(deserialized))
            }
        }
    }

    async fn set<T>(&mut self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        let json_value = serde_json::to_value(value)
            .map_err(|e| StorageError::SerializationError(e, key.to_string()))?;

        {
            let mut data = self.acquire_lock()?;
            let mut root =
                Value::Object(data.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

            Self::set_nested(&mut root, key, json_value);

            // Update in-memory cache
            if let Value::Object(map) = root {
                *data = map.into_iter().collect();
            }
        } // MutexGuard dropped here

        self.flush().await
    }

    async fn remove(&mut self, key: &str) -> Result<bool> {
        let removed = {
            let mut data = self.acquire_lock()?;
            let mut root =
                Value::Object(data.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

            let removed = Self::remove_nested(&mut root, key);

            if removed {
                if let Value::Object(map) = root {
                    *data = map.into_iter().collect();
                }
            }
            removed
        }; // MutexGuard dropped here

        if removed {
            self.flush().await?;
        }

        Ok(removed)
    }

    fn has(&self, key: &str) -> Result<bool> {
        let data = self.acquire_lock()?;
        let root = Value::Object(data.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

        Ok(Self::get_nested(&root, key).is_some())
    }

    async fn flush(&mut self) -> Result<()> {
        let data = self.acquire_lock()?;
        let json = serde_json::to_string_pretty(&*data)
            .map_err(|e| StorageError::SerializationError(e, "storage".to_string()))?;

        self.writer.write_atomic(&json)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = JsonFileStorage::new(Some(temp_dir.path().to_path_buf()));
        assert!(storage.is_ok());
    }

    #[tokio::test]
    async fn test_get_set_string() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = JsonFileStorage::new(Some(temp_dir.path().to_path_buf())).unwrap();

        storage
            .set("test_key", &"test_value".to_string())
            .await
            .unwrap();
        let result: Option<String> = storage.get("test_key").unwrap();
        assert_eq!(result, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_nested_keys() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = JsonFileStorage::new(Some(temp_dir.path().to_path_buf())).unwrap();

        storage
            .set("parent.child", &"nested_value".to_string())
            .await
            .unwrap();
        let result: Option<String> = storage.get("parent.child").unwrap();
        assert_eq!(result, Some("nested_value".to_string()));
    }

    #[tokio::test]
    async fn test_remove() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = JsonFileStorage::new(Some(temp_dir.path().to_path_buf())).unwrap();

        storage
            .set("test_key", &"test_value".to_string())
            .await
            .unwrap();
        assert!(storage.has("test_key").unwrap());

        let removed = storage.remove("test_key").await.unwrap();
        assert!(removed);
        assert!(!storage.has("test_key").unwrap());
    }

    #[tokio::test]
    async fn test_has() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = JsonFileStorage::new(Some(temp_dir.path().to_path_buf())).unwrap();

        assert!(!storage.has("nonexistent").unwrap());

        storage.set("existing", &"value".to_string()).await.unwrap();
        assert!(storage.has("existing").unwrap());
    }

    #[tokio::test]
    async fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        {
            let mut storage = JsonFileStorage::new(Some(path.clone())).unwrap();
            storage
                .set("persist_key", &"persist_value".to_string())
                .await
                .unwrap();
        }

        {
            let storage = JsonFileStorage::new(Some(path)).unwrap();
            let result: Option<String> = storage.get("persist_key").unwrap();
            assert_eq!(result, Some("persist_value".to_string()));
        }
    }
}
