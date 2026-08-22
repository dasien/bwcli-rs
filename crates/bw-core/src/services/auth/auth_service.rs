use crate::models::{
    api::{
        ApiKeyLoginRequest, LoginResponse, PasswordLoginRequest, PreloginRequest, PreloginResponse,
    },
    auth::{DeviceInfo, LoginResult, TwoFactorData, UnlockResult},
    state::{KdfConfig, KdfType},
};
use crate::services::{
    api::{ApiClient, BitwardenApiClient, endpoints},
    auth::{errors::AuthError, session_manager::SessionManager},
    crypto,
    storage::{
        AccountManager, JsonFileStorage, Storage, StorageKey,
    },
};
use anyhow::Result;
use crate::services::sdk_session;
use bitwarden_core::Client;
use bitwarden_core::auth::JwtToken;
use bitwarden_core::client::login_method::UserLoginMethod;
use bitwarden_crypto::EncString;
use bitwarden_crypto::{CryptoError, Kdf, MasterKey, SymmetricCryptoKey};
use secrecy::{ExposeSecret, Secret};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Token lifetime to record, in seconds.
///
/// `expires_in` is defaulted rather than required, so that an informational
/// field cannot fail the whole login. If it *is* missing we record an already-
/// expired token, which makes the first authenticated request renew before
/// sending. That is the safe direction: inventing a lifetime we were not told
/// would mean sending a token the server has already rejected and relying on the
/// 401 retry instead.
fn expires_in(response: &LoginResponse) -> u64 {
    response.expires_in.max(0) as u64
}

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
    account_manager: Arc<AccountManager>,
    /// Needed because the session lifecycle now lives in the SDK
    /// (`bitwarden-unlock`) rather than in this service.
    sdk: Arc<Client>,
}

impl AuthService {
    /// Create new authentication service
    pub fn new(
        storage: Arc<Mutex<JsonFileStorage>>,
        api_client: Arc<BitwardenApiClient>,
        sdk: Arc<Client>,
    ) -> Self {
        let session_manager = Arc::new(SessionManager::new(Arc::clone(&storage)));
        let account_manager = Arc::new(AccountManager::new(Arc::clone(&storage)));

        Self {
            storage,
            api_client,
            session_manager,
            account_manager,
            sdk,
        }
    }

    /// Get a reference to the account manager
    pub fn account_manager(&self) -> &Arc<AccountManager> {
        &self.account_manager
    }

