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
use bitwarden_core::client::persisted_state::{
    ACCOUNT_CRYPTO_STATE, BASE_URLS, BaseUrls, USER_EMAIL, USER_ID,
};
use bitwarden_core::key_management::account_cryptographic_state::WrappedAccountCryptographicState;
use bitwarden_core::key_management::crypto::{InitUserCryptoMethod, InitUserCryptoRequest};
use bitwarden_crypto::{EncString, Kdf, SymmetricCryptoKey};
use bitwarden_unlock::{SessionKey, UnlockClientExt, UnlockMethod};
use bitwarden_core::UserId;
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

    client
        .crypto()
        .initialize_user_crypto(InitUserCryptoRequest {
            user_id: Some(user_id),
            kdf_params: kdf,
            email: email.to_string(),
            account_cryptographic_state: WrappedAccountCryptographicState::V1 { private_key },
            method: InitUserCryptoMethod::DecryptedKey {
                decrypted_user_key: user_key.to_base64().to_string(),
            },
            upgrade_token: None,
        })
        .await
        .context("could not initialize the vault's encryption keys")
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
        .context("could not unlock the vault with the provided session key")
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
