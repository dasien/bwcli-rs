//! One-time carry-over of a `data.json` login into the SDK's state database.
//!
//! Everything the CLI needs to stay logged in — identity, server URLs, tokens,
//! account cryptographic state — used to live in `data.json`. It now lives in
//! `user.sqlite`, which means an install that predates that move looks logged
//! out. This closes that gap so upgrading does not force a re-login.
//!
//! Three properties this deliberately holds to:
//!
//! - **Non-destructive.** Nothing here removes or overwrites an existing
//!   `data.json` value. It does *add* two: the active account id and the
//!   account's email, when those are missing. `data.json` still namespaces sends,
//!   collections, organizations and the last-sync time by user, so leaving it
//!   with no active account would migrate the login and then have eight call
//!   sites unable to find it.
//! - **Never overwrites.** Each setting is written only when the SDK does not
//!   already have it, so running against already-migrated state is a no-op. That
//!   is what makes it safe to call on every startup rather than needing a flag.
//! - **Best-effort, never fatal.** A `data.json` we cannot make sense of leaves
//!   the user exactly where they were: logged out, needing `bw login`. That is
//!   strictly better than refusing to start.
//!
//! The vault itself is *not* carried over — `bw sync` rebuilds it from the
//! server, which avoids inheriting the shape mismatches that made the two
//! on-disk formats incompatible in the first place. The session cannot carry
//! over either: `BW_SESSION` seals a key into state we are only now populating,
//! so the user runs `bw unlock` once.

use crate::services::sdk_session::{self, CLI_CLIENT_ID};
use crate::services::storage::{AccountManager, JsonFileStorage, Storage, StorageKey};
use anyhow::{Context, Result};
use bitwarden_core::Client;
use bitwarden_core::client::login_method::UserLoginMethod;
use bitwarden_core::client::persisted_state::{
    ACCOUNT_CRYPTO_STATE, BASE_URLS, BaseUrls, USER_EMAIL, USER_ID,
};
use bitwarden_core::key_management::account_cryptographic_state::WrappedAccountCryptographicState;
use bitwarden_crypto::{EncString, Kdf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// What a carry-over managed to find, for logging and for tests.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Imported {
    pub identity: bool,
    pub base_urls: bool,
    pub crypto_state: bool,
    pub login_method: bool,
    pub tokens: bool,
}

impl Imported {
    fn any(&self) -> bool {
        self.identity || self.base_urls || self.crypto_state || self.login_method || self.tokens
    }
}

/// Carry a pre-SQLite login over, if there is one and it is needed.
///
/// Returns what was carried over. Errors are logged, not propagated: this runs
/// during startup for every command, and a failure here must not stop `bw
/// --help` from working.
pub async fn migrate_if_needed(client: &Client, storage: &Arc<Mutex<JsonFileStorage>>) {
    // The tokens are the thing that makes a login usable, so their presence is
    // the signal that there is nothing to do. Checking this first keeps the
    // common case down to one settings read.
    if sdk_session::is_authenticated(client).await {
        return;
    }

    match migrate(client, storage).await {
        Ok(imported) if imported.any() => {
            tracing::info!(
                "Carried a previous login over from data.json ({imported:?}); \
                 run 'bw sync' and 'bw unlock' to finish"
            );
        }
        Ok(_) => tracing::debug!("Nothing in data.json to carry over"),
        Err(e) => tracing::debug!("Could not carry a login over from data.json: {e:#}"),
    }
}

