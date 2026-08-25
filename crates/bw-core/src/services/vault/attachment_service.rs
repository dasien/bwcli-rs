//! Attachment service: upload, download and delete cipher attachments.
//!
//! Everything that can go through the SDK's [`bitwarden_vault::AttachmentsClient`]
//! does. Unlike `cipher_client`, `attachment_client` re-exports its types and
//! `VaultClient::attachments()` is public, so slot creation, download-URL lookup
//! and deletion are all callable from here.
//!
//! ## The one gap: uploading the bytes
//!
//! [`AttachmentsClient::create_attachment`] only opens a slot on the server — it
//! documents that "the caller must upload the encrypted bytes". The SDK has the
//! upload machinery (`upload_reencrypted`, used by `upgrade_attachment`) but it
//! is a private method, so we reimplement the same two transports here:
//!
//! - **Azure**: unauthenticated `PUT` to the presigned blob URL. The SAS token in
//!   the URL authorizes it, and a Bearer token must *not* be attached.
//! - **Direct**: authenticated multipart `POST` to
//!   `/ciphers/{id}/attachment/{attachmentId}` on the SDK's own middleware client,
//!   so it shares the SDK's token refresh and retry behaviour.
//!
//! The generated `ciphers_api::post_file_for_existing_attachment` is not usable
//! for this — it takes no body, so it uploads nothing.
//!
//! If the SDK ever makes its uploader public, [`Self::upload`] is what to delete.

use super::VaultError;
use bitwarden_core::Client;
use bitwarden_vault::{
    Attachment, AttachmentFileUploadType, AttachmentView, Cipher, CipherId, CipherView,
    CreateAttachmentRequest, CreatedAttachment, VaultClientExt,
};
use std::path::Path;
use std::sync::Arc;

/// An attachment downloaded and decrypted into memory.
pub struct DownloadedAttachment {
    /// The attachment's decrypted file name, as stored on the item.
    pub file_name: String,
    /// Decrypted contents.
    pub contents: Vec<u8>,
}

/// Service for attachment operations on vault items.
pub struct AttachmentService {
    sdk: Arc<Client>,
}

impl AttachmentService {
    pub fn new(sdk: Arc<Client>) -> Self {
        Self { sdk }
    }

    fn parse_cipher_id(id: &str) -> Result<CipherId, VaultError> {
        id.parse()
            .map_err(|_| VaultError::InvalidInput(format!("'{id}' is not a valid item id")))
    }

