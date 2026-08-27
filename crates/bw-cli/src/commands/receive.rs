use crate::AppContext;
use crate::GlobalArgs;
use crate::output::{CommandResult, Response};
use bitwarden_auth::AuthClientExt;
use bitwarden_auth::send_access::{
    SendAccessCredentials, SendAccessTokenRequest, SendPasswordCredentials,
};
use bitwarden_send::{SendAccessKey, SendClientExt};
use clap::Args;

#[derive(Args)]
pub struct ReceiveCommand {
    /// Send URL or access ID
    #[arg(value_name = "URL")]
    pub url: String,

    /// Password for password-protected Send
    #[arg(long)]
    pub password: Option<String>,
}

/// Split a Send URL into its access id and base64 key.
///
/// Accepts the full share URL, with or without scheme/host:
///   https://vault.bitwarden.com/#/send/<accessId>/<key>
///   #/send/<accessId>/<key>
///   /send/<accessId>/<key>
///
/// The key lives in the URL *fragment*, which is never sent to the server —
/// that is what keeps a Send end-to-end encrypted.
fn parse_send_url(url: &str) -> anyhow::Result<(String, String)> {
    let marker = "/send/";
    let rest = url
        .find(marker)
        .map(|i| &url[i + marker.len()..])
        .ok_or_else(|| {
            anyhow::anyhow!("'{url}' is not a Send URL (expected .../send/<accessId>/<key>)")
        })?;

    let mut parts = rest.trim_end_matches('/').splitn(2, '/');
    let access_id = parts.next().unwrap_or_default().trim();
    let key = parts.next().unwrap_or_default().trim();

    if access_id.is_empty() || key.is_empty() {
        anyhow::bail!("Send URL is missing the access id or the decryption key");
    }

    Ok((access_id.to_string(), key.to_string()))
}

pub async fn execute_receive(
    cmd: ReceiveCommand,
    _global_args: &GlobalArgs,
    ctx: &AppContext,
) -> CommandResult {
    // Receiving is anonymous: no login, no unlock, no session key.
    let (access_id, key_b64) = parse_send_url(&cmd.url)?;

    let access_key = SendAccessKey::from_url_b64(&key_b64)
        .map_err(|e| anyhow::anyhow!("Invalid Send key in URL: {e}"))?;

    // A password is hashed client-side with the send key; the plaintext never
    // leaves the machine.
    let credentials = cmd.password.as_deref().map(|password| {
        SendAccessCredentials::Password(SendPasswordCredentials {
            password_hash_b64: access_key.hash_password_b64(password),
        })
    });

    let token = ctx
        .sdk()
        .auth_new()
        .send_access()
        .request_send_access_token(SendAccessTokenRequest {
            send_id: access_id,
            send_access_credentials: credentials,
        })
        .await
        .map_err(|e| anyhow::anyhow!("Could not get access to this Send: {e}"))?;

    let response = ctx
        .sdk()
        .sends()
        .access_send(token.token)
        .await
        .map_err(|e| anyhow::anyhow!("Could not fetch this Send: {e}"))?;

    let view = access_key
        .decrypt_response(response)
        .map_err(|e| anyhow::anyhow!("Could not decrypt this Send: {e}"))?;

    if view.file.is_some() {
        eprintln!(
            "Note: this is a file Send. Downloading file Sends is not supported yet; \
             showing metadata only."
        );
    }

    Ok(Response::success(serde_json::to_value(view)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_full_share_url() {
        let (id, key) =
            parse_send_url("https://vault.bitwarden.com/#/send/abc123/AAECAwQFBgc").unwrap();
        assert_eq!(id, "abc123");
        assert_eq!(key, "AAECAwQFBgc");
    }

    #[test]
    fn parses_a_self_hosted_url() {
        let (id, key) =
            parse_send_url("https://vault.example.com/path/#/send/xyz/keydata").unwrap();
        assert_eq!(id, "xyz");
        assert_eq!(key, "keydata");
    }

    #[test]
    fn parses_a_bare_fragment() {
        let (id, key) = parse_send_url("#/send/abc/def").unwrap();
        assert_eq!(id, "abc");
        assert_eq!(key, "def");
    }

    #[test]
    fn tolerates_a_trailing_slash() {
        let (id, key) = parse_send_url("https://vault.bitwarden.com/#/send/abc/def/").unwrap();
        assert_eq!(id, "abc");
        assert_eq!(key, "def");
    }

    #[test]
    fn rejects_a_url_without_a_key() {
        assert!(parse_send_url("https://vault.bitwarden.com/#/send/abc").is_err());
    }

    #[test]
    fn rejects_a_non_send_url() {
        assert!(parse_send_url("https://example.com/something").is_err());
    }
}
