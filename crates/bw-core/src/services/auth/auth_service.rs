use crate::models::{
    api::{
        ApiKeyLoginRequest, LoginResponse, PasswordLoginRequest, PreloginRequest, PreloginResponse,
        ProfileResponse,
    },
    auth::{DeviceInfo, LoginResult, TwoFactorData, UnlockResult},
    state::{KdfConfig, KdfType, UserProfile},
};
use crate::services::{
    api::{ApiClient, BitwardenApiClient},
    auth::{errors::AuthError, session_manager::SessionManager},
    crypto,
    storage::{JsonFileStorage, Storage},
};
use anyhow::Result;
use bitwarden_crypto::{CryptoError, Kdf, MasterKey, SymmetricCryptoKey};
use secrecy::{ExposeSecret, Secret};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Authentication service
///
/// Handles all authentication flows including:
/// - Password-based login
/// - API key login
/// - Vault unlock
/// - Lock/logout operations
pub struct AuthService {
    storage: Arc<Mutex<JsonFileStorage>>,
    api_client: Arc<BitwardenApiClient>,
    session_manager: Arc<SessionManager>,
}

impl AuthService {
    /// Create new authentication service
    pub fn new(storage: Arc<Mutex<JsonFileStorage>>, api_client: Arc<BitwardenApiClient>) -> Self {
        let session_manager = Arc::new(SessionManager::new(Arc::clone(&storage)));

        Self {
            storage,
            api_client,
            session_manager,
        }
    }

    /// Login with email and password
    ///
    /// # Arguments
    /// * `email` - User email address
    /// * `password` - Master password
    /// * `two_factor` - Optional 2FA data (if 2FA is required)
    ///
    /// # Returns
    /// LoginResult with session key for BW_SESSION export
    pub async fn login_with_password(
        &self,
        email: &str,
        password: Secret<String>,
        two_factor: Option<TwoFactorData>,
    ) -> Result<LoginResult, AuthError> {
        info!("Starting password login for: {}", email);

        // Step 1: Get KDF configuration from server
        debug!("Fetching KDF configuration");
        let kdf_config = self.fetch_kdf_config(email).await?;

        // Step 2: Derive master key using KDF
        debug!("Deriving master key (this may take a few seconds)");
        let master_key = self
            .derive_master_key(&password, email, &kdf_config)
            .await?;

        // Step 3: Hash password for authentication
        debug!("Hashing password for authentication");
        let hashed_password = self.hash_password_for_auth(&password, &master_key).await?;

        // Step 4: Authenticate with server
        debug!("Authenticating with server");
        let device_info = self.get_device_info().await?;
        let login_response = self
            .authenticate_password(email, &hashed_password, &device_info, two_factor)
            .await?;

        // Step 5: Decrypt user key (if available)
        let _user_key = if let Some(ref encrypted_key) = login_response.key {
            debug!("Decrypting user key");
            Some(self.decrypt_user_key(encrypted_key, &master_key).await?)
        } else {
            warn!("No user key in login response (API key login?)");
            None
        };

        // Step 6: Fetch user profile
        debug!("Fetching user profile");
        let profile = self.fetch_profile(&login_response.access_token).await?;

        // Step 7: Generate session key
        debug!("Generating session key");
        let session_key = SessionManager::generate_session_key();
        let session_key_str = SessionManager::format_for_export(&session_key);

        // Step 8: Persist authentication state
        debug!("Persisting authentication state");
        self.persist_auth_state(
            &profile.id,
            &profile.email,
            &login_response.access_token,
            &login_response.refresh_token,
            login_response.key.as_deref(),
            &kdf_config,
        )
        .await?;

        info!("Login successful for: {}", email);

        Ok(LoginResult {
            user_id: profile.id,
            email: profile.email,
            session_key: session_key_str,
        })
    }

