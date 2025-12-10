//! Import parsers for various formats

pub mod bitwarden_csv;
pub mod bitwarden_json;
pub mod chrome;
pub mod lastpass;
pub mod onepassword;

/// Convert empty strings to None, non-empty to Some(String)
///
/// Commonly used pattern in CSV parsers where empty fields should be None.
pub fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}