/// The carry-over itself, split out so tests can assert on the outcome.
pub async fn migrate(
    client: &Client,
    storage: &Arc<Mutex<JsonFileStorage>>,
) -> Result<Imported> {
    let state = client.platform().state();

    // If SDK state already names a user, that is the account this install is
    // for, and it decides which `data.json` account to read. Letting
    // `activeAccountId` decide instead could pair one account's tokens with
    // another's cryptographic state — a mix-up worse than not migrating.
    let known_user = state
        .setting(USER_ID)
        .context("no user_id setting")?
        .get()
        .await
        .ok()
        .flatten()
        .map(|id| id.to_string());

    let legacy = LegacyState::read(storage, known_user.as_deref())
        .await?
        .context("no account in data.json to carry over")?;

    let mut imported = Imported::default();

    // Email can come from either store; whichever has it, both should.
    let email = match &legacy.email {
        Some(email) => Some(email.clone()),
        None => state
            .setting(USER_EMAIL)
            .context("no user_email setting")?
            .get()
            .await
            .ok()
            .flatten(),
    };

    // Identity. `USER_ID` is load-bearing well beyond this module: unlocking
    // reads it back to set the client's user id, without which encryption fails.
    if state
        .setting(USER_ID)
        .context("no user_id setting")?
        .get()
        .await
        .ok()
        .flatten()
        .is_none()
    {
        let user_id = legacy
            .user_id
            .parse()
            .map_err(|_| anyhow::anyhow!("'{}' is not a valid user id", legacy.user_id))?;
        state
            .setting(USER_ID)
            .context("no user_id setting")?
            .update(user_id)
            .await
            .context("could not carry the user id over")?;

        if let Some(email) = &legacy.email {
            state
                .setting(USER_EMAIL)
                .context("no user_email setting")?
                .update(email.clone())
                .await
                .context("could not carry the email over")?;
        }
        imported.identity = true;
    }

    // Server URLs. Absent from `data.json` for a cloud account, in which case
    // there is nothing to carry and the client's own defaults are already right.
    if let Some(urls) = legacy.base_urls
        && state
            .setting(BASE_URLS)
            .context("no base_urls setting")?
            .get()
            .await
            .ok()
            .flatten()
            .is_none()
    {
        state
            .setting(BASE_URLS)
            .context("no base_urls setting")?
            .update(urls)
            .await
            .context("could not carry the server URLs over")?;
        imported.base_urls = true;
    }

    // Account cryptographic state: the account private key, wrapped by the user
    // key. Unlocking needs it, so a carry-over without it gets the user as far
    // as `bw sync` and no further.
    if let Some(private_key) = legacy.private_key
        && state
            .setting(ACCOUNT_CRYPTO_STATE)
            .context("no account_crypto_state setting")?
            .get()
            .await
            .ok()
            .flatten()
            .is_none()
    {
        state
            .setting(ACCOUNT_CRYPTO_STATE)
            .context("no account_crypto_state setting")?
            .update(WrappedAccountCryptographicState::V1 { private_key })
            .await
            .context("could not carry the account cryptographic state over")?;
        imported.crypto_state = true;
    }

    // Tokens, and the login method renewal needs alongside them. Written last:
    // `is_authenticated` keys off the tokens, so writing them only after
    // everything else means a crash midway through leaves the migration to be
    // retried rather than half-done and considered complete.
    if let Some(tokens) = legacy.tokens {
        let kdf = legacy.kdf.unwrap_or_else(Kdf::default_pbkdf2);
        sdk_session::persist_tokens(
            client,
            UserLoginMethod::Username {
                client_id: CLI_CLIENT_ID.to_string(),
                email: email.clone().unwrap_or_default(),
                kdf,
            },
            tokens.access_token.as_deref().unwrap_or_default(),
            tokens.refresh_token.as_deref(),
            // Deliberately expired: the access token in `data.json` is almost
            // certainly stale, and an old install may not have stored one at
            // all. Recording it as expired makes the first authenticated request
            // renew from the refresh token instead of sending something the
            // server will reject.
            0,
        )
        .await?;
        imported.login_method = true;
        imported.tokens = true;
    }

    // Additive repair of `data.json`, so the rest of the CLI can find the
    // account it just migrated. `bw logout` nulls `activeAccountId`, and an
    // account can sit in the registry with a blank email.
    if imported.tokens {
        adopt_active_account(storage, &legacy.user_id, email.as_deref()).await?;
    }

    Ok(imported)
}

/// Point `data.json` at the migrated account, without disturbing anything set.
async fn adopt_active_account(
    storage: &Arc<Mutex<JsonFileStorage>>,
    user_id: &str,
    email: Option<&str>,
) -> Result<()> {
    let accounts = AccountManager::new(Arc::clone(storage));

    // Fill in the email only when the registry has none. `register_account`
    // replaces the whole entry, so calling it with what we happen to know would
    // otherwise blank out an email — or a name — that was already there.
    let existing = accounts.get_account(user_id).await?;
    if let Some(email) = email.filter(|e| !e.is_empty())
        && existing.is_none_or(|a| a.email.is_empty())
    {
        accounts
            .register_account(user_id, email)
            .await
            .context("could not register the migrated account")?;
    }

    if accounts.get_active_user_id().await?.is_none() {
        accounts
            .set_active_user_id(user_id)
            .await
            .context("could not mark the migrated account active")?;
    }

    Ok(())
}

/// The `data.json` fields worth carrying over, already parsed.
struct LegacyState {
    user_id: String,
    email: Option<String>,
    base_urls: Option<BaseUrls>,
    private_key: Option<EncString>,
    kdf: Option<Kdf>,
    tokens: Option<LegacyTokens>,
}

struct LegacyTokens {
    access_token: Option<String>,
    refresh_token: Option<String>,
}