    /// Login with API key
    ///
    /// # Arguments
    /// * `client_id` - API key client ID (format: "user.{uuid}")
    /// * `client_secret` - API key secret
    ///
    /// # Returns
    /// LoginResult with session key for BW_SESSION export
    pub async fn login_with_api_key(
        &self,
        client_id: &str,
        client_secret: Secret<String>,
    ) -> Result<LoginResult, AuthError> {
        info!("Starting API key login");

        // Get device info
        let device_info = self.get_device_info().await?;

        // Build API key login request
        let request = ApiKeyLoginRequest {
            grant_type: "client_credentials".to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.expose_secret().to_string(),
            scope: "api".to_string(),
            device_type: device_info.device_type,
            device_name: device_info.device_name.clone(),
            device_identifier: device_info.device_identifier.to_string(),
        };

        // Authenticate with server (no Auth-Email header for API key login)
        let login_response: LoginResponse = self
            .api_client
            .post_form("/identity/connect/token", &request, None)
            .await
            .map_err(|e| AuthError::InvalidCredentials {
                message: format!("API key authentication failed: {}", e),
            })?;

        // Fetch user profile
        let profile = self.fetch_profile(&login_response.access_token).await?;

        // Generate session key
        let session_key = SessionManager::generate_session_key();
        let session_key_str = SessionManager::format_for_export(&session_key);

        // Note: API key login doesn't have user key or KDF config
        // Persist minimal authentication state
        self.persist_api_key_auth_state(
            &profile.id,
            &profile.email,
            &login_response.access_token,
            &login_response.refresh_token,
        )
        .await?;

        info!("API key login successful");

        Ok(LoginResult {
            user_id: profile.id,
            email: profile.email,
            session_key: session_key_str,
        })
    }

    /// Unlock vault with master password
    ///
    /// # Arguments
    /// * `password` - Master password
    ///
    /// # Returns
    /// UnlockResult with session key for BW_SESSION export
    pub async fn unlock(&self, password: Secret<String>) -> Result<UnlockResult, AuthError> {
        info!("Starting vault unlock");

        // Check if logged in
        let storage = self.storage.lock().await;
        let profile: Option<UserProfile> = storage.get("userProfile")?;

        if profile.is_none() {
            return Err(AuthError::NotLoggedIn);
        }

        let profile = profile.unwrap();
        let email = profile.email;

        // Load KDF configuration
        let kdf_config: KdfConfig =
            storage
                .get("kdfConfig")?
                .ok_or_else(|| AuthError::KdfError {
                    message: "KDF configuration not found in storage".to_string(),
                })?;

        // Load encrypted user key (stored unencrypted on disk - it's already encrypted by server)
        let encrypted_user_key: Option<String> = storage.get("userKey")?;

        drop(storage); // Release lock

        let encrypted_user_key =
            encrypted_user_key.ok_or_else(|| AuthError::CryptoOperationFailed {
                message: "User key not found in storage".to_string(),
            })?;

        // Derive master key
        debug!("Deriving master key for unlock");
        let master_key = self
            .derive_master_key(&password, &email, &kdf_config)
            .await?;

        // Try to decrypt user key (validates password)
        debug!("Decrypting user key");
        let _user_key = self
            .decrypt_user_key(&encrypted_user_key, &master_key)
            .await
            .map_err(|_| AuthError::InvalidPassword)?;

        // Generate new session key
        debug!("Generating session key");
        let session_key = SessionManager::generate_session_key();
        let session_key_str = SessionManager::format_for_export(&session_key);

        info!("Vault unlock successful");

        Ok(UnlockResult {
            session_key: session_key_str,
        })
    }

    /// Lock vault (clear session keys)
    pub async fn lock(&self) -> Result<(), AuthError> {
        info!("Locking vault");

        // Check if logged in
        let logged_in = self.session_manager.is_logged_in().await?;
        if !logged_in {
            return Err(AuthError::NotLoggedIn);
        }

        // Clear session key hint (actual BW_SESSION is user's responsibility)
        self.session_manager.clear_session_key().await?;

        info!("Vault locked");
        Ok(())
    }

    /// Logout (clear all authentication state)
    pub async fn logout(&self) -> Result<(), AuthError> {
        info!("Logging out");

        let mut storage = self.storage.lock().await;

        // Clear all auth-related data (tokens stored unencrypted)
        storage.remove("accessToken").await?;
        storage.remove("refreshToken").await?;
        storage.remove("userKey").await?;
        storage.remove("userProfile").await?;
        storage.remove("kdfConfig").await?;

        storage.flush().await?;

        // Clear session key hint
        drop(storage);
        self.session_manager.clear_session_key().await?;

        info!("Logout complete");
        Ok(())
    }

