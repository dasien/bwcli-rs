use serde::{Deserialize, Serialize};

/// Token refresh request (OAuth2 refresh token flow)
#[derive(Debug, Serialize)]
pub struct TokenRefreshRequest {
    pub grant_type: String, // Always "refresh_token"
    pub refresh_token: String,
}

/// Token response from authentication/refresh endpoints
///
/// Note: OAuth2 fields use snake_case (access_token, expires_in, etc.)
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub token_type: String,
    pub refresh_token: Option<String>,
    #[serde(rename = "Key")]
    pub key: Option<String>,
}
