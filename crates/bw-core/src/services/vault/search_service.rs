//! Search and filter service
//!
//! Provides efficient filtering without requiring full decryption.

use bitwarden_collections::collection::CollectionId;
use bitwarden_collections::collection::CollectionView;
use bitwarden_core::OrganizationId;
use bitwarden_vault::{Cipher, CipherId, CipherListView, CipherListViewType, FolderId, FolderView};
use std::collections::HashMap;

/// Item filter options for list operations
#[derive(Debug, Default, Clone)]
pub struct ItemFilters {
    pub organization_id: Option<String>,
    pub collection_id: Option<String>,
    pub folder_id: Option<String>,
    pub search: Option<String>,
    pub url: Option<String>,
    pub trash: bool,
}

/// Service for searching and filtering vault items
///
/// Provides efficient filtering without requiring full decryption.
pub struct SearchService;

impl SearchService {
    pub fn new() -> Self {
        Self
    }

    /// Filter ciphers based on criteria
    ///
    /// Returns filtered HashMap of encrypted ciphers (not decrypted yet).
    /// Filtering done on encrypted metadata (IDs, dates, structure).
    pub fn filter_ciphers(
        &self,
        ciphers: &HashMap<String, Cipher>,
        filters: &ItemFilters,
    ) -> HashMap<String, Cipher> {
        ciphers
            .iter()
            .filter(|(_, cipher)| {
                // Trash filter (exclude deleted by default)
                if filters.trash {
                    if cipher.deleted_date.is_none() {
                        return false;
                    }
                } else if cipher.deleted_date.is_some() {
                    return false;
                }

                // Organization filter
                if let Some(org_id) = &filters.organization_id {
                    let org_id_parsed: Option<OrganizationId> = org_id.parse().ok();
                    if cipher.organization_id != org_id_parsed {
                        return false;
                    }
                }

                // Folder filter (including "no folder" as None)
                if let Some(folder_id) = &filters.folder_id {
                    let folder_id_parsed: Option<FolderId> = folder_id.parse().ok();
                    if cipher.folder_id != folder_id_parsed {
                        return false;
                    }
                }

                // Collection filter
                if let Some(collection_id) = &filters.collection_id {
                    if let Ok(collection_id_parsed) = collection_id.parse::<CollectionId>() {
                        if !cipher.collection_ids.contains(&collection_id_parsed) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Note: Search and URL filters require decryption, handled after

                true
            })
            .map(|(id, cipher)| (id.clone(), cipher.clone()))
            .collect()
    }

    /// Filter decrypted folders by search term
    pub fn filter_folders(&self, folders: Vec<FolderView>, search: &str) -> Vec<FolderView> {
        let search_lower = search.to_lowercase();
        folders
            .into_iter()
            .filter(|f| f.name.to_lowercase().contains(&search_lower))
            .collect()
    }

    /// Filter decrypted collections by search term
    pub fn filter_collections(
        &self,
        collections: Vec<CollectionView>,
        search: &str,
    ) -> Vec<CollectionView> {
        let search_lower = search.to_lowercase();
        collections
            .into_iter()
            .filter(|c| c.name.to_lowercase().contains(&search_lower))
            .collect()
    }

    /// Find items whose decrypted name matches `search`.
    ///
    /// Exact (case-insensitive) name matches win outright; substring matches
    /// are only considered when nothing matches exactly. Otherwise searching
    /// for "GitHub" would be ambiguous whenever a "GitHub Enterprise" item also
    /// exists.
    ///
    /// Returns `(id, name)` pairs so callers can report ambiguity usefully.
    pub fn find_by_name(
        &self,
        ciphers: &[CipherListView],
        search: &str,
    ) -> Vec<(CipherId, String)> {
        let needle = search.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }

        let collect = |predicate: &dyn Fn(&str) -> bool| -> Vec<(CipherId, String)> {
            ciphers
                .iter()
                .filter(|c| predicate(&c.name.to_lowercase()))
                .filter_map(|c| c.id.map(|id| (id, c.name.clone())))
                .collect()
        };

        let exact = collect(&|name: &str| name == needle);
        if !exact.is_empty() {
            return exact;
        }

        collect(&|name: &str| name.contains(&needle))
    }

    /// Apply the filters that can only be evaluated after decryption.
    ///
    /// `filter_ciphers` handles everything expressible on encrypted metadata;
    /// `--search` and `--url` need plaintext, so they run here.
    pub fn filter_decrypted(
        &self,
        ciphers: Vec<CipherListView>,
        filters: &ItemFilters,
    ) -> Vec<CipherListView> {
        ciphers
            .into_iter()
            .filter(|c| {
                filters
                    .search
                    .as_deref()
                    .is_none_or(|s| self.matches_search(c, s))
            })
            .filter(|c| filters.url.as_deref().is_none_or(|u| self.matches_url(c, u)))
            .collect()
    }

    /// Case-insensitive substring match over the fields users expect
    /// `--search` to cover: the item name and its subtitle (the username, for
    /// logins).
    ///
    /// Notes are deliberately not searched: `CipherListView::notes` is
    /// wasm-only, so covering notes would mean fully decrypting every item on
    /// every `list` call.
    pub fn matches_search(&self, cipher: &CipherListView, search: &str) -> bool {
        let needle = search.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }

        cipher.name.to_lowercase().contains(&needle)
            || cipher.subtitle.to_lowercase().contains(&needle)
    }