    // Internal helper methods

    /// Fetch KDF configuration from server
    async fn fetch_kdf_config(&self, email: &str) -> Result<KdfConfig, AuthError> {
        let request = PreloginRequest {
            email: email.to_string(),
        };

        let response: PreloginResponse = self
            .api_client
            .post("/identity/accounts/prelogin", &request)
            .await
            .map_err(|e| AuthError::KdfError {
                message: format!("Failed to fetch KDF config: {}", e),
            })?;

        debug!(
            "Prelogin response: kdf={}, iterations={}, memory={:?}, parallelism={:?}",
            response.kdf, response.kdf_iterations, response.kdf_memory, response.kdf_parallelism
        );

        Ok(KdfConfig {
            kdf: if response.kdf == 0 {
                KdfType::PBKDF2SHA256
            } else {
                KdfType::Argon2id
            },
            kdf_iterations: Some(response.kdf_iterations),
            kdf_memory: response.kdf_memory,
            kdf_parallelism: response.kdf_parallelism,
        })
    }

    /// Derive master key using KDF (SDK-backed)
    async fn derive_master_key(
        &self,
        password: &Secret<String>,
        email: &str,
        kdf_config: &KdfConfig,
    ) -> Result<MasterKey, AuthError> {
        // Convert CLI KdfConfig to SDK Kdf
        let kdf: Kdf = kdf_config
            .try_into()
            .map_err(|e: anyhow::Error| AuthError::KdfError {
                message: e.to_string(),
            })?;

        let password_str = password.expose_secret().clone();
        let email_clone = email.to_string();

        // Run KDF in blocking task (CPU-intensive)
        tokio::task::spawn_blocking(move || {
            crypto::derive_master_key(&password_str, &email_clone, &kdf)
        })
        .await
        .map_err(|e| AuthError::CryptoOperationFailed {
            message: format!("KDF task failed: {}", e),
        })?
        .map_err(|e: CryptoError| AuthError::CryptoOperationFailed {
            message: format!("Key derivation failed: {}", e),
        })
    }

    /// Hash password for authentication request (SDK-backed)
    async fn hash_password_for_auth(
        &self,
        password: &Secret<String>,
        master_key: &MasterKey,
    ) -> Result<String, AuthError> {
        let password_str = password.expose_secret().clone();

        // Clone values for the blocking task
        // Note: MasterKey doesn't implement Clone, so we need to work around this
        // by doing the operation synchronously since password hashing is fast
        Ok(crypto::hash_password_for_auth(master_key, &password_str))
    }

    /// Decrypt user key from encrypted key (SDK-backed)
    async fn decrypt_user_key(
        &self,
        encrypted_key: &str,
        master_key: &MasterKey,
    ) -> Result<SymmetricCryptoKey, AuthError> {
        // User key decryption is fast (just AES), so we can do it inline
        crypto::decrypt_user_key(master_key, encrypted_key).map_err(|e: CryptoError| e.into())
    }

