//! Bitwarden JSON import parser
//!
//! Deserializes into its own lenient wire types rather than the SDK's
//! `CipherView`. Exports produced by older clients (or trimmed by hand) omit
//! server-owned fields like `edit` and `viewPassword`, and `CipherView`
//! requires them — importing such a file would fail with an opaque serde error.

use crate::services::import_export::errors::ImportError;
use crate::services::import_export::import::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;

/// Bitwarden JSON parser
pub struct BitwardenJsonParser;

// ---------------------------------------------------------------------------
// Wire format
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct JsonExport {
    encrypted: bool,
    folders: Vec<JsonFolder>,
    items: Vec<JsonItem>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct JsonFolder {
    /// Kept as a string: folder linkage is by equality, and treating it as a
    /// typed UUID would reject otherwise-importable files.
    id: Option<String>,
    name: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct JsonItem {
    #[serde(rename = "type")]
    item_type: u8,
    folder_id: Option<String>,
    favorite: bool,
    name: String,
    notes: Option<String>,
    fields: Vec<JsonField>,
    login: Option<JsonLogin>,
    card: Option<JsonCard>,
    identity: Option<JsonIdentity>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct JsonField {
    name: Option<String>,
    value: Option<String>,
    #[serde(rename = "type")]
    field_type: u8,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct JsonLogin {
    username: Option<String>,
    password: Option<String>,
    totp: Option<String>,
    uris: Vec<JsonUri>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct JsonUri {
    uri: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct JsonCard {
    cardholder_name: Option<String>,
    number: Option<String>,
    brand: Option<String>,
    exp_month: Option<String>,
    exp_year: Option<String>,
    code: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct JsonIdentity {
    title: Option<String>,
    first_name: Option<String>,
    middle_name: Option<String>,
    last_name: Option<String>,
    address1: Option<String>,
    address2: Option<String>,
    address3: Option<String>,
    city: Option<String>,
    state: Option<String>,
    postal_code: Option<String>,
    country: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    ssn: Option<String>,
    username: Option<String>,
    passport_number: Option<String>,
    license_number: Option<String>,
}

impl BitwardenJsonParser {
    pub fn new() -> Self {
        Self
    }

    /// Map the numeric cipher type used in exports.
    ///
    /// Types without a dedicated `ImportItemType` (SSH keys, and the bank
    /// account / driver's license / passport types added in SDK 3.0) are
    /// preserved as notes rather than dropped.
    fn item_type(raw: u8) -> ImportItemType {
        match raw {
            1 => ImportItemType::Login,
            2 => ImportItemType::SecureNote,
            3 => ImportItemType::Card,
            4 => ImportItemType::Identity,
            _ => ImportItemType::SecureNote,
        }
    }
}

#[async_trait]
impl ImportParser for BitwardenJsonParser {
    fn format_name(&self) -> &str {
        "bitwardenjson"
    }

    async fn parse(&self, data: &[u8], _options: &ImportOptions) -> Result<ImportData, ImportError> {
        let export: JsonExport = serde_json::from_slice(data)?;

        if export.encrypted {
            return Err(ImportError::ParseError(
                "This is an encrypted export. Use the encrypted_json format instead".to_string(),
            ));
        }

        let folders = export
            .folders
            .iter()
            .map(|f| ImportFolder {
                name: f.name.clone(),
            })
            .collect();

        let folder_map: HashMap<&str, &str> = export
            .folders
            .iter()
            .filter_map(|f| f.id.as_deref().map(|id| (id, f.name.as_str())))
            .collect();

        let items = export
            .items
            .iter()
            .map(|item| {
                let folder_name = item
                    .folder_id
                    .as_deref()
                    .and_then(|id| folder_map.get(id))
                    .map(|name| (*name).to_string());

                let login = item.login.as_ref().map(|l| ImportLogin {
                    username: l.username.clone(),
                    password: l.password.clone(),
                    totp: l.totp.clone(),
                    uris: l.uris.iter().filter_map(|u| u.uri.clone()).collect(),
                });

                let card = item.card.as_ref().map(|c| ImportCard {
                    cardholder_name: c.cardholder_name.clone(),
                    number: c.number.clone(),
                    brand: c.brand.clone(),
                    exp_month: c.exp_month.clone(),
                    exp_year: c.exp_year.clone(),
                    code: c.code.clone(),
                });

                let identity = item.identity.as_ref().map(|i| ImportIdentity {
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

                let fields = item
                    .fields
                    .iter()
                    .map(|f| ImportField {
                        name: f.name.clone().unwrap_or_default(),
                        value: f.value.clone(),
                        field_type: f.field_type,
                    })
                    .collect();

                ImportItem {
                    item_type: Self::item_type(item.item_type),
                    folder_name,
                    favorite: item.favorite,
                    name: item.name.clone(),
                    notes: item.notes.clone(),
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

impl Default for BitwardenJsonParser {
    fn default() -> Self {
        Self::new()
    }
}
