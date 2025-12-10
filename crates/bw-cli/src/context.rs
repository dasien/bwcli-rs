//! Application context for CLI commands
//!
//! Provides a single initialization point for all services used by CLI commands.
//! This avoids repeated ServiceContainer creation in each command handler.

use anyhow::Result;
use bw_core::services::api::BitwardenApiClient;
use bw_core::services::storage::JsonFileStorage;
use bw_core::services::{Client, ServiceContainer};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Application context containing initialized services
///
/// Created once at startup and passed to all command handlers.
/// This enables consistent service initialization and easier testing.
pub struct AppContext {
    container: Arc<ServiceContainer>,
}

impl AppContext {
    /// Create a new application context with default configuration
    pub fn new() -> Result<Self> {
        let container = ServiceContainer::new(None, None, None, None)?;
        Ok(Self {
            container: Arc::new(container),
        })
    }

    /// Create application context with custom configuration
    #[allow(dead_code)]
    pub fn with_config(
        api_url: Option<String>,
        identity_url: Option<String>,
        storage_path: Option<std::path::PathBuf>,
        timeout_seconds: Option<u64>,
    ) -> Result<Self> {
        let container = ServiceContainer::new(api_url, identity_url, storage_path, timeout_seconds)?;
        Ok(Self {
            container: Arc::new(container),
        })
    }

    /// Get reference to the service container
    pub fn container(&self) -> &Arc<ServiceContainer> {
        &self.container
    }

    /// Get storage service
    pub fn storage(&self) -> Arc<Mutex<JsonFileStorage>> {
        self.container.storage()
    }

    /// Get API client
    pub fn api_client(&self) -> Arc<BitwardenApiClient> {
        self.container.api_client()
    }

    /// Get SDK client
    pub fn sdk(&self) -> &Client {
        self.container.sdk()
    }
}
