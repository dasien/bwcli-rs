//! Export service
//!
//! Formatting is delegated entirely to `bitwarden-exporters`, which is the same
//! code path the other Bitwarden clients use. The CLI keeps ownership of format
//! naming, password policy, and file output.

use crate::services::import_export::errors::ExportError;
use bitwarden_core::Client;
use bitwarden_exporters::{ExporterClientExt, ExportFormat};
use bitwarden_vault::{Cipher, Folder};
use secrecy::{ExposeSecret, Secret};
use std::sync::Arc;

/// Vault data to export.
///
/// These are the *encrypted* SDK types: `export_vault` decrypts them itself
/// using the client's key store, so the caller must not pre-decrypt.
#[derive(Debug)]
pub struct ExportData {
    pub folders: Vec<Folder>,
    pub ciphers: Vec<Cipher>,
}

/// Export options
#[derive(Debug, Clone, Default)]
pub struct ExportOptions {
    pub password: Option<Secret<String>>,
    pub organization_id: Option<String>,
}

/// Export result
#[derive(Debug)]
pub struct ExportResult {
    pub item_count: usize,
    pub format: String,
    pub output_path: Option<String>,
    pub encrypted: bool,
}

/// Format names accepted on the command line, in the order `--help` lists them.
const SUPPORTED_FORMATS: [&str; 3] = ["csv", "encrypted_json", "json"];

/// Service for exporting vault data
pub struct ExportService {
    client: Arc<Client>,
}

impl ExportService {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }

    /// Export vault to the specified format.
    ///
    /// Writes to `output_path` when given, otherwise to stdout.
    pub async fn export(
        &self,
        format: &str,
        output_path: Option<&str>,
        data: ExportData,
        options: ExportOptions,
    ) -> Result<ExportResult, ExportError> {
        // The SDK's organization export is unimplemented upstream and panics
        // (`export_organization_vault` is a `todo!()`), so refuse rather than
        // abort the process.
        if options.organization_id.is_some() {
            return Err(ExportError::UnsupportedFormat(
                "organization export is not supported yet".to_string(),
            ));
        }

        let export_format = match format {
            "json" => ExportFormat::Json,
            "csv" => ExportFormat::Csv,
            "encrypted_json" => {
                // The TypeScript CLI supports an account-key-protected export
                // when no password is given. The SDK only implements the
                // password-protected variant, so require one explicitly rather
                // than silently producing a different kind of file.
                let password = options.password.as_ref().ok_or(ExportError::PasswordRequired)?;
                ExportFormat::EncryptedJson {
                    password: password.expose_secret().to_string(),
                }
            }
            other => return Err(ExportError::UnsupportedFormat(other.to_string())),
        };

        let encrypted = matches!(export_format, ExportFormat::EncryptedJson { .. });
        let item_count = data.ciphers.len();

        let contents = self
            .client
            .exporters()
            .export_vault(data.folders, data.ciphers, export_format)
            .await
            .map_err(|e| ExportError::DecryptionError(e.to_string()))?;

        if let Some(path) = output_path {
            std::fs::write(path, contents.as_bytes())
                .map_err(|e| ExportError::FileWriteError(format!("{}: {}", path, e)))?;
        } else {
            use std::io::Write;
            std::io::stdout()
                .write_all(contents.as_bytes())
                .map_err(ExportError::IoError)?;
        }

        Ok(ExportResult {
            item_count,
            format: format.to_string(),
            output_path: output_path.map(String::from),
            encrypted,
        })
    }

    /// List supported export formats
    pub fn supported_formats(&self) -> Vec<String> {
        SUPPORTED_FORMATS.iter().map(|s| s.to_string()).collect()
    }
}
