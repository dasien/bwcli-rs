//! Chrome passwords CSV import parser

use crate::services::import_export::errors::ImportError;
use crate::services::import_export::import::*;
use async_trait::async_trait;
use csv::ReaderBuilder;

/// Chrome passwords CSV parser
pub struct ChromeParser;

impl ChromeParser {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportParser for ChromeParser {
    fn format_name(&self) -> &str {
        "chrome"
    }

    async fn parse(
        &self,
        data: &[u8],
        _options: &ImportOptions,
    ) -> Result<ImportData, ImportError> {
        let csv_str = std::str::from_utf8(data)
            .map_err(|e| ImportError::ParseError(format!("Invalid UTF-8: {}", e)))?;

        let mut rdr = ReaderBuilder::new().from_reader(csv_str.as_bytes());

        let items: Vec<ImportItem> = rdr
            .records()
            .filter_map(|result| result.ok())
            .map(|record| {
                // Chrome CSV format: name,url,username,password
                let name = record.get(0).unwrap_or("").to_string();
                let url = record.get(1).unwrap_or("").to_string();
                let username = record.get(2).unwrap_or("").to_string();
                let password = record.get(3).unwrap_or("").to_string();

                let login = Some(ImportLogin {
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
                    uris: if url.is_empty() {
                        Vec::new()
                    } else {
                        vec![url]
                    },
                    totp: None,
                });

                ImportItem {
                    item_type: ImportItemType::Login,
                    folder_name: None,
                    favorite: false,
                    name,
                    notes: None,
                    fields: Vec::new(),
                    login,
                    card: None,
                    identity: None,
                }
            })
            .collect();

        Ok(ImportData {
            folders: Vec::new(),
            items,
        })
    }

    fn can_parse(&self, data: &[u8]) -> bool {
        if let Ok(csv_str) = std::str::from_utf8(data) {
            let first_line = csv_str.lines().next().unwrap_or("");
            first_line.contains("name")
                && first_line.contains("url")
                && first_line.contains("username")
                && first_line.contains("password")
                && !first_line.contains("grouping") // Not LastPass
                && !first_line.contains("Type") // Not 1Password
        } else {
            false
        }
    }

    fn requires_password(&self) -> bool {
        false
    }
}
