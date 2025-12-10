//! Bitwarden CSV import parser

use super::non_empty;
use crate::services::import_export::errors::ImportError;
use crate::services::import_export::import::*;
use async_trait::async_trait;
use csv::ReaderBuilder;

/// Bitwarden CSV parser
pub struct BitwardenCsvParser;

impl BitwardenCsvParser {
    pub fn new() -> Self {
        Self
    }

    fn parse_type(type_str: &str) -> ImportItemType {
        match type_str.to_lowercase().as_str() {
            "login" => ImportItemType::Login,
            "note" => ImportItemType::SecureNote,
            "card" => ImportItemType::Card,
            "identity" => ImportItemType::Identity,
            _ => ImportItemType::Login, // Default to login
        }
    }

    fn parse_fields(fields_str: &str) -> Vec<ImportField> {
        if fields_str.is_empty() {
            return Vec::new();
        }

        fields_str
            .split('\n')
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    Some(ImportField {
                        name: parts[0].trim().to_string(),
                        value: Some(parts[1].trim().to_string()),
                        field_type: 0, // Text field
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[async_trait]
impl ImportParser for BitwardenCsvParser {
    fn format_name(&self) -> &str {
        "bitwardencsv"
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

            // Parse record fields
            let folder_name = record.get(0).unwrap_or("").to_string();
            let favorite = record.get(1).unwrap_or("0") == "1";
            let item_type = Self::parse_type(record.get(2).unwrap_or("login"));
            let name = record.get(3).unwrap_or("").to_string();
            let notes = record.get(4).map(|s| s.to_string());
            let fields_str = record.get(5).unwrap_or("");
            let fields = Self::parse_fields(fields_str);

            // Add folder if not empty and not seen
            if !folder_name.is_empty() && folder_names.insert(folder_name.clone()) {
                folders.push(ImportFolder {
                    name: folder_name.clone(),
                });
            }

            // Parse type-specific data
            let login = if item_type == ImportItemType::Login {
                let uris = record
                    .get(7)
                    .unwrap_or("")
                    .split('\n')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();

                Some(ImportLogin {
                    username: record.get(8).and_then(|s| non_empty(s)),
                    password: record.get(9).and_then(|s| non_empty(s)),
                    totp: record.get(10).and_then(|s| non_empty(s)),
                    uris,
                })
            } else {
                None
            };

            let item = ImportItem {
                item_type,
                folder_name: if folder_name.is_empty() {
                    None
                } else {
                    Some(folder_name)
                },
                favorite,
                name,
                notes,
                fields,
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
            first_line.contains("login_uri") && first_line.contains("login_username")
        } else {
            false
        }
    }

    fn requires_password(&self) -> bool {
        false
    }
}
