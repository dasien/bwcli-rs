//! JSON export formatter

use crate::models::vault::{CipherView, FolderView};
use crate::services::import_export::errors::ExportError;
use crate::services::import_export::export::{ExportData, ExportFormatter, ExportOptions};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// JSON export structure (for serialization only - uses references to avoid Clone requirement)
#[derive(Debug, Serialize)]
pub struct JsonExport<'a> {
    pub encrypted: bool,
    pub folders: &'a [FolderView],
    pub items: &'a [CipherView],
}

/// Owned JSON export structure (for deserialization during import)
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonExportOwned {
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
            folders: &data.folders,
            items: &data.ciphers,
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
