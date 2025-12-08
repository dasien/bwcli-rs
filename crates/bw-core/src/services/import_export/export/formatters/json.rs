//! JSON export formatter

use crate::models::vault::{CipherView, FolderView};
use crate::services::import_export::errors::ExportError;
use crate::services::import_export::export::{ExportData, ExportFormatter, ExportOptions};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// JSON export structure
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonExport {
    pub encrypted: bool,
    pub folders: Vec<FolderView>,
    pub items: Vec<CipherView>,
}

/// JSON export formatter
pub struct JsonFormatter;

impl JsonFormatter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExportFormatter for JsonFormatter {
    fn format_name(&self) -> &str {
        "json"
    }

    fn file_extension(&self) -> &str {
        "json"
    }

    async fn format(
        &self,
        data: &ExportData,
        _options: &ExportOptions,
    ) -> Result<Vec<u8>, ExportError> {
        let export = JsonExport {
            encrypted: false,
            folders: data.folders.clone(),
            items: data.ciphers.clone(),
        };

        // Pretty-print JSON with 2-space indentation
        let json = serde_json::to_vec_pretty(&export)?;
        Ok(json)
    }

    fn requires_password(&self) -> bool {
        false
    }

    fn is_encrypted(&self) -> bool {
        false
    }
}