impl LegacyState {
    async fn read(
        storage: &Arc<Mutex<JsonFileStorage>>,
        known_user: Option<&str>,
    ) -> Result<Option<Self>> {
        let storage = storage.lock().await;

        let Some(user_id) = resolve_user(&storage, known_user)? else {
            return Ok(None);
        };

        let access_token = non_empty(&storage, StorageKey::UserAccessToken.format(Some(&user_id)))?;
        let refresh_token =
            non_empty(&storage, StorageKey::UserRefreshToken.format(Some(&user_id)))?;
        let tokens = (access_token.is_some() || refresh_token.is_some()).then_some(LegacyTokens {
            access_token,
            refresh_token,
        });

        let email = storage
            .get::<serde_json::Value>(&StorageKey::GlobalAccounts.format(None))?
            .and_then(|accounts| {
                accounts
                    .get(&user_id)?
                    .get("email")?
                    .as_str()
                    .filter(|e| !e.is_empty())
                    .map(str::to_string)
            });

        // A private key that is present but unparseable is worth a warning: the
        // carry-over still gets the user as far as `bw sync`, but `bw unlock`
        // would then fail with nothing pointing at the cause.
        let private_key = non_empty(&storage, StorageKey::UserPrivateKey.format(Some(&user_id)))?
            .and_then(|k| match k.parse::<EncString>() {
                Ok(key) => Some(key),
                Err(e) => {
                    tracing::warn!(
                        "The account private key in data.json could not be read ({e}); \
                         carrying the login over without it. 'bw unlock' will need a \
                         fresh 'bw login'."
                    );
                    None
                }
            });

        let kdf = storage
            .get::<crate::models::state::KdfConfig>(
                &StorageKey::UserKdfConfig.format(Some(&user_id)),
            )
            .ok()
            .flatten()
            .and_then(|c| Kdf::try_from(&c).ok());

        Ok(Some(Self {
            user_id,
            email,
            base_urls: read_base_urls(&storage)?,
            private_key,
            kdf,
            tokens,
        }))
    }
}

/// Which `data.json` account to carry over, most authoritative first.
///
/// 1. The user SDK state already names. Anything else risks pairing one
///    account's tokens with another's cryptographic state.
/// 2. `activeAccountId`.
/// 3. The sole account holding a token. An install whose `activeAccountId` was
///    cleared (`bw logout` nulls it) still has a recoverable login, and refusing
///    to look would be needlessly strict.
///
/// With several candidates and nothing to break the tie there is no safe answer,
/// so give up rather than guess which account the user meant.
fn resolve_user(storage: &JsonFileStorage, known_user: Option<&str>) -> Result<Option<String>> {
    if let Some(known) = known_user {
        return Ok(has_token(storage, known).then(|| known.to_string()));
    }

    if let Some(serde_json::Value::String(id)) =
        storage.get::<serde_json::Value>(&StorageKey::GlobalActiveAccountId.format(None))?
        && !id.is_empty()
    {
        return Ok(Some(id));
    }

    let Some(accounts) =
        storage.get::<serde_json::Value>(&StorageKey::GlobalAccounts.format(None))?
    else {
        return Ok(None);
    };
    let Some(accounts) = accounts.as_object() else {
        return Ok(None);
    };

    let mut candidates = accounts.keys().filter(|id| has_token(storage, id));

    match (candidates.next(), candidates.next()) {
        (Some(only), None) => Ok(Some(only.clone())),
        (Some(_), Some(_)) => {
            tracing::debug!(
                "data.json holds tokens for more than one account and no active \
                 account; not guessing which to carry over"
            );
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// Whether `data.json` holds any usable token for this account.
fn has_token(storage: &JsonFileStorage, user_id: &str) -> bool {
    [StorageKey::UserAccessToken, StorageKey::UserRefreshToken]
        .iter()
        .any(|k| {
            non_empty(storage, k.format(Some(user_id)))
                .ok()
                .flatten()
                .is_some()
        })
}

/// A string value, treating JSON `null` and `""` as absent.
///
/// Both appear in practice: `bw logout` nulls the token keys rather than
/// removing them, and the TypeScript CLI writes `null` for fields it keeps only
/// in memory.
fn non_empty(storage: &JsonFileStorage, key: String) -> Result<Option<String>> {
    Ok(storage
        .get::<serde_json::Value>(&key)?
        .and_then(|v| v.as_str().map(str::to_string))
        .filter(|s| !s.is_empty()))
}

/// Self-hosted server URLs, if this account has any.
fn read_base_urls(storage: &JsonFileStorage) -> Result<Option<BaseUrls>> {
    let Some(env) = storage.get::<serde_json::Value>(&StorageKey::GlobalConfigByServer.format(None))?
    else {
        return Ok(None);
    };

    // Shape: { "<region or url>": { "environment": { "api": ..., "identity": ... } } }
    let Some(servers) = env.as_object() else {
        return Ok(None);
    };

    let urls = servers.values().find_map(|entry| {
        let e = entry.get("environment")?;
        let api = e.get("api")?.as_str()?;
        let identity = e.get("identity")?.as_str()?;
        (!api.is_empty() && !identity.is_empty()).then(|| BaseUrls {
            api_url: api.to_string(),
            identity_url: identity.to_string(),
        })
    });

    Ok(urls)
}
