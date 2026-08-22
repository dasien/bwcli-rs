//! Import service and parsers

pub mod parsers;
pub mod validator;

use crate::services::import_export::errors::ImportError;
use async_trait::async_trait;
use bitwarden_vault::{
    CardView, CipherRepromptType, CipherType, CipherView, FieldType, FieldView, FolderId,
    IdentityView, LoginUriView, LoginView, SecureNoteType, SecureNoteView,
};
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

impl ImportItem {
    /// Convert into a vault item ready to be created.
    ///
    /// Server-owned fields are placeholders; the write path overwrites them.
    pub fn to_cipher_view(&self, folder_id: Option<FolderId>) -> CipherView {
        let now = chrono::Utc::now();

        let fields: Vec<FieldView> = self
            .fields
            .iter()
            .map(|f| FieldView {
                name: Some(f.name.clone()),
                value: f.value.clone(),
                r#type: match f.field_type {
                    1 => FieldType::Hidden,
                    2 => FieldType::Boolean,
                    3 => FieldType::Linked,
                    _ => FieldType::Text,
                },
                linked_id: None,
            })
            .collect();

        CipherView {
            id: None,
            organization_id: None,
            folder_id,
            collection_ids: vec![],
            key: None,
            name: self.name.clone(),
            notes: self.notes.clone(),
            r#type: match self.item_type {
                ImportItemType::Login => CipherType::Login,
                ImportItemType::SecureNote => CipherType::SecureNote,
                ImportItemType::Card => CipherType::Card,
                ImportItemType::Identity => CipherType::Identity,
            },
            login: self.login.as_ref().map(|l| LoginView {
                username: l.username.clone(),
                password: l.password.clone(),
                password_revision_date: None,
                uris: Some(
                    l.uris
                        .iter()
                        .map(|u| LoginUriView {
                            uri: Some(u.clone()),
                            r#match: None,
                            uri_checksum: None,
                        })
                        .collect(),
                ),
                totp: l.totp.clone(),
                autofill_on_page_load: None,
                fido2_credentials: None,
            }),
            identity: self.identity.as_ref().map(|i| IdentityView {
                title: i.title.clone(),
                first_name: i.first_name.clone(),
                middle_name: i.middle_name.clone(),
                last_name: i.last_name.clone(),
                address1: i.address1.clone(),
                address2: i.address2.clone(),
                address3: i.address3.clone(),
                city: i.city.clone(),
                state: i.state.clone(),
                postal_code: i.postal_code.clone(),
                country: i.country.clone(),
                company: None,
                email: i.email.clone(),
                phone: i.phone.clone(),
                ssn: i.ssn.clone(),
                username: i.username.clone(),
                passport_number: i.passport_number.clone(),
                license_number: i.license_number.clone(),
            }),
            card: self.card.as_ref().map(|c| CardView {
                cardholder_name: c.cardholder_name.clone(),
                exp_month: c.exp_month.clone(),
                exp_year: c.exp_year.clone(),
                code: c.code.clone(),
                brand: c.brand.clone(),
                number: c.number.clone(),
            }),
            secure_note: matches!(self.item_type, ImportItemType::SecureNote).then_some(
                SecureNoteView {
                    r#type: SecureNoteType::Generic,
                },
            ),
            ssh_key: None,
            bank_account: None,
            drivers_license: None,
            passport: None,
            favorite: self.favorite,
            reprompt: CipherRepromptType::None,
            organization_use_totp: false,
            edit: true,
            permissions: None,
            view_password: true,
            local_data: None,
            attachments: None,
            attachment_decryption_failures: None,
            fields: (!fields.is_empty()).then_some(fields),
            password_history: None,
            creation_date: now,
            deleted_date: None,
            revision_date: now,
            archived_date: None,
        }
    }
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

    /// Read, parse and validate an import file.
    ///
    /// Does not touch the vault — callers create the items. Split out from
    /// [`Self::import`] so the CLI can turn the parsed data into real vault
    /// entries.
    pub async fn parse_file(
        &self,
        format: &str,
        file_path: &str,
        options: ImportOptions,
    ) -> Result<ImportData, ImportError> {
        self.read_parse_validate(format, file_path, options).await
    }

    /// Parse and validate an import file, reporting what *would* be created.
    ///
    /// Note this does not write to the vault; use [`Self::parse_file`] plus the
    /// vault write path for that.
    pub async fn import(
        &self,
        format: &str,
        file_path: &str,
        options: ImportOptions,
    ) -> Result<ImportResult, ImportError> {
        let import_data = self.read_parse_validate(format, file_path, options).await?;

        Ok(ImportResult {
            items_created: import_data.items.len(),
            folders_created: import_data.folders.len(),
            format: format.to_string(),
        })
    }

    async fn read_parse_validate(
        &self,
        format: &str,
        file_path: &str,
        options: ImportOptions,
    ) -> Result<ImportData, ImportError> {
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

        // An empty or header-only file parses cleanly into zero items, which
        // would otherwise be reported as a successful import of nothing.
        if import_data.items.is_empty() && import_data.folders.is_empty() {
            return Err(ImportError::ParseError(format!(
                "no data found in import file '{}'",
                file_path
            )));
        }

        // Validate
        validator::validate(&import_data)?;

        Ok(import_data)
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