    /// The encrypted cipher as stored. Attachment crypto is defined against the
    /// encrypted cipher, not its view, because the attachment key is wrapped by
    /// the cipher key.
    async fn encrypted_cipher(&self, cipher_id: CipherId) -> Result<Cipher, VaultError> {
        self.sdk
            .platform()
            .state()
            .get::<Cipher>()
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .get(cipher_id)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::ItemNotFound)
    }

    /// Upload a file as a new attachment on `item_id`.
    ///
    /// The item is addressed by id only, matching the TypeScript CLI's
    /// `--itemid`, which does not accept a search term.
    pub async fn create(&self, item_id: &str, file_path: &Path) -> Result<CipherView, VaultError> {
        let cipher_id = Self::parse_cipher_id(item_id)?;

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                VaultError::InvalidInput(format!("'{}' does not name a file", file_path.display()))
            })?
            .to_string();

        let data = std::fs::read(file_path).map_err(|e| {
            VaultError::IoError(format!("cannot read {}: {e}", file_path.display()))
        })?;

        let cipher = self.encrypted_cipher(cipher_id).await?;
        let attachments = self.sdk.vault().attachments();

        // Only `file_name` is ours to set: `encrypt_buffer` generates the
        // attachment key and fills in size and size_name from the ciphertext.
        let view = AttachmentView {
            id: None,
            url: None,
            size: None,
            size_name: None,
            file_name: Some(file_name),
            key: None,
        };

        let encrypted = attachments
            .encrypt_buffer(cipher.clone(), view, &data)
            .map_err(|e| VaultError::EncryptionError(e.to_string()))?;

        let request = Self::slot_request(&encrypted.attachment, encrypted.contents.len(), &cipher)?;

        let created = attachments
            .create_attachment(cipher_id, request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // The slot exists on the server now. If the upload fails, the item would
        // otherwise keep an attachment entry pointing at nothing, so roll it back
        // — and report the upload failure, not the rollback's outcome.
        if let Err(upload_error) = self.upload(cipher_id, &created, encrypted.contents).await {
            if let Err(rollback_error) = attachments
                .delete_attachment(cipher_id, created.attachment_id.clone())
                .await
            {
                tracing::warn!(
                    "upload failed and the orphaned attachment slot {} on item {cipher_id} \
                     could not be removed: {rollback_error}",
                    created.attachment_id,
                );
            }
            return Err(upload_error);
        }

        // `create_attachment` has already written the server's cipher into the
        // repository, so this decrypts what a later read would see.
        self.sdk
            .vault()
            .ciphers()
            .decrypt(created.cipher)
            .await
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
    }

    /// Build the slot request from the encrypted attachment metadata.
    fn slot_request(
        attachment: &Attachment,
        encrypted_len: usize,
        cipher: &Cipher,
    ) -> Result<CreateAttachmentRequest, VaultError> {
        let missing =
            |what: &str| VaultError::EncryptionError(format!("encrypted attachment has no {what}"));

        Ok(CreateAttachmentRequest {
            key: attachment.key.clone().ok_or_else(|| missing("key"))?,
            file_name: attachment
                .file_name
                .clone()
                .ok_or_else(|| missing("file name"))?,
            file_size: encrypted_len as u64,
            // The server rejects the write if the item has moved on since we
            // read it, rather than silently attaching to a stale revision.
            last_known_revision_date: cipher.revision_date,
            as_admin: false,
        })
    }

    /// Upload the encrypted bytes to the slot the server just opened.
    ///
    /// See the module docs for why this is not the SDK's job here.
    async fn upload(
        &self,
        cipher_id: CipherId,
        created: &CreatedAttachment,
        contents: Vec<u8>,
    ) -> Result<(), VaultError> {
        match created.file_upload_type {
            AttachmentFileUploadType::Azure => {
                let response = reqwest::Client::new()
                    .put(&created.upload_url)
                    .header("x-ms-blob-type", "BlockBlob")
                    .body(contents)
                    .send()
                    .await
                    .map_err(|e| VaultError::ApiError(format!("attachment upload failed: {e}")))?;

                if !response.status().is_success() {
                    return Err(VaultError::ApiError(format!(
                        "attachment upload failed with status {}",
                        response.status()
                    )));
                }
            }
            AttachmentFileUploadType::Direct => {
                let api = self.sdk.internal.get_api_configurations();
                let url = format!(
                    "{}/ciphers/{}/attachment/{}",
                    api.api_config.base_path,
                    cipher_id,
                    bitwarden_api_base::urlencode(&created.attachment_id),
                );

                // Must be the SDK's reqwest version; see `reqwest_sdk` in the
                // workspace manifest.
                let form = reqwest_sdk::multipart::Form::new().part(
                    "data",
                    reqwest_sdk::multipart::Part::bytes(contents).file_name("data"),
                );

                let response = api
                    .api_config
                    .client
                    .post(url)
                    .with_extension(bitwarden_api_base::AuthRequired::Bearer)
                    .multipart(form)
                    .send()
                    .await
                    .map_err(|e| VaultError::ApiError(format!("attachment upload failed: {e}")))?;

                if !response.status().is_success() {
                    return Err(VaultError::ApiError(format!(
                        "attachment upload failed with status {}",
                        response.status()
                    )));
                }
            }
        }

        Ok(())
    }

    /// Download and decrypt an attachment.
    ///
    /// `id_or_name` matches the TypeScript CLI: an exact attachment id, or a
    /// substring of the file name. See [`find_attachment`].
    pub async fn download(
        &self,
        item_id: &str,
        id_or_name: &str,
    ) -> Result<DownloadedAttachment, VaultError> {
        let cipher_id = Self::parse_cipher_id(item_id)?;
        let cipher = self.encrypted_cipher(cipher_id).await?;

        let cipher_view = self
            .sdk
            .vault()
            .ciphers()
            .get(item_id)
            .await
            .map_err(|_| VaultError::ItemNotFound)?;

        let attachment = find_attachment(cipher_view.attachments.as_deref(), id_or_name)?;
        let attachment_id = attachment
            .id
            .clone()
            .ok_or_else(|| VaultError::InvalidInput("attachment has no id".to_string()))?;

        let attachments = self.sdk.vault().attachments();

        // No local fallback here on purpose: the SDK already retries a 404 against
        // the stored `attachment.url`, which is the same fallback the TypeScript
        // CLI implements by hand. Adding another would swallow auth and network
        // failures behind a stale URL.
        let url = attachments
            .get_attachment_download_url(cipher_id, attachment_id, None)
            .await
            .map_err(|e| {
                VaultError::ApiError(format!("no download url for this attachment: {e}"))
            })?;

        let response = reqwest::Client::new()
            .get(&url)
            .header("cache-control", "no-cache")
            .send()
            .await
            .map_err(|e| VaultError::ApiError(format!("attachment download failed: {e}")))?;

        if !response.status().is_success() {
            return Err(VaultError::ApiError(format!(
                "a {} error occurred while downloading the attachment",
                response.status().as_u16()
            )));
        }

        let encrypted = response
            .bytes()
            .await
            .map_err(|e| VaultError::ApiError(format!("attachment download failed: {e}")))?;

        let contents = attachments
            .decrypt_buffer(cipher, attachment.clone(), &encrypted)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))?;

        Ok(DownloadedAttachment {
            file_name: attachment
                .file_name
                .clone()
                .unwrap_or_else(|| "attachment".to_string()),
            contents,
        })
    }

    /// Delete an attachment from an item.
    ///
    /// Matches by attachment id only — the TypeScript CLI does not accept a file
    /// name here, unlike `get attachment`, because deleting the wrong attachment
    /// on a fuzzy match is not recoverable.
    pub async fn delete(&self, item_id: &str, attachment_id: &str) -> Result<(), VaultError> {
        let cipher_id = Self::parse_cipher_id(item_id)?;
        let cipher = self.encrypted_cipher(cipher_id).await?;

        let attachments = cipher.attachments.as_deref().unwrap_or_default();
        if attachments.is_empty() {
            return Err(VaultError::InvalidInput(
                "no attachments available for this item".to_string(),
            ));
        }

        let matched = attachments
            .iter()
            .find(|a| {
                a.id.as_deref()
                    .is_some_and(|id| id.eq_ignore_ascii_case(attachment_id))
            })
            .ok_or_else(|| {
                VaultError::InvalidInput(format!("attachment '{attachment_id}' was not found"))
            })?;

        let matched_id = matched
            .id
            .clone()
            .ok_or_else(|| VaultError::InvalidInput("attachment has no id".to_string()))?;

        self.sdk
            .vault()
            .attachments()
            .delete_attachment(cipher_id, matched_id)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        Ok(())
    }
}

