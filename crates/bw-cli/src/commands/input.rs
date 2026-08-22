//! Input parsing module for vault create/edit commands
//!
//! Supports parsing JSON input from:
//! - Base64-encoded JSON (TypeScript CLI compatible)
//! - Raw JSON (detected by leading '{')
//! - Stdin (detected by "-" argument)

use base64::Engine;
use bitwarden_collections::collection::CollectionId;
use bitwarden_core::OrganizationId;
use bitwarden_vault::{
    CardView, CipherRepromptType, CipherType, CipherView, FieldView, FolderId, IdentityView,
    LoginView, SecureNoteView, SshKeyView,
};
use chrono::Utc;
use serde::Deserialize;
use std::io::Read;
use thiserror::Error;

/// Maximum input size to prevent DoS attacks
const MAX_INPUT_SIZE: usize = 1_000_000; // 1MB

/// Input parsing error types
#[derive(Error, Debug)]
pub enum InputError {
    #[error("Invalid base64 encoding: {0}")]
    Base64DecodeError(String),

    #[error("Invalid JSON: {0}")]
    JsonParseError(String),

    #[error("Failed to read stdin: {0}")]
    StdinError(String),

    #[error("Input too large (max {MAX_INPUT_SIZE} bytes)")]
    InputTooLarge,

    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Folder input structure for creation/update
#[derive(Debug, Clone, Deserialize)]
pub struct FolderInput {
    pub name: String,
}

/// Parse item JSON input from various formats
///
/// Supports:
/// 1. Base64-encoded JSON (TypeScript CLI compatible)
/// 2. Raw JSON (detected by leading '{')
/// 3. Stdin (detected by "-" argument)
pub fn parse_item_input(input: &str) -> Result<CipherView, InputError> {
    let json_string = get_json_string(input)?;
    parse_cipher_view(&json_string)
}

/// Parse folder JSON input from various formats
pub fn parse_folder_input(input: &str) -> Result<FolderInput, InputError> {
    let json_string = get_json_string(input)?;
    parse_folder(&json_string)
}

/// Get JSON string from input (handling stdin, base64, raw JSON)
pub(crate) fn get_json_string(input: &str) -> Result<String, InputError> {
    // 1. If input is "-", read from stdin
    if input == "-" {
        return read_stdin();
    }

    let trimmed = input.trim();

    // 2. If input starts with '{' or '[', treat as raw JSON
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Ok(trimmed.to_string());
    }

    // 3. Otherwise, try base64 decoding
    decode_base64(trimmed)
}

/// Read input from stdin
fn read_stdin() -> Result<String, InputError> {
    let mut buffer = String::new();
    let mut stdin = std::io::stdin();
    let mut limited_reader = stdin.by_ref().take(MAX_INPUT_SIZE as u64 + 1);

    limited_reader
        .read_to_string(&mut buffer)
        .map_err(|e| InputError::StdinError(e.to_string()))?;

    if buffer.len() > MAX_INPUT_SIZE {
        return Err(InputError::InputTooLarge);
    }

    let trimmed = buffer.trim().to_string();

    // Stdin content might also be base64 encoded
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        Ok(trimmed)
    } else {
        decode_base64(&trimmed)
    }
}

/// Decode base64-encoded JSON string
fn decode_base64(input: &str) -> Result<String, InputError> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| InputError::Base64DecodeError(e.to_string()))?;

    String::from_utf8(decoded).map_err(|e| InputError::Base64DecodeError(e.to_string()))
}

/// User-supplied shape of a vault item.
///
/// Deliberately *not* `CipherView`. `CipherView` is a domain type whose
/// server-owned fields (`creationDate`, `revisionDate`, `edit`,
/// `viewPassword`, `organizationUseTotp`, ...) are required, and whose
/// `collectionIds` is a non-nullable sequence. Deserializing user input
/// straight into it meant `bw get template item | bw create item` failed on the
/// CLI's own template output. Everything here is optional/defaulted, and
/// server-owned fields are filled in by `into_cipher_view`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CipherInput {
    organization_id: Option<OrganizationId>,
    folder_id: Option<FolderId>,
    #[serde(deserialize_with = "null_as_default")]
    collection_ids: Vec<CollectionId>,
    r#type: Option<CipherType>,
    name: String,
    notes: Option<String>,
    favorite: bool,
    reprompt: Option<CipherRepromptType>,
    fields: Option<Vec<FieldView>>,
    login: Option<LoginView>,
    secure_note: Option<SecureNoteView>,
    card: Option<CardView>,
    identity: Option<IdentityView>,
    ssh_key: Option<SshKeyView>,
}

/// Treat an explicit `null` as the type's default.
///
/// The item templates emit `"collectionIds": null`, which serde otherwise
/// rejects for a non-Option sequence.
fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

