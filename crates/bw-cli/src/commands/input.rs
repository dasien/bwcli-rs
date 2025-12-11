//! Input parsing module for vault create/edit commands
//!
//! Supports parsing JSON input from:
//! - Base64-encoded JSON (TypeScript CLI compatible)
//! - Raw JSON (detected by leading '{')
//! - Stdin (detected by "-" argument)

use base64::Engine;
use bw_core::models::vault::CipherView;
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
fn get_json_string(input: &str) -> Result<String, InputError> {
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

/// Parse JSON string into CipherView
fn parse_cipher_view(json_str: &str) -> Result<CipherView, InputError> {
    serde_json::from_str(json_str).map_err(|e| InputError::JsonParseError(e.to_string()))
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

    #[test]
    fn test_parse_raw_json() {
        let input = r#"{"id":"","type":1,"name":"Test Item","notes":null,"favorite":false,"collectionIds":[],"revisionDate":"","login":{"username":"user","password":"pass","uris":[],"totp":null}}"#;
        let result = parse_item_input(input);
        assert!(result.is_ok());
        let cipher = result.unwrap();
        assert_eq!(cipher.name, "Test Item");
    }

    #[test]
    fn test_parse_base64_json() {
        let json = r#"{"id":"","type":1,"name":"Test Item","notes":null,"favorite":false,"collectionIds":[],"revisionDate":"","login":{"username":"user","password":"pass","uris":[],"totp":null}}"#;
        let encoded = base64::engine::general_purpose::STANDARD.encode(json);
        let result = parse_item_input(&encoded);
        assert!(result.is_ok());
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
