//! Bitwarden JSON import parser

use crate::models::vault::{CipherType, FolderId};
use crate::services::import_export::errors::ImportError;
use crate::services::import_export::export::formatters::json::JsonExportOwned;
use crate::services::import_export::import::*;
use async_trait::async_trait;

/// Bitwarden JSON parser
pub struct BitwardenJsonParser;

impl BitwardenJsonParser {
    pub fn new() -> Self {
        Self
    }

    fn cipher_type_to_import_type(cipher_type: CipherType) -> ImportItemType {
        match cipher_type {
            CipherType::Login => ImportItemType::Login,
            CipherType::SecureNote => ImportItemType::SecureNote,
            CipherType::Card => ImportItemType::Card,
            CipherType::Identity => ImportItemType::Identity,
            CipherType::SshKey => ImportItemType::SecureNote, // Map SSH keys to SecureNote for now
        }
    }
}

#[async_trait]
impl ImportParser for BitwardenJsonParser {
    fn format_name(&self) -> &str {
        "bitwardenjson"
    }

    async fn parse(
        &self,
        data: &[u8],
        _options: &ImportOptions,
    ) -> Result<ImportData, ImportError> {
        let export: JsonExportOwned = serde_json::from_slice(data)?;

        // Check it's not encrypted
        if export.encrypted {
            return Err(ImportError::ParseError(
                "This is an encrypted export. Use the encrypted_json format instead".to_string(),
            ));
        }

        // Convert folders
        let folders = export
            .folders
            .iter()
            .map(|f| ImportFolder {
                name: f.name.clone(),
            })
            .collect();

        // Build folder ID to name map - FolderView.id is Option<FolderId>
        let folder_map: std::collections::HashMap<FolderId, String> = export
            .folders
            .iter()
            .filter_map(|f| f.id.clone().map(|id| (id, f.name.clone())))
            .collect();

        // Convert items
        let items = export
            .items
            .iter()
            .map(|cipher| {
                // cipher.folder_id is Option<FolderId>
                let folder_name = cipher
                    .folder_id
                    .as_ref()
                    .and_then(|id| folder_map.get(id))
                    .cloned();

                // login.uris is Option<Vec<LoginUriView>>
                let login = cipher.login.as_ref().map(|l| ImportLogin {
                    username: l.username.clone(),
                    password: l.password.clone(),
                    totp: l.totp.clone(),
                    uris: l
                        .uris
                        .as_ref()
                        .map(|uris| uris.iter().filter_map(|u| u.uri.clone()).collect())
                        .unwrap_or_default(),
                });

                let card = cipher.card.as_ref().map(|c| ImportCard {
                    cardholder_name: c.cardholder_name.clone(),
                    number: c.number.clone(),
                    brand: c.brand.clone(),
                    exp_month: c.exp_month.clone(),
                    exp_year: c.exp_year.clone(),
                    code: c.code.clone(),
                });

                let identity = cipher.identity.as_ref().map(|i| ImportIdentity {
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
                    phone: i.phone.clone(),
                    email: i.email.clone(),
                    ssn: i.ssn.clone(),
                    username: i.username.clone(),
                    passport_number: i.passport_number.clone(),
                    license_number: i.license_number.clone(),
                });

                // cipher.fields is Option<Vec<FieldView>>
                let fields = cipher
                    .fields
                    .as_ref()
                    .map(|fields| {
                        fields
                            .iter()
                            .map(|f| ImportField {
                                name: f.name.clone().unwrap_or_default(),
                                value: f.value.clone(),
                                field_type: f.r#type as u8,
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                ImportItem {
                    // Use r#type instead of cipher_type
                    item_type: Self::cipher_type_to_import_type(cipher.r#type),
                    folder_name,
                    favorite: cipher.favorite,
                    name: cipher.name.clone(),
                    notes: cipher.notes.clone(),
                    fields,
                    login,
                    card,
                    identity,
                }
            })
            .collect();

        Ok(ImportData { folders, items })
    }

    fn can_parse(&self, data: &[u8]) -> bool {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(data) {
            // Check for Bitwarden JSON structure
            json.get("items").is_some()
                && json.get("folders").is_some()
                && json.get("encrypted") == Some(&serde_json::Value::Bool(false))
        } else {
            false
        }
    }

    fn requires_password(&self) -> bool {
        false
    }
}
