//! LastPass CSV import parser

use super::non_empty;
use crate::services::import_export::errors::ImportError;
use crate::services::import_export::import::*;
use async_trait::async_trait;
use csv::ReaderBuilder;

/// LastPass CSV parser
pub struct LastPassParser;

impl LastPassParser {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ImportParser for LastPassParser {
    fn format_name(&self) -> &str {
        "lastpass"
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

            // LastPass CSV format: url,username,password,extra,name,grouping,fav
            let url = record.get(0).unwrap_or("").to_string();
            let username = record.get(1).unwrap_or("").to_string();
            let password = record.get(2).unwrap_or("").to_string();
            let extra = record.get(3).unwrap_or("").to_string(); // notes
            let name = record.get(4).unwrap_or("").to_string();
            let grouping = record.get(5).unwrap_or("").to_string(); // folder
            let fav = record.get(6).unwrap_or("0") == "1";

            // Add folder if not empty and not seen
            if !grouping.is_empty() && folder_names.insert(grouping.clone()) {
                folders.push(ImportFolder {
                    name: grouping.clone(),
                });
            }

            // Create login item
            let login = Some(ImportLogin {
                username: non_empty(&username),
                password: non_empty(&password),
                uris: if url.is_empty() {
                    Vec::new()
                } else {
                    vec![url]
                },
                totp: None,
            });

            let item = ImportItem {
                item_type: ImportItemType::Login,
                folder_name: non_empty(&grouping),
                favorite: fav,
                name,
                notes: non_empty(&extra),
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
            first_line.contains("url")
                && first_line.contains("username")
                && first_line.contains("grouping")
        } else {
            false
        }
    }

    fn requires_password(&self) -> bool {
        false
    }
}
