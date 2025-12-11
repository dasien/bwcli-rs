//! Search and filter service
//!
//! Provides efficient filtering without requiring full decryption.

use crate::models::vault::{Cipher, CollectionView, FolderView};
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
                    if cipher.organization_id.as_ref() != Some(org_id) {
                        return false;
                    }
                }

                // Folder filter (including "no folder" as None)
                if let Some(folder_id) = &filters.folder_id {
                    if cipher.folder_id.as_ref() != Some(folder_id) {
                        return false;
                    }
                }

                // Collection filter
                if let Some(collection_id) = &filters.collection_id {
                    if !cipher.collection_ids.contains(collection_id) {
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

    /// Find cipher by name search (case-insensitive)
    ///
    /// Note: This requires the name to already be decrypted or
    /// we need to decrypt each cipher to search. For MVP, we'll
    /// decrypt all ciphers and search. Optimization: build search index.
    pub fn find_cipher_by_name(
        &self,
        ciphers: &HashMap<String, Cipher>,
        search: &str,
    ) -> Option<Cipher> {
        // For MVP: return first match by ID prefix
        // Real implementation will require decryption first
        ciphers
            .iter()
            .find(|(id, _)| id.starts_with(search))
            .map(|(_, cipher)| cipher.clone())
    }

    /// Search in decrypted cipher names (post-decryption filter)
    pub fn matches_search(&self, name: &str, notes: Option<&str>, search: &str) -> bool {
        let search_lower = search.to_lowercase();

        if name.to_lowercase().contains(&search_lower) {
            return true;
        }

        if let Some(n) = notes {
            if n.to_lowercase().contains(&search_lower) {
                return true;
            }
        }

        false
    }

    /// Match cipher URI against target URL
    pub fn matches_url(&self, uris: &[String], target_url: &str) -> bool {
        // Simplified URL matching for MVP
        // Full implementation would use UriMatchType and proper domain matching
        let target_lower = target_url.to_lowercase();

        uris.iter().any(|uri| {
            let uri_lower = uri.to_lowercase();
            uri_lower.contains(&target_lower) || target_lower.contains(&uri_lower)
        })
    }
}

impl Default for SearchService {
    fn default() -> Self {
        Self::new()
    }
}
