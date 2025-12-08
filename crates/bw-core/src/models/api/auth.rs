use serde::{Deserialize, Serialize};

/// Prelogin request to get KDF parameters
#[derive(Debug, Serialize)]
pub struct PreloginRequest {
    pub email: String,
}

/// Prelogin response with KDF configuration
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreloginResponse {
    /// KDF type: 0 = PBKDF2-SHA256, 1 = Argon2id
    pub kdf: u8,
    /// PBKDF2 iterations (default: 600000)
    pub kdf_iterations: u32,
    /// Argon2id memory in MB (default: 64)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kdf_memory: Option<u32>,
    /// Argon2id parallelism (default: 4)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kdf_parallelism: Option<u32>,
}

/// Password login request (OAuth2 password grant)
///
/// NOTE: This must be form-encoded, not JSON
#[derive(Debug, Serialize)]
pub struct PasswordLoginRequest {
    /// OAuth2 grant type
    pub grant_type: String,
    /// Email address
    pub username: String,
    /// Base64 hashed password
    pub password: String,
    /// OAuth2 scope
    pub scope: String,
    /// Client ID ("cli")
    pub client_id: String,
    /// Device type code (8 = CLI)
    #[serde(rename = "deviceType")]
    pub device_type: u8,
    /// Device name
    #[serde(rename = "deviceName")]
    pub device_name: String,
    /// Device identifier (UUID)
    #[serde(rename = "deviceIdentifier")]
    pub device_identifier: String,

    // Optional 2FA fields
    #[serde(rename = "twoFactorToken", skip_serializing_if = "Option::is_none")]
    pub two_factor_token: Option<String>,
    #[serde(rename = "twoFactorProvider", skip_serializing_if = "Option::is_none")]
    pub two_factor_provider: Option<u8>,
    #[serde(rename = "twoFactorRemember", skip_serializing_if = "Option::is_none")]
    pub two_factor_remember: Option<u8>,
}

/// API key login request (OAuth2 client credentials grant)
///
/// NOTE: This must be form-encoded, not JSON
#[derive(Debug, Serialize)]
pub struct ApiKeyLoginRequest {
    /// OAuth2 grant type
    pub grant_type: String,
    /// Client ID (format: "user.{uuid}")
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// OAuth2 scope
    pub scope: String,
    /// Device type code (8 = CLI)
    #[serde(rename = "deviceType")]
    pub device_type: u8,
    /// Device name
    #[serde(rename = "deviceName")]
    pub device_name: String,
    /// Device identifier (UUID)
    #[serde(rename = "deviceIdentifier")]
    pub device_identifier: String,
}

/// Login response (both password and API key)
///
/// Note: OAuth2 fields use snake_case (access_token, expires_in, etc.)
/// Bitwarden-specific fields use PascalCase (Key, Kdf, etc.) with explicit renames
#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    /// Access token (JWT)
    pub access_token: String,
    /// Token expiry in seconds (typically 3600 = 1 hour)
    pub expires_in: i64,
    /// Token type ("Bearer")
    pub token_type: String,
    /// Refresh token
    pub refresh_token: String,

    /// Encrypted user key (EncString format)
    /// Note: Capital 'K' in response
    #[serde(rename = "Key")]
    pub key: Option<String>,

    /// Encrypted RSA private key
    #[serde(rename = "PrivateKey")]
    pub private_key: Option<String>,

    /// KDF type
    #[serde(rename = "Kdf")]
    pub kdf: u8,
    /// KDF iterations
    #[serde(rename = "KdfIterations")]
    pub kdf_iterations: u32,
    /// KDF memory (Argon2id)
    #[serde(rename = "KdfMemory")]
    pub kdf_memory: Option<u32>,
    /// KDF parallelism (Argon2id)
    #[serde(rename = "KdfParallelism")]
    pub kdf_parallelism: Option<u32>,

    /// Master password reset required
    #[serde(rename = "ResetMasterPassword")]
    pub reset_master_password: bool,

    /// Available 2FA providers (if 2FA required)
    #[serde(rename = "TwoFactorProviders")]
    pub two_factor_providers: Option<Vec<u8>>,
    /// 2FA provider details
    #[serde(rename = "TwoFactorProviders2")]
    pub two_factor_providers2: Option<serde_json::Value>,
}

/// User profile response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResponse {
    /// User ID (GUID)
    pub id: String,
    /// Email address
    pub email: String,
    /// Display name
    pub name: Option<String>,
    /// Email verification status
    pub email_verified: bool,
    /// Premium account status
    pub premium: bool,
    /// Security stamp for session invalidation
    pub security_stamp: String,
    /// Two-factor enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub two_factor_enabled: Option<bool>,
}
