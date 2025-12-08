//! Export service and formatters

pub mod formatters;

use crate::models::vault::{CipherView, FolderView};
use crate::services::import_export::errors::ExportError;
use async_trait::async_trait;
use secrecy::Secret;
use std::collections::HashMap;
use std::sync::Arc;

/// Export data structure (decrypted vault items)
#[derive(Debug, Clone)]
pub struct ExportData {
    pub folders: Vec<FolderView>,
    pub ciphers: Vec<CipherView>,
}

/// Export options
#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub password: Option<Secret<String>>,
    pub organization_id: Option<String>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            password: None,
            organization_id: None,
        }
    }
}

/// Export result
#[derive(Debug)]
pub struct ExportResult {
    pub item_count: usize,
    pub format: String,
    pub output_path: Option<String>,
    pub encrypted: bool,
}

/// Trait for export format implementations
#[async_trait]
pub trait ExportFormatter: Send + Sync {
    /// Format name (e.g., "csv", "json", "encrypted_json")
    fn format_name(&self) -> &str;

    /// File extension (e.g., "csv", "json")
    fn file_extension(&self) -> &str;

    /// Format vault data for export
    async fn format(
        &self,
        data: &ExportData,
        options: &ExportOptions,
    ) -> Result<Vec<u8>, ExportError>;

    /// Whether this format requires encryption password
    fn requires_password(&self) -> bool;

    /// Whether this is an encrypted format
    fn is_encrypted(&self) -> bool;
}

/// Service for exporting vault data
pub struct ExportService {
    formatters: HashMap<String, Arc<dyn ExportFormatter>>,
}

impl ExportService {
    /// Create a new export service with all formatters
    pub fn new() -> Self {
        let mut formatters: HashMap<String, Arc<dyn ExportFormatter>> = HashMap::new();

        // Register formatters
        formatters.insert(
            "csv".to_string(),
            Arc::new(formatters::csv::CsvFormatter::new()),
        );
        formatters.insert(
            "json".to_string(),
            Arc::new(formatters::json::JsonFormatter::new()),
        );
        formatters.insert(
            "encrypted_json".to_string(),
            Arc::new(formatters::encrypted_json::EncryptedJsonFormatter::new()),
        );

        Self { formatters }
    }

    /// Export vault to specified format
    pub async fn export(
        &self,
        format: &str,
        output_path: Option<&str>,
        data: ExportData,
        options: ExportOptions,
    ) -> Result<ExportResult, ExportError> {
        // Get formatter
        let formatter = self
            .formatters
            .get(format)
            .ok_or_else(|| ExportError::UnsupportedFormat(format.to_string()))?;

        // Validate password requirement
        if formatter.requires_password() && options.password.is_none() {
            return Err(ExportError::PasswordRequired);
        }

        // Format data
        let formatted = formatter.format(&data, &options).await?;

        // Write to file or return for stdout
        if let Some(path) = output_path {
            std::fs::write(path, &formatted)
                .map_err(|e| ExportError::FileWriteError(format!("{}: {}", path, e)))?;
        } else {
            // Write to stdout
            use std::io::Write;
            std::io::stdout()
                .write_all(&formatted)
                .map_err(ExportError::IoError)?;
        }

        Ok(ExportResult {
            item_count: data.ciphers.len(),
            format: format.to_string(),
            output_path: output_path.map(String::from),
            encrypted: formatter.is_encrypted(),
        })
    }

    /// List supported export formats
    pub fn supported_formats(&self) -> Vec<String> {
        let mut formats: Vec<String> = self.formatters.keys().cloned().collect();
        formats.sort();
        formats
    }
}

impl Default for ExportService {
    fn default() -> Self {
        Self::new()
    }
}
