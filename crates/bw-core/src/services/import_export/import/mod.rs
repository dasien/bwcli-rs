//! Import service and parsers

pub mod parsers;
pub mod validator;

use crate::services::import_export::errors::ImportError;
use async_trait::async_trait;
use secrecy::Secret;
use std::collections::HashMap;
use std::sync::Arc;

/// Import item type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportItemType {
    Login,
    SecureNote,
    Card,
    Identity,
}

/// Import folder
#[derive(Debug, Clone)]
pub struct ImportFolder {
    pub name: String,
}

/// Import login data
#[derive(Debug, Clone)]
pub struct ImportLogin {
    pub username: Option<String>,
    pub password: Option<String>,
    pub uris: Vec<String>,
    pub totp: Option<String>,
}

/// Import card data
#[derive(Debug, Clone)]
pub struct ImportCard {
    pub cardholder_name: Option<String>,
    pub number: Option<String>,
    pub brand: Option<String>,
    pub exp_month: Option<String>,
    pub exp_year: Option<String>,
    pub code: Option<String>,
}

/// Import identity data
#[derive(Debug, Clone)]
pub struct ImportIdentity {
    pub title: Option<String>,
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    pub last_name: Option<String>,
    pub address1: Option<String>,
    pub address2: Option<String>,
    pub address3: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub ssn: Option<String>,
    pub username: Option<String>,
    pub passport_number: Option<String>,
    pub license_number: Option<String>,
}

/// Import field
#[derive(Debug, Clone)]
pub struct ImportField {
    pub name: String,
    pub value: Option<String>,
    pub field_type: u8,
}

/// Import item
#[derive(Debug, Clone)]
pub struct ImportItem {
    pub item_type: ImportItemType,
    pub folder_name: Option<String>,
    pub favorite: bool,
    pub name: String,
    pub notes: Option<String>,
    pub fields: Vec<ImportField>,
    pub login: Option<ImportLogin>,
    pub card: Option<ImportCard>,
    pub identity: Option<ImportIdentity>,
}

/// Import data structure (intermediate format)
#[derive(Debug, Clone)]
pub struct ImportData {
    pub folders: Vec<ImportFolder>,
    pub items: Vec<ImportItem>,
}

/// Import options
#[derive(Debug, Clone)]
pub struct ImportOptions {
    pub password: Option<Secret<String>>,
    pub organization_id: Option<String>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            password: None,
            organization_id: None,
        }
    }
}

/// Import result
#[derive(Debug)]
pub struct ImportResult {
    pub items_created: usize,
    pub folders_created: usize,
    pub format: String,
}

/// Format information
#[derive(Debug, Clone)]
pub struct FormatInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
}

/// Trait for import format parsers
#[async_trait]
pub trait ImportParser: Send + Sync {
    /// Format name (e.g., "bitwardenjson", "lastpass")
    fn format_name(&self) -> &str;

    /// Parse import file
    async fn parse(&self, data: &[u8], options: &ImportOptions) -> Result<ImportData, ImportError>;

    /// Check if this parser can handle the data (for auto-detection)
    fn can_parse(&self, data: &[u8]) -> bool;

    /// Whether this format requires decryption password
    fn requires_password(&self) -> bool;
}

/// Service for importing data into vault
pub struct ImportService {
    parsers: HashMap<String, Arc<dyn ImportParser>>,
}

impl ImportService {
    /// Create a new import service with all parsers
    pub fn new() -> Self {
        let mut parsers: HashMap<String, Arc<dyn ImportParser>> = HashMap::new();

        // Register parsers
        parsers.insert(
            "bitwardencsv".to_string(),
            Arc::new(parsers::bitwarden_csv::BitwardenCsvParser::new()),
        );
        parsers.insert(
            "bitwardenjson".to_string(),
            Arc::new(parsers::bitwarden_json::BitwardenJsonParser::new()),
        );
        parsers.insert(
            "lastpass".to_string(),
            Arc::new(parsers::lastpass::LastPassParser::new()),
        );
        parsers.insert(
            "1password".to_string(),
            Arc::new(parsers::onepassword::OnePasswordParser::new()),
        );
        parsers.insert(
            "chrome".to_string(),
            Arc::new(parsers::chrome::ChromeParser::new()),
        );

        Self { parsers }
    }

    /// Import data from file
    pub async fn import(
        &self,
        format: &str,
        file_path: &str,
        options: ImportOptions,
    ) -> Result<ImportResult, ImportError> {
        // Check file size
        const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB
        let metadata = std::fs::metadata(file_path)?;
        if metadata.len() > MAX_FILE_SIZE {
            return Err(ImportError::FileTooLarge {
                size: metadata.len(),
                max: MAX_FILE_SIZE,
            });
        }

        // Read file
        let data = std::fs::read(file_path)
            .map_err(|e| ImportError::FileReadError(format!("{}: {}", file_path, e)))?;

        // Get parser
        let parser = self
            .parsers
            .get(format)
            .ok_or_else(|| ImportError::UnsupportedFormat(format.to_string()))?;

        // Parse data
        let import_data = parser.parse(&data, &options).await?;

        // Validate
        validator::validate(&import_data)?;

        // TODO: Transform to CipherView and create in vault
        // For now, just return a placeholder result
        Ok(ImportResult {
            items_created: import_data.items.len(),
            folders_created: import_data.folders.len(),
            format: format.to_string(),
        })
    }

    /// List supported import formats
    pub fn supported_formats(&self) -> Vec<FormatInfo> {
        vec![
            FormatInfo {
                name: "bitwardencsv".to_string(),
                display_name: "Bitwarden (csv)".to_string(),
                description: "Bitwarden CSV export".to_string(),
            },
            FormatInfo {
                name: "bitwardenjson".to_string(),
                display_name: "Bitwarden (json)".to_string(),
                description: "Bitwarden JSON export".to_string(),
            },
            FormatInfo {
                name: "lastpass".to_string(),
                display_name: "LastPass".to_string(),
                description: "LastPass CSV export".to_string(),
            },
            FormatInfo {
                name: "1password".to_string(),
                display_name: "1Password".to_string(),
                description: "1Password CSV export".to_string(),
            },
            FormatInfo {
                name: "chrome".to_string(),
                display_name: "Chrome".to_string(),
                description: "Chrome passwords CSV".to_string(),
            },
        ]
    }
}

impl Default for ImportService {
    fn default() -> Self {
        Self::new()
    }
}