    /// Match a login item's URIs against a target URL.
    ///
    /// When both sides parse as absolute URLs the hosts are compared, so
    /// `--url https://example.com/login` matches a stored `https://example.com`.
    /// Otherwise falls back to asking whether the stored URI contains the
    /// target.
    ///
    /// This is host equality, not Bitwarden's full `UriMatchType` semantics
    /// (base-domain, host, starts-with, regex, never) — those need
    /// `GlobalDomains` equivalence data the SDK does not expose to us.
    pub fn matches_url(&self, cipher: &CipherListView, target_url: &str) -> bool {
        let CipherListViewType::Login(login) = &cipher.r#type else {
            return false;
        };

        let Some(uris) = &login.uris else {
            return false;
        };

        let target = target_url.trim();
        if target.is_empty() {
            return true;
        }

        let target_host = Self::host_of(target);
        let target_lower = target.to_lowercase();

        uris.iter()
            .filter_map(|u| u.uri.as_deref())
            .any(|uri| match (&target_host, Self::host_of(uri)) {
                (Some(wanted), Some(stored)) => wanted == &stored,
                _ => uri.to_lowercase().contains(&target_lower),
            })
    }

    /// Host portion of an absolute URL, lowercased. `None` when the value is
    /// not an absolute URL (bare domains like `example.com` included).
    fn host_of(value: &str) -> Option<String> {
        url::Url::parse(value)
            .ok()?
            .host_str()
            .map(|h| h.to_lowercase())
    }
}

