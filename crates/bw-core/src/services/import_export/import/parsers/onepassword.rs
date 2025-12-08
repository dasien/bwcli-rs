//! 1Password CSV import parser

use crate::services::import_export::errors::ImportError;
use crate::services::import_export::import::*;
use async_trait::async_trait;
use csv::ReaderBuilder;

/// 1Password CSV parser
pub struct OnePasswordParser;

impl OnePasswordParser {
    pub fn new() -> Self {
        Self
    }

    fn parse_type(type_str: &str) -> ImportItemType {
        match type_str.to_lowercase().as_str() {
            "login" => ImportItemType::Login,
            "secure note" => ImportItemType::SecureNote,
            "credit card" => ImportItemType::Card,
            "identity" => ImportItemType::Identity,
            _ => ImportItemType::Login,
        }
    }
}

#[async_trait]
impl ImportParser for OnePasswordParser {
    fn format_name(&self) -> &str {
        "1password"
    }

    async fn parse(
        &self,
        data: &[u8],
        _options: &ImportOptions,
    ) -> Result<ImportData, ImportError> {
        let csv_str = std::str::from_utf8(data)
            .map_err(|e| ImportError::ParseError(format!("Invalid UTF-8: {}", e)))?;

        let mut rdr = ReaderBuilder::new().from_reader(csv_str.as_bytes());

        let mut folders = Vec::new();
        let mut items = Vec::new();
        let mut folder_names = std::collections::HashSet::new();

        for result in rdr.records() {
            let record = result?;

            // 1Password CSV format: Title,Website,Username,Password,Notes,Type,Folder
            let title = record.get(0).unwrap_or("").to_string();
            let website = record.get(1).unwrap_or("").to_string();
            let username = record.get(2).unwrap_or("").to_string();
            let password = record.get(3).unwrap_or("").to_string();
            let notes = record.get(4).unwrap_or("").to_string();
            let item_type_str = record.get(5).unwrap_or("Login");
            let folder = record.get(6).unwrap_or("").to_string();

            let item_type = Self::parse_type(item_type_str);

            // Add folder if not empty and not seen
            if !folder.is_empty() && folder_names.insert(folder.clone()) {
                folders.push(ImportFolder {
                    name: folder.clone(),
                });
            }

            // Create login data if type is login
            let login = if item_type == ImportItemType::Login {
                Some(ImportLogin {
                    username: if username.is_empty() {
                        None
                    } else {
                        Some(username)
                    },
                    password: if password.is_empty() {
                        None
                    } else {
                        Some(password)
                    },
                    uris: if website.is_empty() {
                        Vec::new()
                    } else {
                        vec![website]
                    },
                    totp: None,
                })
            } else {
                None
            };

            let item = ImportItem {
                item_type,
                folder_name: if folder.is_empty() {
                    None
                } else {
                    Some(folder)
                },
                favorite: false,
                name: title,
                notes: if notes.is_empty() { None } else { Some(notes) },
                fields: Vec::new(),
                login,
                card: None,
                identity: None,
            };

            items.push(item);
        }

        Ok(ImportData { folders, items })
    }

    fn can_parse(&self, data: &[u8]) -> bool {
        if let Ok(csv_str) = std::str::from_utf8(data) {
            let first_line = csv_str.lines().next().unwrap_or("");
            first_line.contains("Title")
                && first_line.contains("Website")
                && first_line.contains("Type")
        } else {
            false
        }
    }

    fn requires_password(&self) -> bool {
        false
    }
}
