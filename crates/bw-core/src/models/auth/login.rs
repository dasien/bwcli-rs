/// Result of a successful login operation
#[derive(Debug, Clone)]
pub struct LoginResult {
    /// User ID (GUID)
    pub user_id: String,
    /// User email address
    pub email: String,
    /// Base64-encoded session key for BW_SESSION export
    pub session_key: String,
}

/// Result of a successful unlock operation
#[derive(Debug, Clone)]
pub struct UnlockResult {
    /// Base64-encoded session key for BW_SESSION export
    pub session_key: String,
}