impl Default for SearchService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitwarden_vault::{CipherRepromptType, LoginListView, LoginUriView};
    use chrono::Utc;

    fn login_view(name: &str, subtitle: &str, uris: &[&str]) -> CipherListView {
        let now = Utc::now();

        CipherListView {
            id: Some(CipherId::new_v4()),
            organization_id: None,
            folder_id: None,
            collection_ids: vec![],
            key: None,
            name: name.to_string(),
            subtitle: subtitle.to_string(),
            r#type: CipherListViewType::Login(LoginListView {
                fido2_credentials: None,
                has_fido2: false,
                username: Some(subtitle.to_string()),
                totp: None,
                uris: Some(
                    uris.iter()
                        .map(|u| LoginUriView {
                            uri: Some((*u).to_string()),
                            r#match: None,
                            uri_checksum: None,
                        })
                        .collect(),
                ),
            }),
            favorite: false,
            reprompt: CipherRepromptType::None,
            organization_use_totp: false,
            edit: true,
            permissions: None,
            view_password: true,
            attachments: 0,
            has_old_attachments: false,
            creation_date: now,
            deleted_date: None,
            revision_date: now,
            archived_date: None,
            copyable_fields: vec![],
            local_data: None,
        }
    }

    fn note_view(name: &str) -> CipherListView {
        CipherListView {
            r#type: CipherListViewType::SecureNote,
            ..login_view(name, "", &[])
        }
    }

    // --- matches_search ---------------------------------------------------

    #[test]
    fn search_matches_name_case_insensitively() {
        let s = SearchService::new();
        let cipher = login_view("GitHub", "octocat", &[]);

        assert!(s.matches_search(&cipher, "github"));
        assert!(s.matches_search(&cipher, "HUB"));
    }

    #[test]
    fn search_matches_subtitle() {
        let s = SearchService::new();
        let cipher = login_view("GitHub", "octocat@example.com", &[]);

        assert!(s.matches_search(&cipher, "octocat"));
    }

    #[test]
    fn search_rejects_non_matches() {
        let s = SearchService::new();
        let cipher = login_view("GitHub", "octocat", &[]);

        assert!(!s.matches_search(&cipher, "gitlab"));
    }

    #[test]
    fn empty_search_matches_everything() {
        let s = SearchService::new();
        let cipher = login_view("GitHub", "octocat", &[]);

        assert!(s.matches_search(&cipher, "   "));
    }

    // --- matches_url -----------------------------------------------------

    #[test]
    fn url_matches_on_host_ignoring_path_and_scheme_details() {
        let s = SearchService::new();
        let cipher = login_view("GitHub", "octocat", &["https://github.com"]);

        assert!(s.matches_url(&cipher, "https://github.com/login/oauth"));
    }

    #[test]
    fn url_does_not_match_a_different_host() {
        let s = SearchService::new();
        let cipher = login_view("GitHub", "octocat", &["https://github.com"]);

        assert!(!s.matches_url(&cipher, "https://gitlab.com"));
    }

    /// Regression: the previous implementation also asked whether the *target*
    /// contained the stored URI, so a stored URI of "a" matched a search for
    /// "cat".
    #[test]
    fn url_does_not_match_when_target_merely_contains_the_stored_uri() {
        let s = SearchService::new();
        let cipher = login_view("Short", "user", &["a"]);

        assert!(!s.matches_url(&cipher, "cat"));
    }

    #[test]
    fn url_never_matches_non_login_items() {
        let s = SearchService::new();

        assert!(!s.matches_url(&note_view("My Note"), "https://example.com"));
    }

    #[test]
    fn bare_domain_falls_back_to_substring() {
        let s = SearchService::new();
        let cipher = login_view("GitHub", "octocat", &["https://github.com"]);

        // "github.com" is not an absolute URL, so no host to compare.
        assert!(s.matches_url(&cipher, "github.com"));
    }

    // --- find_by_name ----------------------------------------------------

    /// Regression: `find_cipher_by_name` used to match on *id prefix*, so
    /// `bw get item <name>` never found anything by name.
    #[test]
    fn find_by_name_matches_on_name_not_id() {
        let s = SearchService::new();
        let ciphers = vec![login_view("GitHub", "octocat", &[])];

        let found = s.find_by_name(&ciphers, "GitHub");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, "GitHub");
    }

    #[test]
    fn exact_name_match_wins_over_substring() {
        let s = SearchService::new();
        let ciphers = vec![
            login_view("GitHub", "octocat", &[]),
            login_view("GitHub Enterprise", "admin", &[]),
        ];

        let found = s.find_by_name(&ciphers, "github");

        assert_eq!(found.len(), 1, "exact match should not be ambiguous");
        assert_eq!(found[0].1, "GitHub");
    }

    #[test]
    fn substring_match_is_used_when_nothing_matches_exactly() {
        let s = SearchService::new();
        let ciphers = vec![login_view("GitHub Enterprise", "admin", &[])];

        let found = s.find_by_name(&ciphers, "enterprise");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, "GitHub Enterprise");
    }

    #[test]
    fn ambiguous_substring_returns_all_matches() {
        let s = SearchService::new();
        let ciphers = vec![
            login_view("Mail (work)", "a", &[]),
            login_view("Mail (personal)", "b", &[]),
        ];

        assert_eq!(s.find_by_name(&ciphers, "mail").len(), 2);
    }

    #[test]
    fn find_by_name_rejects_empty_search() {
        let s = SearchService::new();
        let ciphers = vec![login_view("GitHub", "octocat", &[])];

        assert!(s.find_by_name(&ciphers, "  ").is_empty());
    }

    // --- filter_decrypted -------------------------------------------------

    /// Regression: `--search` and `--url` were silently ignored because
    /// nothing ever called the matcher functions.
    #[test]
    fn filter_decrypted_applies_search_and_url_together() {
        let s = SearchService::new();
        let ciphers = vec![
            login_view("GitHub", "octocat", &["https://github.com"]),
            login_view("GitLab", "octocat", &["https://gitlab.com"]),
            login_view("GitHub Old", "octocat", &["https://example.com"]),
        ];

        let filters = ItemFilters {
            search: Some("github".to_string()),
            url: Some("https://github.com".to_string()),
            ..Default::default()
        };

        let result = s.filter_decrypted(ciphers, &filters);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "GitHub");
    }

    #[test]
    fn filter_decrypted_without_filters_is_identity() {
        let s = SearchService::new();
        let ciphers = vec![
            login_view("A", "a", &[]),
            login_view("B", "b", &[]),
        ];

        let result = s.filter_decrypted(ciphers, &ItemFilters::default());

        assert_eq!(result.len(), 2);
    }
}
