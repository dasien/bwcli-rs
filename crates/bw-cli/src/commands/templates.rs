//! Template generation module for vault items
//!
//! Provides JSON templates matching the TypeScript CLI format exactly
//! for creating vault items, folders, and other entities.

use serde_json::{Value, json};
use thiserror::Error;

/// Template error types
#[derive(Error, Debug)]
pub enum TemplateError {
    #[error(
        "Unknown template type: {0}. Valid types: item, item.login, item.secureNote, item.card, item.identity, folder, item.field, item.login.uri"
    )]
    UnknownTemplate(String),
}

/// Get item template by type
///
/// # Arguments
/// * `template_type` - Template type to generate
///
/// # Supported types
/// - `item` or `item.login` - Login item template
/// - `item.secureNote` or `item.securenote` - Secure note template
/// - `item.card` - Card (payment) template
/// - `item.identity` - Identity template
/// - `folder` - Folder template
/// - `item.field` - Custom field template
/// - `item.login.uri` - Login URI template
pub fn get_item_template(template_type: &str) -> Result<Value, TemplateError> {
    match template_type.to_lowercase().as_str() {
        "item" | "item.login" => Ok(login_template()),
        "item.securenote" => Ok(secure_note_template()),
        "item.card" => Ok(card_template()),
        "item.identity" => Ok(identity_template()),
        "folder" => Ok(folder_template()),
        "item.field" => Ok(field_template()),
        "item.login.uri" => Ok(uri_template()),
        _ => Err(TemplateError::UnknownTemplate(template_type.to_string())),
    }
}

/// Login item template (type=1)
fn login_template() -> Value {
    json!({
        "organizationId": null,
        "collectionIds": null,
        "folderId": null,
        "type": 1,
        "name": "Item name",
        "notes": "Some notes about this item.",
        "favorite": false,
        "fields": [],
        "login": {
            "uris": [
                {
                    "match": null,
                    "uri": "https://example.com"
                }
            ],
            "username": "jdoe",
            "password": "myp@ssword123",
            "totp": null
        },
        "secureNote": null,
        "card": null,
        "identity": null,
        "reprompt": 0
    })
}

/// Secure note template (type=2)
fn secure_note_template() -> Value {
    json!({
        "organizationId": null,
        "collectionIds": null,
        "folderId": null,
        "type": 2,
        "name": "Item name",
        "notes": "Some notes about this item.",
        "favorite": false,
        "fields": [],
        "login": null,
        "secureNote": {
            "type": 0
        },
        "card": null,
        "identity": null,
        "reprompt": 0
    })
}

/// Card (payment) template (type=3)
fn card_template() -> Value {
    json!({
        "organizationId": null,
        "collectionIds": null,
        "folderId": null,
        "type": 3,
        "name": "Item name",
        "notes": "Some notes about this item.",
        "favorite": false,
        "fields": [],
        "login": null,
        "secureNote": null,
        "card": {
            "cardholderName": "John Doe",
            "brand": "visa",
            "number": "4242424242424242",
            "expMonth": "04",
            "expYear": "2025",
            "code": "123"
        },
        "identity": null,
        "reprompt": 0
    })
}

/// Identity template (type=4)
fn identity_template() -> Value {
    json!({
        "organizationId": null,
        "collectionIds": null,
        "folderId": null,
        "type": 4,
        "name": "Item name",
        "notes": "Some notes about this item.",
        "favorite": false,
        "fields": [],
        "login": null,
        "secureNote": null,
        "card": null,
        "identity": {
            "title": "Mr",
            "firstName": "John",
            "middleName": "William",
            "lastName": "Doe",
            "address1": "123 Main St",
            "address2": "Apt 1",
            "address3": null,
            "city": "New York",
            "state": "NY",
            "postalCode": "10001",
            "country": "US",
            "company": "Acme Inc",
            "email": "jdoe@example.com",
            "phone": "555-123-4567",
            "ssn": "123-45-6789",
            "username": "jdoe",
            "passportNumber": "123456789",
            "licenseNumber": "D1234567"
        },
        "reprompt": 0
    })
}

/// Folder template
fn folder_template() -> Value {
    json!({
        "name": "Folder name"
    })
}

/// Custom field template
fn field_template() -> Value {
    json!({
        "name": "Field name",
        "value": "Some value",
        "type": 0
    })
}

/// Login URI template
fn uri_template() -> Value {
    json!({
        "match": null,
        "uri": "https://example.com"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_template_structure() {
        let template = get_item_template("item.login").unwrap();
        assert_eq!(template["type"], 1);
        assert!(template["login"].is_object());
        assert!(template["login"]["uris"].is_array());
    }

    #[test]
    fn test_secure_note_template_structure() {
        let template = get_item_template("item.secureNote").unwrap();
        assert_eq!(template["type"], 2);
        assert!(template["secureNote"].is_object());
        assert_eq!(template["secureNote"]["type"], 0);
    }

    #[test]
    fn test_card_template_structure() {
        let template = get_item_template("item.card").unwrap();
        assert_eq!(template["type"], 3);
        assert!(template["card"].is_object());
        assert!(template["card"]["number"].is_string());
    }

    #[test]
    fn test_identity_template_structure() {
        let template = get_item_template("item.identity").unwrap();
        assert_eq!(template["type"], 4);
        assert!(template["identity"].is_object());
        assert!(template["identity"]["firstName"].is_string());
    }

    #[test]
    fn test_folder_template_structure() {
        let template = get_item_template("folder").unwrap();
        assert!(template["name"].is_string());
    }

    #[test]
    fn test_field_template_structure() {
        let template = get_item_template("item.field").unwrap();
        assert!(template["name"].is_string());
        assert!(template["value"].is_string());
        assert_eq!(template["type"], 0);
    }

    #[test]
    fn test_uri_template_structure() {
        let template = get_item_template("item.login.uri").unwrap();
        assert!(template["uri"].is_string());
    }

    #[test]
    fn test_unknown_template() {
        let result = get_item_template("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_case_insensitive() {
        // Should work with different case variations
        assert!(get_item_template("ITEM").is_ok());
        assert!(get_item_template("Item.Login").is_ok());
        assert!(get_item_template("item.SECURENOTE").is_ok());
    }

    #[test]
    fn test_item_alias() {
        // "item" should be an alias for "item.login"
        let item = get_item_template("item").unwrap();
        let login = get_item_template("item.login").unwrap();
        assert_eq!(item, login);
    }
}
