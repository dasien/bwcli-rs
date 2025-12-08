use serde::Deserialize;
use std::collections::HashMap;

/// Bitwarden API error response format
#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    #[serde(rename = "Message")]
    pub message: Option<String>,

    #[serde(rename = "ValidationErrors")]
    pub validation_errors: Option<HashMap<String, Vec<String>>>,

    #[serde(rename = "error")]
    pub error: Option<String>,

    #[serde(rename = "error_description")]
    pub error_description: Option<String>,
}