impl CipherInput {
    fn into_cipher_view(self) -> CipherView {
        // Server-owned timestamps are placeholders; WriteService overwrites
        // them before submission.
        let now = Utc::now();

        CipherView {
            id: None,
            organization_id: self.organization_id,
            folder_id: self.folder_id,
            collection_ids: self.collection_ids,
            key: None,
            name: self.name,
            notes: self.notes,
            r#type: self.r#type.unwrap_or(CipherType::Login),
            login: self.login,
            identity: self.identity,
            card: self.card,
            secure_note: self.secure_note,
            ssh_key: self.ssh_key,
            bank_account: None,
            drivers_license: None,
            passport: None,
            favorite: self.favorite,
            reprompt: self.reprompt.unwrap_or(CipherRepromptType::None),
            organization_use_totp: false,
            edit: true,
            permissions: None,
            view_password: true,
            local_data: None,
            attachments: None,
            attachment_decryption_failures: None,
            fields: self.fields,
            password_history: None,
            creation_date: now,
            deleted_date: None,
            revision_date: now,
            archived_date: None,
        }
    }
}

/// Parse JSON string into CipherView
fn parse_cipher_view(json_str: &str) -> Result<CipherView, InputError> {
    let input: CipherInput =
        serde_json::from_str(json_str).map_err(|e| InputError::JsonParseError(e.to_string()))?;

    if input.name.trim().is_empty() {
        return Err(InputError::MissingField("name".to_string()));
    }

    Ok(input.into_cipher_view())
}

/// Parse JSON string into FolderInput
fn parse_folder(json_str: &str) -> Result<FolderInput, InputError> {
    let folder: FolderInput =
        serde_json::from_str(json_str).map_err(|e| InputError::JsonParseError(e.to_string()))?;

    if folder.name.is_empty() {
        return Err(InputError::MissingField("name".to_string()));
    }

    Ok(folder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    /// Minimal item JSON a user would realistically type. Server-owned fields
    /// are deliberately absent — the parser must supply them.
    const MINIMAL_ITEM_JSON: &str = r#"{"type":1,"name":"Test Item","notes":null,"favorite":false,"collectionIds":null,"login":{"username":"user","password":"pass","uris":[],"totp":null}}"#;

    #[test]
    fn test_parse_raw_json() {
        let result = parse_item_input(MINIMAL_ITEM_JSON);
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
        let cipher = result.unwrap();
        assert_eq!(cipher.name, "Test Item");
        // Not supplied by the user, so the parser fills them in.
        assert!(cipher.collection_ids.is_empty());
        assert!(cipher.edit);
        assert!(cipher.view_password);
    }

    /// Regression guard: `bw get template <type>` output must be accepted by
    /// `bw create item`. This round trip was broken because the parser
    /// deserialized directly into `CipherView`, which rejects the templates'
    /// `"collectionIds": null` and requires server-owned date fields.
    #[test]
    fn test_item_templates_are_parseable() {
        for template in [
            "item",
            "item.login",
            "item.securenote",
            "item.card",
            "item.identity",
        ] {
            let json = super::super::templates::get_item_template(template)
                .unwrap_or_else(|e| panic!("template {template} should exist: {e}"));
            let rendered = serde_json::to_string(&json).unwrap();

            let parsed = parse_item_input(&rendered);
            assert!(
                parsed.is_ok(),
                "template `{template}` is not accepted by `create item`: {:?}",
                parsed.err()
            );
        }
    }

    #[test]
    fn test_parse_base64_json() {
        let encoded = base64::engine::general_purpose::STANDARD.encode(MINIMAL_ITEM_JSON);
        let result = parse_item_input(&encoded);
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
        let cipher = result.unwrap();
        assert_eq!(cipher.name, "Test Item");
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_item_input("{invalid}");
        assert!(matches!(result, Err(InputError::JsonParseError(_))));
    }

    #[test]
    fn test_parse_invalid_base64() {
        // "hello world" is not valid base64 with special chars
        let result = parse_item_input("not-valid-base64!!!");
        assert!(matches!(result, Err(InputError::Base64DecodeError(_))));
    }

    #[test]
    fn test_parse_folder_json() {
        let input = r#"{"name":"My Folder"}"#;
        let result = parse_folder_input(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "My Folder");
    }

    #[test]
    fn test_parse_folder_base64() {
        let json = r#"{"name":"My Folder"}"#;
        let encoded = base64::engine::general_purpose::STANDARD.encode(json);
        let result = parse_folder_input(&encoded);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "My Folder");
    }

    #[test]
    fn test_parse_folder_empty_name() {
        let input = r#"{"name":""}"#;
        let result = parse_folder_input(input);
        assert!(matches!(result, Err(InputError::MissingField(_))));
    }
}
