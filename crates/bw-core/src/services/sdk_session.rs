//! Session lifecycle, owned by the SDK.
//!
//! Replaces the CLI's hand-rolled `BW_SESSION` handling. `bitwarden-unlock`
//! implements exactly this design: a random session key that seals the user key
//! into persistent state, handed to the user as `BW_SESSION` and passed back on
//! the next invocation.
//!
//! Compared to what it replaces, unlocking through the SDK also restores
//! organization keys, which the hand-rolled path never did.
//!
//! Note the session key format is *not* the TypeScript CLI's 64-byte enc+MAC
//! blob — it is a `SymmetricKeyEnvelope`. Session keys are therefore not
//! interchangeable between the two CLIs, which is intended (see
//! `docs/sdk-3.0-migration.md`).

use anyhow::{Context, Result};
use bitwarden_core::Client;
use bitwarden_core::client::login_method::UserLoginMethod;
use bitwarden_core::client::persisted_state::{
    ACCOUNT_CRYPTO_STATE, AUTHENTICATION_TOKENS, AuthenticationTokens, BASE_URLS, BaseUrls,
    USER_EMAIL, USER_ID, USER_LOGIN_METHOD,
};
use bitwarden_core::key_management::account_cryptographic_state::WrappedAccountCryptographicState;
use bitwarden_core::key_management::crypto::{InitUserCryptoMethod, InitUserCryptoRequest};
use bitwarden_crypto::{EncString, Kdf, SymmetricCryptoKey};
use bitwarden_unlock::{SessionKey, UnlockClientExt, UnlockMethod};
use bitwarden_core::UserId;
use chrono::Utc;
use std::str::FromStr;

/// Load a freshly decrypted user key into the SDK's key store.
///
/// Used by login and password unlock, where we hold the plaintext user key.
/// Everything afterwards (minting a session key, vault crypto) depends on the
/// key store being populated.
pub async fn initialize_crypto(
    client: &Client,
    user_id: &str,
    email: &str,
    kdf: Kdf,
    private_key: EncString,
    user_key: &SymmetricCryptoKey,
) -> Result<()> {
    // Must be Some: the SDK reads the client's user id back when initializing
    // the local user data key, and a `None` here fails later with the opaque
    // "Unable to initialize local user data key".
    let user_id = UserId::from_str(user_id)
        .map_err(|_| anyhow::anyhow!("'{user_id}' is not a valid user id"))?;

    // `initialize_user_crypto` unconditionally overwrites USER_LOGIN_METHOD with
    // `UserLoginMethod::Username { client_id: "" }` (crypto.rs:403). That is
    // destructive for us in two ways: it blanks the `client_id` the token
    // handler sends on renewal, and for an API-key login it throws away the
    // client secret those tokens are re-minted from. Since `bw unlock` runs
    // through here, without this an unlock would quietly break token renewal.
    // So snapshot the login method and put it back.
    let previous = read_login_method(client).await;

    let result = client
        .crypto()
        .initialize_user_crypto(InitUserCryptoRequest {
            user_id: Some(user_id),
            kdf_params: kdf.clone(),
            email: email.to_string(),
            account_cryptographic_state: WrappedAccountCryptographicState::V1 { private_key },
            method: InitUserCryptoMethod::DecryptedKey {
                decrypted_user_key: user_key.to_base64().to_string(),
            },
            upgrade_token: None,
        })
        .await
        .context("could not initialize the vault's encryption keys");

    // Restore what we had, or — on a first login, where there was nothing to
    // snapshot — write a correct password login method rather than leaving the
    // SDK's blank one behind.
    let restored = previous.unwrap_or(UserLoginMethod::Username {
        client_id: CLI_CLIENT_ID.to_string(),
        email: email.to_string(),
        kdf,
    });
    if let Err(e) = write_login_method(client, restored).await {
        tracing::warn!("Could not restore the login method after initializing crypto: {e:#}");
    }

    result
}

/// The `client_id` the CLI authenticates as, and the one renewal must send.
pub const CLI_CLIENT_ID: &str = "cli";

async fn read_login_method(client: &Client) -> Option<UserLoginMethod> {
    client
        .platform()
        .state()
        .setting(USER_LOGIN_METHOD)
        .ok()?
        .get()
        .await
        .ok()
        .flatten()
        // A blank client_id is the SDK's placeholder, not a real login method;
        // treat it as absent so it gets replaced rather than preserved.
        .filter(|m| !matches!(m, UserLoginMethod::Username { client_id, .. } if client_id.is_empty()))
}

async fn write_login_method(client: &Client, login_method: UserLoginMethod) -> Result<()> {
    client
        .platform()
        .state()
        .setting(USER_LOGIN_METHOD)
        .context("no user_login_method setting")?
        .update(login_method)
        .await
        .context("could not persist the login method")
}

