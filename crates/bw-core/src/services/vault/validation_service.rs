//! Input validation service for vault write operations

use crate::models::vault::{CipherType, CipherView, ValidationError};
use regex::Regex;

/// Field length limits for vault items
///
/// These limits match the Bitwarden server's validation rules.
pub mod limits {
    /// Maximum length for cipher name field
    pub const CIPHER_NAME_MAX_LEN: usize = 1000;
    /// Maximum length for cipher notes field
    pub const CIPHER_NOTES_MAX_LEN: usize = 10000;
    /// Maximum length for login URI field
    pub const CIPHER_URI_MAX_LEN: usize = 10000;
    /// Maximum length for folder name field
    pub const FOLDER_NAME_MAX_LEN: usize = 1000;
}

/// Service for validating cipher and folder inputs before submission
pub struct ValidationService {
    uuid_regex: Regex,
}

impl ValidationService {
    /// Create new validation service
    pub fn new() -> Self {
        Self {
            uuid_regex: Regex::new(
                r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$",
            )
            .unwrap(),
        }
    }

    /// Validate cipher for creation
    pub fn validate_cipher_create(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        // Required fields
        self.validate_required_fields(cipher)?;

        // Type-specific validation
        match cipher.cipher_type {
            CipherType::Login => self.validate_login(cipher)?,
            CipherType::SecureNote => self.validate_secure_note(cipher)?,
            CipherType::Card => self.validate_card(cipher)?,
            CipherType::Identity => self.validate_identity(cipher)?,
            CipherType::SshKey => {} // No specific validation for SSH keys yet
        }

        // Field constraints
        self.validate_field_lengths(cipher)?;

        // UUID validation
        self.validate_uuids(cipher)?;

        Ok(())
    }

    /// Validate cipher for update
    pub fn validate_cipher_update(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        // ID must be present for updates
        if cipher.id.is_empty() {
            return Err(ValidationError::MissingField("id".to_string()));
        }

        self.validate_cipher_create(cipher)?;
        Ok(())
    }

    /// Validate folder name
    pub fn validate_folder_name(&self, name: &str) -> Result<(), ValidationError> {
        if name.is_empty() {
            return Err(ValidationError::EmptyField("name".to_string()));
        }

        if name.len() > limits::FOLDER_NAME_MAX_LEN {
            return Err(ValidationError::FieldTooLong {
                field: "name".to_string(),
                max: limits::FOLDER_NAME_MAX_LEN,
                actual: name.len(),
            });
        }

        Ok(())
    }

    // Private validation helpers

    fn validate_required_fields(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        if cipher.name.is_empty() {
            return Err(ValidationError::MissingField("name".to_string()));
        }
        Ok(())
    }

    fn validate_login(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        if cipher.login.is_none() {
            return Err(ValidationError::TypeMismatch {
                cipher_type: "Login".to_string(),
                field: "login".to_string(),
            });
        }

        let login = cipher.login.as_ref().unwrap();

        // Validate URIs
        for uri_view in &login.uris {
            if let Some(uri_str) = &uri_view.uri {
                if uri_str.len() > limits::CIPHER_URI_MAX_LEN {
                    return Err(ValidationError::FieldTooLong {
                        field: "uri".to_string(),
                        max: limits::CIPHER_URI_MAX_LEN,
                        actual: uri_str.len(),
                    });
                }
            }
        }

        // Validate TOTP format if present
        if let Some(totp) = &login.totp {
            self.validate_totp_format(totp)?;
        }

        Ok(())
    }

    fn validate_secure_note(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        if cipher.secure_note.is_none() {
            return Err(ValidationError::TypeMismatch {
                cipher_type: "SecureNote".to_string(),
                field: "secure_note".to_string(),
            });
        }
        Ok(())
    }

    fn validate_card(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        if cipher.card.is_none() {
            return Err(ValidationError::TypeMismatch {
                cipher_type: "Card".to_string(),
                field: "card".to_string(),
            });
        }
        Ok(())
    }

    fn validate_identity(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        if cipher.identity.is_none() {
            return Err(ValidationError::TypeMismatch {
                cipher_type: "Identity".to_string(),
                field: "identity".to_string(),
            });
        }
        Ok(())
    }

    fn validate_field_lengths(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        // Name: max 1000 chars
        if cipher.name.len() > limits::CIPHER_NAME_MAX_LEN {
            return Err(ValidationError::FieldTooLong {
                field: "name".to_string(),
                max: limits::CIPHER_NAME_MAX_LEN,
                actual: cipher.name.len(),
            });
        }

        // Notes: max 10000 chars
        if let Some(notes) = &cipher.notes {
            if notes.len() > limits::CIPHER_NOTES_MAX_LEN {
                return Err(ValidationError::FieldTooLong {
                    field: "notes".to_string(),
                    max: limits::CIPHER_NOTES_MAX_LEN,
                    actual: notes.len(),
                });
            }
        }

        Ok(())
    }

