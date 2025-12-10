use super::errors::StorageError;
use anyhow::Result;
use directories::ProjectDirs;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Platform-aware storage path resolution
pub struct StoragePath;

impl StoragePath {
    /// Resolve storage directory path
    ///
    /// # Priority Order
    /// 1. `./bw-data` - Relative to executable (portable mode)
    /// 2. `BITWARDENCLI_APPDATA_DIR` - Custom path via environment variable
    /// 3. Platform default - OS-specific application data directory
    ///
    /// # Platform Defaults
    /// - **macOS**: `~/Library/Application Support/Bitwarden CLI`
    /// - **Windows**: `%APPDATA%/Bitwarden CLI`
    /// - **Linux**: `$XDG_CONFIG_HOME/Bitwarden CLI` or `~/.config/Bitwarden CLI`
    pub fn resolve(custom_path: Option<PathBuf>) -> Result<PathBuf> {
        // 1. Check for custom path argument
        if let Some(path) = custom_path {
            return Self::canonicalize_path(path);
        }

        // 2. Check for ./bw-data (portable mode)
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let portable_path = exe_dir.join("bw-data");
                if portable_path.exists() && portable_path.is_dir() {
                    return Ok(portable_path);
                }
            }
        }

        // 3. Check BITWARDENCLI_APPDATA_DIR environment variable
        if let Ok(env_path) = env::var("BITWARDENCLI_APPDATA_DIR") {
            let path = PathBuf::from(env_path);
            return Self::canonicalize_path(path);
        }

        // 4. Use platform default - matching TypeScript CLI path exactly
        // On macOS: ~/Library/Application Support/Bitwarden CLI
        // On Windows: %APPDATA%/Bitwarden CLI
        // On Linux: ~/.config/Bitwarden CLI
        #[cfg(target_os = "macos")]
        {
            let home = env::var("HOME").map_err(|_| {
                StorageError::PathResolutionError("Could not determine home directory".to_string())
            })?;
            return Ok(PathBuf::from(home).join("Library/Application Support/Bitwarden CLI"));
        }

        #[cfg(target_os = "windows")]
        {
            let appdata = env::var("APPDATA").map_err(|_| {
                StorageError::PathResolutionError(
                    "Could not determine APPDATA directory".to_string(),
                )
            })?;
            return Ok(PathBuf::from(appdata).join("Bitwarden CLI"));
        }

        #[cfg(target_os = "linux")]
        {
            // Use XDG_CONFIG_HOME if set, otherwise ~/.config
            let config_home = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
                let home = env::var("HOME").unwrap_or_else(|_| String::from("~"));
                format!("{}/.config", home)
            });
            return Ok(PathBuf::from(config_home).join("Bitwarden CLI"));
        }

        // Fallback for other platforms (shouldn't happen)
        #[allow(unreachable_code)]
        {
            let project_dirs =
                ProjectDirs::from("com", "Bitwarden", "Bitwarden CLI").ok_or_else(|| {
                    StorageError::PathResolutionError(
                        "Could not determine platform application directory".to_string(),
                    )
                })?;
            Ok(project_dirs.data_dir().to_path_buf())
        }
    }

    /// Ensure directory exists with correct permissions
    ///
    /// # Permissions
    /// - Directory: 0700 (owner read/write/execute only)
    /// - Data file: 0600 (owner read/write only)
    pub fn ensure_directory_exists(path: &PathBuf) -> Result<()> {
        if !path.exists() {
            fs::create_dir_all(path)
                .map_err(|e| StorageError::CreateDirectoryError(e, path.clone()))?;

            // Set permissions (Unix-only, ignored on Windows)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::Permissions::from_mode(0o700);
                fs::set_permissions(path, perms)
                    .map_err(|e| StorageError::PermissionError(e, path.clone()))?;
            }
        }

        // Verify directory is writable
        if !Self::is_writable(path) {
            return Err(StorageError::NotWritableError(path.clone()).into());
        }

        Ok(())
    }

    /// Check if path is writable
    fn is_writable(path: &std::path::Path) -> bool {
        // Attempt to create a temp file
        let test_file = path.join(".write-test");
        match fs::write(&test_file, b"test") {
            Ok(_) => {
                let _ = fs::remove_file(&test_file);
                true
            }
            Err(_) => false,
        }
    }

    /// Canonicalize path (resolve to absolute path)
    fn canonicalize_path(path: PathBuf) -> Result<PathBuf> {
        if path.is_absolute() {
            Ok(path)
        } else {
            env::current_dir().map(|cwd| cwd.join(path)).map_err(|e| {
                StorageError::PathResolutionError(format!("Failed to get current directory: {}", e))
                    .into()
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_env_var_override() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // SAFETY: Test runs in isolation, env var modification is acceptable
        unsafe {
            env::set_var("BITWARDENCLI_APPDATA_DIR", temp_path.to_str().unwrap());
        }

        let resolved = StoragePath::resolve(None).unwrap();
        assert_eq!(resolved, temp_path);

        // SAFETY: Test cleanup
        unsafe {
            env::remove_var("BITWARDENCLI_APPDATA_DIR");
        }
    }

    #[test]
    fn test_custom_path() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let resolved = StoragePath::resolve(Some(temp_path.clone())).unwrap();
        assert_eq!(resolved, temp_path);
    }

    #[test]
    fn test_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("test-storage");

        assert!(!storage_path.exists());

        StoragePath::ensure_directory_exists(&storage_path).unwrap();

        assert!(storage_path.exists());
        assert!(storage_path.is_dir());
    }

    #[test]
    fn test_is_writable() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        assert!(StoragePath::is_writable(&temp_path));
    }
}
