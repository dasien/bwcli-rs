use super::errors::StorageError;
use anyhow::Result;
use fs2::FileExt;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

/// Atomic file writer with file locking
///
/// Uses temp file + atomic rename pattern to prevent corruption
/// Implements file locking to handle concurrent access
pub struct AtomicWriter {
    target_path: PathBuf,
}

impl AtomicWriter {
    pub fn new(target_path: PathBuf) -> Self {
        Self { target_path }
    }

    /// Write content atomically to target file
    ///
    /// # Process
    /// 1. Acquire file lock on target path
    /// 2. Write content to temporary file in same directory
    /// 3. Flush and sync to ensure data is on disk
    /// 4. Atomically rename temp file to target path
    /// 5. Release file lock
    ///
    /// # Atomicity Guarantee
    /// The rename operation is atomic on all platforms (POSIX and Windows)
    /// If process crashes during write, either old or new data is present
    pub fn write_atomic(&self, content: &str) -> Result<()> {
        // Acquire lock file
        let lock = self.acquire_lock()?;

        // Create temp file in same directory (ensures same filesystem)
        let temp_path = self.temp_file_path();

        let mut file =
            File::create(&temp_path).map_err(|e| StorageError::WriteError(e, temp_path.clone()))?;

        file.write_all(content.as_bytes())
            .map_err(|e| StorageError::WriteError(e, temp_path.clone()))?;

        // Flush and sync to ensure data reaches disk
        file.flush()
            .map_err(|e| StorageError::WriteError(e, temp_path.clone()))?;
        file.sync_all()
            .map_err(|e| StorageError::WriteError(e, temp_path.clone()))?;

        drop(file);

        // Atomic rename
        fs::rename(&temp_path, &self.target_path)
            .map_err(|e| StorageError::WriteError(e, self.target_path.clone()))?;

        // Set file permissions (Unix-only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&self.target_path, perms)
                .map_err(|e| StorageError::PermissionError(e, self.target_path.clone()))?;
        }

        // Release lock
        drop(lock);

        Ok(())
    }

    /// Generate temp file path
    fn temp_file_path(&self) -> PathBuf {
        let mut temp = self.target_path.clone();
        let mut filename = temp
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        filename.push_str(".tmp");
        temp.set_file_name(filename);
        temp
    }

    /// Acquire file lock for concurrent access protection
    ///
    /// Uses fs2 crate for cross-platform file locking
    /// Returns lock guard that releases lock on drop
    fn acquire_lock(&self) -> Result<FileLock> {
        FileLock::acquire(&self.target_path)
    }
}

/// File lock guard
///
/// Automatically releases lock when dropped
struct FileLock {
    _file: File,
    lock_path: PathBuf,
}

impl FileLock {
    fn acquire(target_path: &std::path::Path) -> Result<Self> {
        let lock_path = target_path.with_extension("lock");

        let file =
            File::create(&lock_path).map_err(|e| StorageError::WriteError(e, lock_path.clone()))?;

        file.lock_exclusive()
            .map_err(|e| StorageError::WriteError(e, lock_path.clone()))?;

        Ok(Self {
            _file: file,
            lock_path,
        })
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // Lock is released automatically when file is closed/dropped
        // Clean up lock file if it exists
        let _ = fs::remove_file(&self.lock_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("test.json");

        let writer = AtomicWriter::new(target_path.clone());
        writer.write_atomic("test content").unwrap();

        let content = fs::read_to_string(&target_path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_temp_file_path() {
        let target_path = PathBuf::from("/tmp/data.json");
        let writer = AtomicWriter::new(target_path);

        let temp_path = writer.temp_file_path();
        assert_eq!(temp_path, PathBuf::from("/tmp/data.json.tmp"));
    }

    #[test]
    fn test_overwrite_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("test.json");

        let writer = AtomicWriter::new(target_path.clone());

        writer.write_atomic("first content").unwrap();
        let content1 = fs::read_to_string(&target_path).unwrap();
        assert_eq!(content1, "first content");

        writer.write_atomic("second content").unwrap();
        let content2 = fs::read_to_string(&target_path).unwrap();
        assert_eq!(content2, "second content");
    }
}