    fn validate_uuids(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        // Validate folder_id format if present
        if let Some(folder_id) = &cipher.folder_id {
            if !folder_id.is_empty() && !self.is_valid_uuid(folder_id) {
                return Err(ValidationError::InvalidUuid {
                    field: "folderId".to_string(),
                    value: folder_id.clone(),
                });
            }
        }

        // Validate organization_id format if present
        if let Some(org_id) = &cipher.organization_id {
            if !org_id.is_empty() && !self.is_valid_uuid(org_id) {
                return Err(ValidationError::InvalidUuid {
                    field: "organizationId".to_string(),
                    value: org_id.clone(),
                });
            }
        }

        Ok(())
    }

    fn is_valid_uuid(&self, s: &str) -> bool {
        self.uuid_regex.is_match(s)
    }

    fn validate_totp_format(&self, totp: &str) -> Result<(), ValidationError> {
        if !totp.starts_with("otpauth://") {
            return Err(ValidationError::InvalidFormat {
                field: "totp".to_string(),
                expected: "otpauth:// URI".to_string(),
                actual: totp.to_string(),
            });
        }
        Ok(())
    }
}

impl Default for ValidationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::vault::{CipherLoginUriView, CipherLoginView};

    fn create_valid_login_cipher() -> CipherView {
        CipherView {
            id: "test-id".to_string(),
            organization_id: None,
            folder_id: None,
            cipher_type: CipherType::Login,
            name: "Test Login".to_string(),
            notes: None,
            favorite: false,
            collection_ids: vec![],
            revision_date: "2024-01-01T00:00:00Z".to_string(),
            creation_date: None,
            deleted_date: None,
            login: Some(CipherLoginView {
                username: Some("user@example.com".to_string()),
                password: Some("password123".to_string()),
                uris: vec![],
                totp: None,
            }),
            secure_note: None,
            card: None,
            identity: None,
            attachments: vec![],
            fields: vec![],
        }
    }

    #[test]
    fn test_validate_cipher_create_success() {
        let validator = ValidationService::new();
        let cipher = create_valid_login_cipher();

        let result = validator.validate_cipher_create(&cipher);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_cipher_missing_name() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.name = "".to_string();

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::MissingField(_))));
    }

    #[test]
    fn test_validate_cipher_name_too_long() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.name = "a".repeat(1001);

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::FieldTooLong { .. })));
    }

    #[test]
    fn test_validate_invalid_uuid() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.folder_id = Some("not-a-uuid".to_string());

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::InvalidUuid { .. })));
    }

    #[test]
    fn test_validate_folder_name_success() {
        let validator = ValidationService::new();
        let result = validator.validate_folder_name("My Folder");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_folder_name_empty() {
        let validator = ValidationService::new();
        let result = validator.validate_folder_name("");
        assert!(matches!(result, Err(ValidationError::EmptyField(_))));
    }

    #[test]
    fn test_validate_folder_name_too_long() {
        let validator = ValidationService::new();
        let result = validator.validate_folder_name(&"a".repeat(1001));
        assert!(matches!(result, Err(ValidationError::FieldTooLong { .. })));
    }

    #[test]
    fn test_validate_notes_too_long() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.notes = Some("a".repeat(10001));

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::FieldTooLong { .. })));
    }

    #[test]
    fn test_validate_totp_invalid_format() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.login = Some(CipherLoginView {
            username: Some("user@example.com".to_string()),
            password: Some("password123".to_string()),
            uris: vec![],
            totp: Some("invalid_totp".to_string()),
        });

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::InvalidFormat { .. })));
    }

    #[test]
    fn test_validate_totp_valid_format() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.login = Some(CipherLoginView {
            username: Some("user@example.com".to_string()),
            password: Some("password123".to_string()),
            uris: vec![],
            totp: Some(
                "otpauth://totp/Example:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example"
                    .to_string(),
            ),
        });

        let result = validator.validate_cipher_create(&cipher);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_uri_too_long() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.login = Some(CipherLoginView {
            username: Some("user@example.com".to_string()),
            password: Some("password123".to_string()),
            uris: vec![CipherLoginUriView {
                uri: Some("a".repeat(10001)),
                match_type: None,
            }],
            totp: None,
        });

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::FieldTooLong { .. })));
    }

    #[test]
    fn test_validate_valid_uuid() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.folder_id = Some("550e8400-e29b-41d4-a716-446655440000".to_string());

        let result = validator.validate_cipher_create(&cipher);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_invalid_organization_uuid() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.organization_id = Some("not-a-valid-uuid".to_string());

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::InvalidUuid { .. })));
    }

    #[test]
    fn test_validate_cipher_update_missing_id() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.id = "".to_string();

        let result = validator.validate_cipher_update(&cipher);
        assert!(matches!(result, Err(ValidationError::MissingField(_))));
    }

    #[test]
    fn test_validate_cipher_update_with_id_success() {
        let validator = ValidationService::new();
        let cipher = create_valid_login_cipher();

        let result = validator.validate_cipher_update(&cipher);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_secure_note_type_mismatch() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.cipher_type = CipherType::SecureNote;
        cipher.login = None;
        cipher.secure_note = None; // Missing secure_note data

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::TypeMismatch { .. })));
    }

    #[test]
    fn test_validate_card_type_mismatch() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.cipher_type = CipherType::Card;
        cipher.card = None; // Missing card data

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::TypeMismatch { .. })));
    }

    #[test]
    fn test_validate_identity_type_mismatch() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.cipher_type = CipherType::Identity;
        cipher.identity = None; // Missing identity data

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::TypeMismatch { .. })));
    }
}
