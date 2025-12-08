use serde::{Deserialize, Serialize};

/// Two-factor authentication data
#[derive(Debug, Clone)]
pub struct TwoFactorData {
    /// 2FA code/token
    pub token: String,
    /// 2FA provider code
    pub provider: u8,
    /// Remember this device for 2FA
    pub remember: bool,
}

/// Two-factor authentication method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TwoFactorMethod {
    Authenticator = 0,
    Email = 1,
    Duo = 2,
    YubiKey = 3,
    U2F = 4,
    WebAuthn = 7,
}

impl TwoFactorMethod {
    /// Get user-facing display name for this method
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Authenticator => "Authenticator app",
            Self::Email => "Email",
            Self::Duo => "Duo",
            Self::YubiKey => "YubiKey",
            Self::U2F => "FIDO U2F",
            Self::WebAuthn => "FIDO2 WebAuthn",
        }
    }

    /// Convert to provider code for API requests
    pub fn to_provider_code(&self) -> u8 {
        *self as u8
    }

    /// Parse from provider code
    pub fn from_provider_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Authenticator),
            1 => Some(Self::Email),
            2 => Some(Self::Duo),
            3 => Some(Self::YubiKey),
            4 => Some(Self::U2F),
            7 => Some(Self::WebAuthn),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_factor_method_display_names() {
        assert_eq!(
            TwoFactorMethod::Authenticator.display_name(),
            "Authenticator app"
        );
        assert_eq!(TwoFactorMethod::Email.display_name(), "Email");
    }

    #[test]
    fn test_two_factor_method_provider_codes() {
        assert_eq!(TwoFactorMethod::Authenticator.to_provider_code(), 0);
        assert_eq!(TwoFactorMethod::Email.to_provider_code(), 1);
        assert_eq!(
            TwoFactorMethod::from_provider_code(0),
            Some(TwoFactorMethod::Authenticator)
        );
        assert_eq!(TwoFactorMethod::from_provider_code(99), None);
    }
}