/// Persist what a later `unlock` needs to rebuild this session.
///
/// `ACCOUNT_CRYPTO_STATE` is the load-bearing one: `UnlockClient::unlock` reads
/// it alongside the sealed user key, and unlocking fails without it.
pub async fn persist_account_state(
    client: &Client,
    user_id: &str,
    email: &str,
    api_url: &str,
    identity_url: &str,
    private_key: EncString,
) -> Result<()> {
    let state = client.platform().state();

    let id = UserId::from_str(user_id)
        .map_err(|_| anyhow::anyhow!("'{user_id}' is not a valid user id"))?;
    state
        .setting(USER_ID)
        .context("no user_id setting")?
        .update(id)
        .await
        .context("could not persist the user id")?;

    state
        .setting(USER_EMAIL)
        .context("no user_email setting")?
        .update(email.to_string())
        .await
        .context("could not persist the user email")?;

    state
        .setting(BASE_URLS)
        .context("no base_urls setting")?
        .update(BaseUrls {
            identity_url: identity_url.to_string(),
            api_url: api_url.to_string(),
        })
        .await
        .context("could not persist the server URLs")?;

    state
        .setting(ACCOUNT_CRYPTO_STATE)
        .context("no account_crypto_state setting")?
        .update(WrappedAccountCryptographicState::V1 { private_key })
        .await
        .context("could not persist the account cryptographic state")?;

    Ok(())
}

/// Hand the tokens from a successful login to the SDK.
///
/// `PasswordManagerTokenHandler` reads `AUTHENTICATION_TOKENS` on every
/// authenticated request and renews from it, so writing these two settings is
/// the whole of token management on our side — there is no refresh code left in
/// the CLI.
///
/// `USER_LOGIN_METHOD` is not optional: renewal needs the `client_id` to send to
/// the identity service, and for an API-key login it needs the credentials
/// themselves, because those tokens are re-minted rather than refreshed.
/// Without it, renewal fails with `NotAuthenticated` and the CLI stops working
/// about an hour after login.
///
/// The SDK's own `login_password` does this via `InternalClient::set_tokens`,
/// which is `pub(crate)`. We write the same settings directly, with a
/// `client_id` of `cli` rather than the `web` that method hardcodes.
pub async fn persist_tokens(
    client: &Client,
    login_method: UserLoginMethod,
    access_token: &str,
    refresh_token: Option<&str>,
    expires_in: u64,
) -> Result<()> {
    write_login_method(client, login_method).await?;

    client
        .platform()
        .state()
        .setting(AUTHENTICATION_TOKENS)
        .context("no authentication_tokens setting")?
        .update(AuthenticationTokens {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.map(str::to_string),
            expires_on: Utc::now().timestamp() + expires_in as i64,
        })
        .await
        .context("could not persist the authentication tokens")?;

    Ok(())
}

/// Whether a login has been persisted, i.e. whether authenticated calls can be
/// attempted at all.
///
/// Deliberately does not check expiry: the token handler renews an expired
/// access token transparently, so an expired one is still "logged in".
pub async fn is_authenticated(client: &Client) -> bool {
    let Ok(setting) = client.platform().state().setting(AUTHENTICATION_TOKENS) else {
        return false;
    };

    matches!(setting.get().await, Ok(Some(_)))
}

/// Forget the persisted tokens and login method, i.e. `bw logout`.
///
/// Clearing the login method matters as much as the tokens: leaving an API-key
/// login method behind would let the token handler mint fresh tokens from the
/// stored client secret after logout.
pub async fn clear_tokens(client: &Client) -> Result<()> {
    let state = client.platform().state();

    state
        .setting(AUTHENTICATION_TOKENS)
        .context("no authentication_tokens setting")?
        .delete()
        .await
        .context("could not clear the authentication tokens")?;

    state
        .setting(USER_LOGIN_METHOD)
        .context("no user_login_method setting")?
        .delete()
        .await
        .context("could not clear the login method")?;

    Ok(())
}

/// Mint a session key, sealing the current user key into state.
///
/// Requires the key store to be populated — call [`initialize_crypto`] first.
/// The returned string is what the user exports as `BW_SESSION`.
pub async fn mint_session_key(client: &Client) -> Result<String> {
    let key = client
        .unlock()
        .generate_session_key()
        .await
        .context("could not create a session key")?;

    Ok(key.to_string())
}

/// Restore the vault's encryption keys from a session key.
///
/// Also restores organization keys from persisted state.
pub async fn unlock_with_session(client: &Client, session: &str) -> Result<()> {
    let key = SessionKey::from_str(session)
        .map_err(|_| anyhow::anyhow!("BW_SESSION is not a valid session key"))?;

    client
        .unlock()
        .unlock(UnlockMethod::SessionKey(key))
        .await
        .context("could not unlock the vault with the provided session key")?;

    // `UnlockClient::unlock` restores the keys but does not set the client's
    // user id — only `initialize_user_crypto` and `load_from_state` do. Without
    // it, decryption works but *encryption* fails with "Client User Id has not
    // been set", because an EncryptionContext records who encrypted the item.
    // So creates and edits would break while reads looked fine.
    let user_id = client
        .platform()
        .state()
        .setting(USER_ID)
        .context("no user_id setting")?
        .get()
        .await
        .context("could not read the stored user id")?
        .ok_or_else(|| anyhow::anyhow!("no user id in state; run 'bw login' again"))?;

    // Already-set is fine: the same process may have initialized crypto directly.
    if let Err(e) = client.internal.init_user_id(user_id).await {
        tracing::debug!("User id was already set: {e}");
    }

    Ok(())
}

/// Invalidate the stored session key, i.e. `bw lock`.
///
/// Deletes the sealed user key, so existing `BW_SESSION` values stop working.
pub async fn invalidate_session(client: &Client) -> Result<()> {
    client
        .unlock()
        .invalidate_session_key()
        .await
        .context("could not invalidate the session key")
}
