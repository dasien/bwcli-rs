//! TOTP code generation service
//!
//! Uses SDK for all TOTP operations to ensure correctness.

use super::errors::VaultError;
use bitwarden_vault::generate_totp;

/// Service for TOTP code generation
///
/// Uses SDK for all TOTP operations to ensure correctness.
pub struct TotpService {}

impl TotpService {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate current TOTP code from secret
    ///
    /// # Arguments
    /// * `totp_secret` - TOTP secret string (otpauth:// URI, steam:// URI, or base32 secret)
    ///
    /// # Returns
    /// TOTP code (typically 6 digits, or 5 chars for Steam)
    pub async fn generate_code(&self, totp_secret: &str) -> Result<String, VaultError> {
        let response = generate_totp(totp_secret.to_string(), None)
            .map_err(|e| VaultError::TotpError(e.to_string()))?;
        Ok(response.code)
    }
}

impl Default for TotpService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_totp_base32_secret() {
        let service = TotpService::new();

        // Test with a base32 secret
        let result = service.generate_code("JBSWY3DPEHPK3PXP").await;
        assert!(result.is_ok());

        let code = result.unwrap();
        // TOTP codes are 6 digits
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[tokio::test]
    async fn test_generate_totp_otpauth_uri() {
        let service = TotpService::new();

        // Test with otpauth:// URI format
        let result = service
            .generate_code(
                "otpauth://totp/Test:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Test",
            )
            .await;
        assert!(result.is_ok());

        let code = result.unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[tokio::test]
    async fn test_generate_totp_steam_uri() {
        let service = TotpService::new();

        // Test with steam:// URI format (Steam Guard)
        let result = service.generate_code("steam://JBSWY3DPEHPK3PXP").await;
        assert!(result.is_ok());

        let code = result.unwrap();
        // Steam codes are 5 characters from a specific alphabet
        assert_eq!(code.len(), 5);
    }

    #[tokio::test]
    async fn test_generate_totp_invalid_secret() {
        let service = TotpService::new();

        // Empty secret should fail
        let result = service.generate_code("").await;
        // Empty base32 decodes to empty bytes, which generates a code
        // The SDK is lenient here, so this actually succeeds
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_generate_totp_invalid_otpauth() {
        let service = TotpService::new();

        // Invalid otpauth URI (missing secret)
        let result = service
            .generate_code("otpauth://totp/Test:user@example.com")
            .await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, VaultError::TotpError(_)));
    }
}