/// Resolve `id_or_name` against an item's attachments.
///
/// Mirrors the TypeScript CLI's `getAttachment`: an id match or a file-name
/// substring match, both case-insensitive, with an exact file-name match winning
/// outright so `--output photo.jpg` is not ambiguous against `photo.jpg.bak`.
/// Anything still ambiguous is an error rather than an arbitrary pick — handing
/// back the wrong file silently is the failure worth avoiding.
fn find_attachment<'a>(
    attachments: Option<&'a [AttachmentView]>,
    id_or_name: &str,
) -> Result<&'a AttachmentView, VaultError> {
    let attachments = attachments.unwrap_or_default();
    if attachments.is_empty() {
        return Err(VaultError::InvalidInput(
            "no attachments available for this item".to_string(),
        ));
    }

    let needle = id_or_name.to_lowercase();

    let matches: Vec<&AttachmentView> = attachments
        .iter()
        .filter(|a| {
            let id_match =
                a.id.as_deref()
                    .is_some_and(|id| id.to_lowercase() == needle);
            let name_match = a
                .file_name
                .as_deref()
                .is_some_and(|n| n.to_lowercase().contains(&needle));
            id_match || name_match
        })
        .collect();

    if matches.is_empty() {
        return Err(VaultError::InvalidInput(format!(
            "attachment '{id_or_name}' was not found"
        )));
    }

    let exact: Vec<&AttachmentView> = matches
        .iter()
        .copied()
        .filter(|a| {
            a.file_name
                .as_deref()
                .is_some_and(|n| n.to_lowercase() == needle)
        })
        .collect();

    let candidates = if exact.len() == 1 { exact } else { matches };

    match candidates.len() {
        1 => Ok(candidates[0]),
        _ => Err(VaultError::MultipleItemsFound {
            search: id_or_name.to_string(),
            matches: candidates
                .iter()
                .map(|a| {
                    format!(
                        "{} ({})",
                        a.file_name.as_deref().unwrap_or("<unnamed>"),
                        a.id.as_deref().unwrap_or("<no id>")
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attachment(id: &str, file_name: &str) -> AttachmentView {
        AttachmentView {
            id: Some(id.to_string()),
            url: None,
            size: None,
            size_name: None,
            file_name: Some(file_name.to_string()),
            key: None,
        }
    }

    #[test]
    fn no_attachments_is_an_error() {
        let err = find_attachment(None, "anything").unwrap_err();
        assert!(err.to_string().contains("no attachments available"));

        let err = find_attachment(Some(&[]), "anything").unwrap_err();
        assert!(err.to_string().contains("no attachments available"));
    }

    #[test]
    fn matches_by_exact_id_case_insensitively() {
        let list = [attachment("ABC123", "photo.jpg")];
        let found = find_attachment(Some(&list), "abc123").unwrap();
        assert_eq!(found.id.as_deref(), Some("ABC123"));
    }

    #[test]
    fn matches_by_file_name_substring() {
        let list = [attachment("abc123", "holiday-photo.jpg")];
        let found = find_attachment(Some(&list), "HOLIDAY").unwrap();
        assert_eq!(found.id.as_deref(), Some("abc123"));
    }

    #[test]
    fn an_exact_file_name_beats_a_substring_match() {
        // Without the exact-match rule this is ambiguous, and `bw get attachment
        // photo.jpg` would refuse to do the obvious thing.
        let list = [
            attachment("aaa", "photo.jpg"),
            attachment("bbb", "photo.jpg.bak"),
        ];
        let found = find_attachment(Some(&list), "photo.jpg").unwrap();
        assert_eq!(found.id.as_deref(), Some("aaa"));
    }

    #[test]
    fn ambiguous_matches_are_reported_not_guessed() {
        let list = [
            attachment("aaa", "report-q1.pdf"),
            attachment("bbb", "report-q2.pdf"),
        ];
        let err = find_attachment(Some(&list), "report").unwrap_err();
        let message = err.to_string();
        assert!(message.contains("report-q1.pdf"), "{message}");
        assert!(message.contains("report-q2.pdf"), "{message}");
    }

    #[test]
    fn two_identical_file_names_stay_ambiguous() {
        // Two exact matches must not collapse to the first one.
        let list = [
            attachment("aaa", "photo.jpg"),
            attachment("bbb", "photo.jpg"),
        ];
        let err = find_attachment(Some(&list), "photo.jpg").unwrap_err();
        assert!(err.to_string().contains("aaa"));
        assert!(err.to_string().contains("bbb"));
    }

    #[test]
    fn a_missing_name_does_not_match_everything() {
        let mut nameless = attachment("aaa", "");
        nameless.file_name = None;
        let list = [nameless];
        let err = find_attachment(Some(&list), "photo").unwrap_err();
        assert!(err.to_string().contains("was not found"));
    }
}
