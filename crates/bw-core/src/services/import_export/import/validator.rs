//! Import data validation

use crate::services::import_export::errors::{ImportError, ValidationError};
use crate::services::import_export::import::{ImportData, ImportItemType};

/// Validate import data structure
pub fn validate(data: &ImportData) -> Result<(), ImportError> {
    let mut errors = Vec::new();

    // Validate each item
    for (i, item) in data.items.iter().enumerate() {
        let line = i + 1;

        // Name required
        if item.name.trim().is_empty() {
            errors.push(ValidationError {
                line: Some(line),
                field: Some("name".to_string()),
                message: "Name is required".to_string(),
            });
        }

        // Type-specific validation
        match item.item_type {
            ImportItemType::Login => {
                validate_login(&item.login, line, &mut errors);
            }
            ImportItemType::Card => {
                validate_card(&item.card, line, &mut errors);
            }
            ImportItemType::Identity => {
                validate_identity(&item.identity, line, &mut errors);
            }
            ImportItemType::SecureNote => {
                // Secure notes have no additional validation
            }
        }
    }

    // Fail-fast if any errors
    if !errors.is_empty() {
        // Log errors to stderr
        eprintln!("❌ Validation failed with {} error(s):\n", errors.len());
        for error in &errors {
            if let Some(line) = error.line {
                eprint!("  Line {}: ", line);
            }
            if let Some(field) = &error.field {
                eprint!("{}: ", field);
            }
            eprintln!("{}", error.message);
        }
        eprintln!("\nNo items were imported. Please fix the errors and try again.");

        return Err(ImportError::ValidationError {
            error_count: errors.len(),
        });
    }

    Ok(())
}

fn validate_login(
    login: &Option<crate::services::import_export::import::ImportLogin>,
    line: usize,
    errors: &mut Vec<ValidationError>,
) {
    if let Some(login) = login {
        // At least username or password should be present
        if login.username.is_none() && login.password.is_none() {
            errors.push(ValidationError {
                line: Some(line),
                field: Some("login".to_string()),
                message: "Login must have username or password".to_string(),
            });
        }

        // Validate URIs if present
        for (i, uri) in login.uris.iter().enumerate() {
            if uri.trim().is_empty() {
                errors.push(ValidationError {
                    line: Some(line),
                    field: Some(format!("login.uri[{}]", i)),
                    message: "URI cannot be empty".to_string(),
                });
            }
        }
    }
}

fn validate_card(
    card: &Option<crate::services::import_export::import::ImportCard>,
    line: usize,
    errors: &mut Vec<ValidationError>,
) {
    if let Some(card) = card {
        // At least card number should be present
        if card.number.is_none() {
            errors.push(ValidationError {
                line: Some(line),
                field: Some("card.number".to_string()),
                message: "Card must have a number".to_string(),
            });
        }
    }
}

fn validate_identity(
    _identity: &Option<crate::services::import_export::import::ImportIdentity>,
    _line: usize,
    _errors: &mut Vec<ValidationError>,
) {
    // Identity items are flexible, no strict validation needed
}