    /// Login with email and password
    ///
    /// # Arguments
    /// * `email` - User email address
    /// * `password` - Master password
    /// * `two_factor` - Optional 2FA data (if 2FA is required)
    /// * `new_device_otp` - Optional new device verification OTP (sent via email)
    ///
    /// # Returns
    /// LoginResult with session key for BW_SESSION export
    pub async fn login_with_password(
        &self,
        email: &str,
        password: Secret<String>,
        two_factor: Option<TwoFactorData>,
        new_device_otp: Option<String>,
    ) -> Result<LoginResult, AuthError> {
        info!("Starting password login");

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
            .authenticate_password(
                email,
                &hashed_password,
                &device_info,
                two_factor,
                new_device_otp,
            )
            .await?;

        // Step 5: Decrypt user key (if available)
        let user_key = if let Some(ref encrypted_key) = login_response.key {
            debug!("Decrypting user key");
            Some(self.decrypt_user_key(encrypted_key, &master_key).await?)
        } else {
            warn!("No user key in login response (API key login?)");
            None
        };

        // Step 6: Identify the user from the token's claims (no extra round trip)
        let (user_id, email) = Self::identify_user(&login_response.access_token)?;

        // Step 7: Hand the session to the SDK.
        //
        // `generate_session_key` seals the user key out of the key store, so the
        // key store has to be populated first. The SDK persists both the sealed
        // key and the account cryptographic state, which is what a later
        // `unlock` reads back.
        let session_key_str = match (&user_key, login_response.private_key.as_deref()) {
            (Some(uk), Some(private_key)) => {
                debug!("Initializing vault crypto and minting a session key");
                self.establish_session(&user_id, &email, &kdf_config, private_key, uk)
                    .await?
            }
            _ => {
                // No user key or no account keys: nothing to unlock with. The
                // user runs `bw unlock` to get a session.
                warn!("Login response carried no user key; vault stays locked");
                String::new()
            }
        };

        // Step 8: Hand the tokens to the SDK, which owns them from here on.
        let kdf: Kdf = (&kdf_config)
            .try_into()
            .map_err(|e: anyhow::Error| AuthError::KdfError {
                message: e.to_string(),
            })?;
        sdk_session::persist_tokens(
            &self.sdk,
            UserLoginMethod::Username {
                client_id: sdk_session::CLI_CLIENT_ID.to_string(),
                email: email.clone(),
                kdf,
            },
            &login_response.access_token,
            Some(&login_response.refresh_token),
            expires_in(&login_response),
        )
        .await
        .map_err(|e| AuthError::Other(format!("{e:#}")))?;

        // Step 9: Persist authentication state
        debug!("Persisting authentication state");
        self.persist_auth_state(
            &user_id,
            &email,
            login_response.key.as_deref(),
            login_response.private_key.as_deref(),
            &kdf_config,
        )
        .await?;

        info!("Login successful");

        Ok(LoginResult {
            user_id: user_id.clone(),
            email: email.clone(),
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
            .post_form(endpoints::identity::TOKEN, &request, None)
            .await
            .map_err(|e| AuthError::InvalidCredentials {
                message: format!("API key authentication failed: {}", e),
            })?;

        // Identify the user from the token's claims
        let (user_id, email) = Self::identify_user(&login_response.access_token)?;

        // API-key login returns no user key, so there is nothing to seal and no
        // session to mint. The user runs `bw unlock` with their master password
        // to get one. Previously a session key was handed out here that could
        // not unlock anything.
        let session_key_str = String::new();

        // The stored login method carries the API key itself, because these
        // tokens are re-minted from it rather than refreshed. The `kdf` is a
        // placeholder: renewal never reads it, and an API-key login is not told
        // the account's KDF parameters.
        sdk_session::persist_tokens(
            &self.sdk,
            UserLoginMethod::ApiKey {
                client_id: client_id.to_string(),
                client_secret: client_secret.expose_secret().to_string(),
                email: email.clone(),
                kdf: Kdf::default_pbkdf2(),
            },
            &login_response.access_token,
            Some(&login_response.refresh_token),
            expires_in(&login_response),
        )
        .await
        .map_err(|e| AuthError::Other(format!("{e:#}")))?;

        // Note: API key login doesn't have user key or KDF config
        // Persist minimal authentication state
        self.persist_api_key_auth_state(&user_id, &email).await?;

        info!("API key login successful");

        Ok(LoginResult {
            user_id: user_id.clone(),
            email: email.clone(),
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

        // Get active user ID
        let user_id = self
            .account_manager
            .get_active_user_id()
            .await?
            .ok_or(AuthError::NotLoggedIn)?;

        // Get account info for email
        let account = self
            .account_manager
            .get_account(&user_id)
            .await?
            .ok_or(AuthError::NotLoggedIn)?;

        let email = account.email;

        // Load KDF configuration using namespaced key
        let storage = self.storage.lock().await;
        let kdf_key = StorageKey::UserKdfConfig.format(Some(&user_id));
        let kdf_config: KdfConfig = storage.get(&kdf_key)?.ok_or_else(|| AuthError::KdfError {
            message: "KDF configuration not found in storage".to_string(),
        })?;

        // Load encrypted user key using namespaced key
        let user_key_key = StorageKey::UserKey.format(Some(&user_id));
        let encrypted_user_key: Option<String> = storage.get(&user_key_key)?;

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
        let user_key = self
            .decrypt_user_key(&encrypted_user_key, &master_key)
            .await
            .map_err(|_| AuthError::InvalidPassword)?;

        // Hand the session to the SDK, exactly as login does.
        let private_key = {
            let storage = self.storage.lock().await;
            storage
                .get::<String>(&StorageKey::UserPrivateKey.format(Some(&user_id)))?
                .ok_or_else(|| AuthError::CryptoOperationFailed {
                    message: "Account private key not found. Run 'bw login' again.".to_string(),
                })?
        };

        let session_key_str = self
            .establish_session(&user_id, &email, &kdf_config, &private_key, &user_key)
            .await?;

        info!("Vault unlock successful");

        Ok(UnlockResult {
            session_key: session_key_str,
        })
    }

    /// Lock vault (clear session keys and protected user key)
    pub async fn lock(&self) -> Result<(), AuthError> {
        info!("Locking vault");

        // Presence of an active account is still a precondition, but the id
        // itself is no longer needed: the sealed key lives in SDK state.
        self.account_manager
            .get_active_user_id()
            .await?
            .ok_or(AuthError::NotLoggedIn)?;

        // The SDK's token state is the only record of being logged in; tokens no
        // longer live in `data.json`.
        if !sdk_session::is_authenticated(&self.sdk).await {
            return Err(AuthError::NotLoggedIn);
        }

        // Invalidating the sealed user key is what actually locks the vault;
        // any outstanding BW_SESSION stops working.
        sdk_session::invalidate_session(&self.sdk)
            .await
            .map_err(|e| AuthError::Other(e.to_string()))?;

        info!("Vault locked");
        Ok(())
    }

    /// Logout (clear all authentication state)
    pub async fn logout(&self) -> Result<(), AuthError> {
        info!("Logging out");

        let user_id = self
            .account_manager
            .get_active_user_id()
            .await?
            .ok_or(AuthError::NotLoggedIn)?;

        // Tokens and the login method live in SDK state. Clearing the login
        // method matters as much as clearing the tokens: for an API-key login it
        // holds the client secret, which the token handler would happily use to
        // mint fresh tokens after logout.
        sdk_session::clear_tokens(&self.sdk)
            .await
            .map_err(|e| AuthError::Other(format!("{e:#}")))?;

        // Versions before tokens moved into SDK state wrote them to `data.json`.
        // Nothing reads those keys any more, but a logout should not leave a
        // usable refresh token behind, so scrub them for anyone upgrading.
        self.scrub_legacy_tokens(&user_id).await?;

        // The sealed user key lives in SDK state too, so invalidating the
        // session is what actually revokes vault access. Best-effort: the tokens
        // are already gone by this point.
        if let Err(e) = sdk_session::invalidate_session(&self.sdk).await {
            debug!("Could not invalidate the session key during logout: {e:#}");
        }

        // Clear active account (but preserve in accounts registry)
        self.account_manager.clear_active_account().await?;

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
            .post(endpoints::identity::PRELOGIN, &request)
            .await
            .map_err(|e| AuthError::KdfError {
                message: format!("Failed to fetch KDF config: {}", e),
            })?;

        debug!(
            "Prelogin response: kdf={}, iterations={}, memory={:?}, parallelism={:?}",
            response.kdf, response.kdf_iterations, response.kdf_memory, response.kdf_parallelism
        );

        Ok(KdfConfig {
            kdf_type: if response.kdf == 0 {
                KdfType::PBKDF2SHA256
            } else {
                KdfType::Argon2id
            },
            iterations: Some(response.kdf_iterations),
            memory: response.kdf_memory,
            parallelism: response.kdf_parallelism,
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
        new_device_otp: Option<String>,
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
            new_device_otp,
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

        // Debug: log non-sensitive request metadata
        debug!(
            "Login request: device_type={}, device_name={}, device_id={}",
            device_info.device_type, device_info.device_name, device_info.device_identifier,
        );

        self.api_client
            .post_form(endpoints::identity::TOKEN, &request, Some(extra_headers))
            .await
            .map_err(|e| {
                let error_str = e.to_string().to_lowercase();
                // Check for new device verification required
                if error_str.contains("new device verification required") {
                    return AuthError::NewDeviceVerificationRequired;
                }
                // TODO: Parse error response for 2FA requirement
                AuthError::InvalidCredentials {
                    message: format!("Authentication failed: {}", e),
                }
            })
    }

    /// Initialize vault crypto from a decrypted user key, persist what a later
    /// unlock needs, and mint the `BW_SESSION` value.
    async fn establish_session(
        &self,
        user_id: &str,
        email: &str,
        kdf_config: &KdfConfig,
        private_key: &str,
        user_key: &SymmetricCryptoKey,
    ) -> Result<String, AuthError> {
        let kdf: Kdf = kdf_config
            .try_into()
            .map_err(|e: anyhow::Error| AuthError::KdfError {
                message: e.to_string(),
            })?;

        let private_key: EncString =
            private_key
                .parse()
                .map_err(|_| AuthError::CryptoOperationFailed {
                    message: "Account private key is malformed".to_string(),
                })?;

        // `{:#}` keeps the whole context chain; plain Display would report only
        // the outermost message and hide the actual cause.
        let to_auth_err = |e: anyhow::Error| AuthError::CryptoOperationFailed {
            message: format!("{e:#}"),
        };

        sdk_session::initialize_crypto(
            &self.sdk,
            user_id,
            email,
            kdf,
            private_key.clone(),
            user_key,
        )
        .await
        .map_err(to_auth_err)?;

        sdk_session::persist_account_state(
            &self.sdk,
            user_id,
            email,
            self.api_client.environment().api_url(),
            self.api_client.environment().identity_url(),
            private_key,
        )
        .await
        .map_err(to_auth_err)?;

        sdk_session::mint_session_key(&self.sdk)
            .await
            .map_err(to_auth_err)
    }

    /// Identify the user from the access token's own claims.
    ///
    /// Replaces a `GET /accounts/profile` round trip: the id and email we need
    /// are already in the JWT we were just issued. That also retires the
    /// hand-rolled `ProfileResponse` model — a field rename there would have
    /// broken login exactly as `ForcePasswordReset` did.
    ///
    /// The SDK notes `JwtToken` does not verify the signature. That is fine
    /// here: the token came directly from the identity server over TLS, and we
    /// are reading our own identity from it, not making a trust decision.
    fn identify_user(access_token: &str) -> Result<(String, String), AuthError> {
        let token: JwtToken = access_token.parse().map_err(|e| AuthError::Other(format!(
            "the server returned an access token we could not read: {e}"
        )))?;

        let email = token.email.ok_or_else(|| {
            AuthError::Other("the access token did not identify the user's email".to_string())
        })?;

        Ok((token.sub, email))
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
    /// Uses TypeScript CLI compatible namespaced keys:
    /// - `stateVersion`: 73 (if not already set)
    /// - `global_account_accounts`: account registry
    /// - `global_account_activeAccountId`: currently active user
    /// - `user_{id}_crypto_userKey`: encrypted user key
    /// - `user_{id}_crypto_privateKey`: account private key (wrapped by the user key)
    /// - `user_{id}_kdf_config`: KDF configuration
    ///
    /// Tokens are deliberately absent: they belong to the SDK's state database
    /// now (see [`sdk_session::persist_tokens`]), and writing them here too
    /// would recreate the split token state that made refresh unreliable.
    async fn persist_auth_state(
        &self,
        user_id: &str,
        email: &str,
        encrypted_user_key: Option<&str>,
        private_key: Option<&str>,
        kdf_config: &KdfConfig,
    ) -> Result<(), AuthError> {
        let mut storage = self.storage.lock().await;

        // Ensure state version is set (for new storage files)
        storage.ensure_state_version().await?;

        // Register account in global accounts registry
        drop(storage); // Release lock for account_manager
        self.account_manager
            .register_account(user_id, email)
            .await?;

        // Set as active account
        self.account_manager.set_active_user_id(user_id).await?;

        // Re-acquire storage lock
        let mut storage = self.storage.lock().await;

        if let Some(key) = encrypted_user_key {
            // User key is already encrypted by the server with the master key
            storage
                .set(&StorageKey::UserKey.format(Some(user_id)), &key.to_string())
                .await?;
        }

        // The account private key is required to initialize SDK crypto on
        // subsequent invocations (InitUserCryptoRequest::account_cryptographic_state).
        if let Some(key) = private_key {
            storage
                .set(
                    &StorageKey::UserPrivateKey.format(Some(user_id)),
                    &key.to_string(),
                )
                .await?;
        }

        // Store KDF config with user-namespaced key
        storage
            .set(&StorageKey::UserKdfConfig.format(Some(user_id)), kdf_config)
            .await?;

        storage.flush().await?;

        Ok(())
    }

    /// Null out the `data.json` token keys written by pre-migration versions.
    ///
    /// Set to null rather than removed, matching what the TypeScript CLI does
    /// with these keys.
    async fn scrub_legacy_tokens(&self, user_id: &str) -> Result<(), AuthError> {
        let mut storage = self.storage.lock().await;

        for key in [StorageKey::UserAccessToken, StorageKey::UserRefreshToken] {
            storage
                .set(&key.format(Some(user_id)), &serde_json::Value::Null)
                .await?;
        }

        storage.flush().await?;
        Ok(())
    }

    /// Register the account for an API-key login.
    ///
    /// API-key login provides no KDF config and no user key, and the tokens are
    /// the SDK's, so all that is left here is the account registry.
    async fn persist_api_key_auth_state(
        &self,
        user_id: &str,
        email: &str,
    ) -> Result<(), AuthError> {
        {
            let mut storage = self.storage.lock().await;
            storage.ensure_state_version().await?;
            storage.flush().await?;
        }

        self.account_manager
            .register_account(user_id, email)
            .await?;
        self.account_manager.set_active_user_id(user_id).await?;

        Ok(())
    }
}

#[cfg(test)]
mod identity_tests {
    use super::AuthService;
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

    /// Build an unsigned JWT. `JwtToken` does not verify signatures, which is
    /// fine: the token arrives straight from the identity server over TLS and we
    /// only read our own identity out of it.
    fn jwt(payload: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
        let body = URL_SAFE_NO_PAD.encode(payload.as_bytes());
        format!("{header}.{body}.signature")
    }

    #[test]
    fn reads_user_id_and_email_from_the_token() {
        let token = jwt(
            r#"{"exp":1787419631,"sub":"73d54ce4-b9f5-4bf1-a921-b3ad0104a632","email":"user@example.com","scope":["api","offline_access"]}"#,
        );

        let (user_id, email) = AuthService::identify_user(&token).unwrap();

        assert_eq!(user_id, "73d54ce4-b9f5-4bf1-a921-b3ad0104a632");
        assert_eq!(email, "user@example.com");
    }

    #[test]
    fn rejects_a_token_without_an_email_claim() {
        let token = jwt(r#"{"exp":1787419631,"sub":"abc","scope":["api"]}"#);

        let err = AuthService::identify_user(&token).unwrap_err();
        assert!(err.to_string().contains("email"), "got: {err}");
    }

    #[test]
    fn rejects_a_malformed_token() {
        let err = AuthService::identify_user("not-a-jwt").unwrap_err();
        assert!(err.to_string().contains("could not read"), "got: {err}");
    }
}
