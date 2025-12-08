use crate::models::send::{SendFile, SendText};
use serde::{Deserialize, Serialize};

/// Send type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SendType {
    Text = 0,
    File = 1,
}

impl SendType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SendType::Text => "text",
            SendType::File => "file",
        }
    }
}

impl std::str::FromStr for SendType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" | "0" => Ok(SendType::Text),
            "file" | "1" => Ok(SendType::File),
            _ => Err(format!("Invalid send type: {}", s)),
        }
    }
}

/// Send object representing a temporary secure share
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Send {
    /// Send ID (UUID)
    pub id: String,

    /// Access ID used in public URL
    pub access_id: String,

    /// Send type: 0=Text, 1=File
    #[serde(rename = "type")]
    pub send_type: SendType,

    /// Encrypted name (EncString)
    pub name: String,

    /// Encrypted notes (EncString)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// Text Send data (if type=0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<SendText>,

    /// File Send data (if type=1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<SendFile>,

    /// Encrypted encryption key (EncString)
    pub key: String,

    /// Maximum access count (null = unlimited)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_access_count: Option<u32>,

    /// Current access count
    pub access_count: u32,

    /// Expiration date (ISO 8601, null = no expiration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,

    /// Deletion date (ISO 8601)
    pub deletion_date: String,

    /// Whether Send is disabled
    pub disabled: bool,

    /// Whether password is required for access
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// Revision date (ISO 8601)
    pub revision_date: String,

    /// Whether to hide email from recipient
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_email: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_type_from_str() {
        use std::str::FromStr;

        assert_eq!(SendType::from_str("text").unwrap(), SendType::Text);
        assert_eq!(SendType::from_str("Text").unwrap(), SendType::Text);
        assert_eq!(SendType::from_str("0").unwrap(), SendType::Text);

        assert_eq!(SendType::from_str("file").unwrap(), SendType::File);
        assert_eq!(SendType::from_str("File").unwrap(), SendType::File);
        assert_eq!(SendType::from_str("1").unwrap(), SendType::File);

        assert!(SendType::from_str("invalid").is_err());
    }
}
