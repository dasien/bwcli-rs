use secrecy::Secret;
use serde::{Deserialize, Serialize};

/// Authentication state
///
/// Tokens are stored encrypted with __PROTECTED__ prefix
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthState {
    /// Access token (JWT) - ENCRYPTED
    /// Stored as: __PROTECTED__tokens.accessToken
    #[serde(skip)]
    pub access_token: Option<Secret<String>>,

    /// Refresh token - ENCRYPTED
    /// Stored as: __PROTECTED__tokens.refreshToken
    #[serde(skip)]
    pub refresh_token: Option<Secret<String>>,

    /// Token expiration timestamp (Unix seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_expiry: Option<i64>,

    /// User ID (GUID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}