    /// Authenticate with password
    async fn authenticate_password(
        &self,
        email: &str,
        hashed_password: &str,
        device_info: &DeviceInfo,
        two_factor: Option<TwoFactorData>,
    ) -> Result<LoginResponse, AuthError> {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

        let request = PasswordLoginRequest {
            grant_type: "password".to_string(),
            username: email.to_string(),
            password: hashed_password.to_string(),
            scope: "api offline_access".to_string(),
            client_id: "cli".to_string(),
            device_type: device_info.device_type,
            device_name: device_info.device_name.clone(),
            device_identifier: device_info.device_identifier.to_string(),
            two_factor_token: two_factor.as_ref().map(|tf| tf.token.clone()),
            two_factor_provider: two_factor.as_ref().map(|tf| tf.provider),
            two_factor_remember: two_factor
                .as_ref()
                .map(|tf| if tf.remember { 1 } else { 0 }),
        };

        // Required headers for password login:
        // - Auth-Email: base64url encoded email (no padding)
        // - Device-Type: device type as string (e.g., "7" for macOS)
        let auth_email = URL_SAFE_NO_PAD.encode(email.as_bytes());
        let device_type_str = device_info.device_type.to_string();
        let extra_headers = vec![
            ("Auth-Email", auth_email.clone()),
            ("Device-Type", device_type_str.clone()),
        ];

        // Debug: log what we're sending
        debug!(
            "Login request: email={}, password_hash={}, device_type={}, device_name={}, device_id={}, auth_email_header={}",
            email,
            hashed_password,
            device_info.device_type,
            device_info.device_name,
            device_info.device_identifier,
            auth_email
        );

        self.api_client
            .post_form("/identity/connect/token", &request, Some(extra_headers))
            .await
            .map_err(|e| {
                // TODO: Parse error response for 2FA requirement
                AuthError::InvalidCredentials {
                    message: format!("Authentication failed: {}", e),
                }
            })
    }

    /// Fetch user profile
    async fn fetch_profile(&self, access_token: &str) -> Result<ProfileResponse, AuthError> {
        // Use get_authenticated which takes the token directly,
        // avoiding the need to store it before the profile fetch
        let profile: ProfileResponse = self
            .api_client
            .get_authenticated("/api/accounts/profile", access_token)
            .await
            .map_err(|e| AuthError::Api(e.into()))?;

        Ok(profile)
    }

    /// Get or create device info
    async fn get_device_info(&self) -> Result<DeviceInfo, AuthError> {
        let device_id_str = self.session_manager.get_or_create_device_id().await?;
        let device_id = uuid::Uuid::parse_str(&device_id_str)
            .map_err(|e| AuthError::Other(format!("Invalid device ID: {}", e)))?;

        Ok(DeviceInfo::new(Some(device_id)))
    }

    /// Persist authentication state to storage
    ///
    /// Note: Tokens are stored unencrypted on disk during login, similar to the
    /// official TypeScript CLI when secure storage is not available.
    /// The BW_SESSION key is only used for encrypting the vault cache, not the tokens.
    async fn persist_auth_state(
        &self,
        user_id: &str,
        email: &str,
        access_token: &str,
        refresh_token: &str,
        encrypted_user_key: Option<&str>,
        kdf_config: &KdfConfig,
    ) -> Result<(), AuthError> {
        let mut storage = self.storage.lock().await;

        // Store tokens (unencrypted on disk, like official CLI without secure storage)
        // The official CLI has 3 modes: SecureStorage, Disk (unencrypted), Memory
        // We use Disk mode since we don't have platform secure storage integration yet
        storage
            .set("accessToken", &access_token.to_string())
            .await?;
        storage
            .set("refreshToken", &refresh_token.to_string())
            .await?;

        if let Some(key) = encrypted_user_key {
            // User key is already encrypted by the server with the master key
            storage.set("userKey", &key.to_string()).await?;
        }

        // Store profile and config (plain)
        let profile = UserProfile {
            id: user_id.to_string(),
            email: email.to_string(),
            name: None,
            email_verified: true,
            premium: false,
            security_stamp: None,
        };

        storage.set("userProfile", &profile).await?;
        storage.set("kdfConfig", &kdf_config).await?;

        storage.flush().await?;

        Ok(())
    }

    /// Persist API key authentication state (no KDF or user key)
    async fn persist_api_key_auth_state(
        &self,
        user_id: &str,
        email: &str,
        access_token: &str,
        refresh_token: &str,
    ) -> Result<(), AuthError> {
        let mut storage = self.storage.lock().await;

        // Store tokens (unencrypted on disk, like official CLI without secure storage)
        storage
            .set("accessToken", &access_token.to_string())
            .await?;
        storage
            .set("refreshToken", &refresh_token.to_string())
            .await?;

        // Store profile
        let profile = UserProfile {
            id: user_id.to_string(),
            email: email.to_string(),
            name: None,
            email_verified: true,
            premium: false,
            security_stamp: None,
        };

        storage.set("userProfile", &profile).await?;
        storage.flush().await?;

        Ok(())
    }
}
