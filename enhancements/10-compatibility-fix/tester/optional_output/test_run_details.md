# Detailed Test Run Results

## Test Environment

- **Platform:** macOS (Darwin 25.1.0)
- **Rust Version:** stable
- **Date:** 2025-12-09

## Full Test Output

### Library Tests (`cargo test --lib`)

```
running 101 tests
test models::auth::device::tests::test_device_info_with_existing_id ... ok
test models::auth::device::tests::test_device_info_creation ... ok
test models::auth::session::tests::test_session_key_invalid_base64 ... ok
test models::auth::session::tests::test_session_key_invalid_length ... ok
test models::auth::session::tests::test_session_key_encoding ... ok
test models::api::auth::tests::test_password_login_request_form_encoding ... ok
test models::auth::session::tests::test_session_key_generation ... ok
test models::auth::session::tests::test_session_key_roundtrip ... ok
test models::auth::two_factor::tests::test_two_factor_method_display_names ... ok
test models::auth::two_factor::tests::test_two_factor_method_provider_codes ... ok
test models::send::send::tests::test_send_type_from_str ... ok
test models::state::kdf::tests::test_argon2id_conversion ... ok
test models::state::kdf::tests::test_argon2id_default_params ... ok
test models::state::kdf::tests::test_pbkdf2_conversion ... ok
test models::state::kdf::tests::test_pbkdf2_default_iterations ... ok
test services::api::environment::tests::test_default_cloud_environment ... ok
test services::auth::errors::tests::test_crypto_error_insufficient_kdf_conversion ... ok
test services::api::environment::tests::test_localhost_http_allowed ... ok
test services::api::environment::tests::test_trailing_slash_removal ... ok
test services::api::environment::tests::test_https_validation ... ok
test services::auth::errors::tests::test_crypto_error_invalid_mac_conversion ... ok
test services::api::environment::tests::test_custom_base_url ... ok
test services::auth::errors::tests::test_crypto_error_invalid_key_conversion ... ok
test services::auth::errors::tests::test_user_message_crypto_operation_failed ... ok
test services::auth::errors::tests::test_user_message_invalid_password ... ok
test services::auth::errors::tests::test_user_message_kdf_error ... ok
test services::auth::session_manager::tests::test_validate_session_key_invalid ... ok
test services::auth::session_manager::tests::test_generate_session_key ... ok
test services::auth::session_manager::tests::test_format_for_export ... ok
test services::generator::passphrase::tests::test_capitalization ... ok
test services::generator::passphrase::tests::test_custom_separator ... ok
test services::container::tests::test_service_container_creation ... ok
test services::generator::passphrase::tests::test_custom_word_count ... ok
test services::generator::passphrase::tests::test_include_number ... ok
test services::generator::passphrase::tests::test_default_passphrase_generation ... ok
test services::generator::passphrase::tests::test_passphrase_uses_valid_words ... ok
test services::generator::passphrase::tests::test_validation_word_count_too_high ... ok
test services::generator::passphrase::tests::test_validation_word_count_too_low ... ok
test services::generator::password::tests::test_custom_length ... ok
test services::generator::password::tests::test_default_password_generation ... ok
test services::generator::password::tests::test_excluded_characters ... ok
test services::generator::password::tests::test_minimum_requirements ... ok
test services::generator::password::tests::test_only_numbers ... ok
test services::generator::password::tests::test_validation_invalid_length ... ok
test services::generator::password::tests::test_validation_no_character_sets ... ok
test services::generator::password::tests::test_validation_requirements_exceed_length ... ok
test services::generator::wordlist::tests::test_no_empty_words ... ok
test services::generator::wordlist::tests::test_wordlist_contains_valid_words ... ok
test services::auth::session_manager::tests::test_device_id_persistence ... ok
test services::generator::passphrase::tests::test_passphrase_randomness ... ok
test services::sdk::tests::test_create_sdk_client_custom_urls ... ok
test services::sdk::tests::test_create_sdk_client_defaults ... ok
test services::sdk::tests::test_get_device_type ... ok
test services::generator::wordlist::tests::test_wordlist_size ... ok
test services::storage::account::tests::test_is_not_logged_in_without_active_account ... ok
test services::storage::account::tests::test_no_active_user_initially ... ok
test services::storage::account::tests::test_clear_active_account ... ok
test services::storage::account::tests::test_get_all_accounts ... ok
test services::storage::account::tests::test_register_and_get_account ... ok
test services::storage::account::tests::test_remove_account ... ok
test services::storage::account::tests::test_set_and_get_active_user ... ok
test services::storage::atomic::tests::test_temp_file_path ... ok
test services::storage::atomic::tests::test_atomic_write ... ok
test services::storage::json_storage::tests::test_get_set_string ... ok
test services::storage::json_storage::tests::test_has ... ok
test services::storage::json_storage::tests::test_new_storage ... ok
test services::storage::atomic::tests::test_overwrite_existing_file ... ok
test services::storage::json_storage::tests::test_nested_keys ... ok
test services::storage::keys::tests::test_global_key_formatting ... ok
test services::storage::keys::tests::test_requires_user_id ... ok
test services::storage::keys::tests::test_user_key_formatting ... ok
test services::storage::keys::tests::test_user_key_without_user_id_panics - should panic ... ok
test services::storage::path::tests::test_custom_path ... ok
test services::storage::json_storage::tests::test_persistence ... ok
test services::storage::path::tests::test_env_var_override ... ok
test services::storage::path::tests::test_directory_creation ... ok
test services::storage::path::tests::test_is_writable ... ok
test services::vault::validation_service::tests::test_validate_cipher_create_success ... ok
test services::vault::validation_service::tests::test_validate_card_type_mismatch ... ok
test services::vault::validation_service::tests::test_validate_cipher_missing_name ... ok
test services::vault::validation_service::tests::test_validate_cipher_name_too_long ... ok
test services::vault::validation_service::tests::test_validate_cipher_update_missing_id ... ok
test services::vault::validation_service::tests::test_validate_folder_name_empty ... ok
test services::vault::validation_service::tests::test_validate_cipher_update_with_id_success ... ok
test services::vault::validation_service::tests::test_validate_folder_name_success ... ok
test services::vault::validation_service::tests::test_validate_folder_name_too_long ... ok
test services::vault::validation_service::tests::test_validate_identity_type_mismatch ... ok
test services::vault::validation_service::tests::test_validate_invalid_organization_uuid ... ok
test services::vault::validation_service::tests::test_validate_notes_too_long ... ok
test services::vault::validation_service::tests::test_validate_invalid_uuid ... ok
test services::vault::validation_service::tests::test_validate_secure_note_type_mismatch ... ok
test services::vault::validation_service::tests::test_validate_totp_invalid_format ... ok
test services::storage::json_storage::tests::test_remove ... ok
test services::vault::validation_service::tests::test_validate_totp_valid_format ... ok
test services::vault::validation_service::tests::test_validate_uri_too_long ... ok
test services::vault::validation_service::tests::test_validate_valid_uuid ... ok
test services::crypto::tests::test_decrypt_user_key_invalid_format ... ok
test services::crypto::tests::test_derive_master_key_pbkdf2 ... ok
test services::crypto::tests::test_derive_master_key_argon2id ... ok
test services::crypto::tests::test_email_normalization ... ok
test services::crypto::tests::test_derive_master_key_pbkdf2_600k ... ok

test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.23s
```

## Tests Added by This Enhancement

### StorageKey Tests (keys.rs)

```rust
#[test]
fn test_global_key_formatting() {
    assert_eq!(StorageKey::StateVersion.format(None), "stateVersion");
    assert_eq!(
        StorageKey::GlobalAppId.format(None),
        "global_applicationId_appId"
    );
    assert_eq!(
        StorageKey::GlobalAccounts.format(None),
        "global_account_accounts"
    );
    assert_eq!(
        StorageKey::GlobalActiveAccountId.format(None),
        "global_account_activeAccountId"
    );
}

#[test]
fn test_user_key_formatting() {
    let user_id = "abc-123-def";
    assert_eq!(
        StorageKey::UserAccessToken.format(Some(user_id)),
        "user_abc-123-def_token_accessToken"
    );
    assert_eq!(
        StorageKey::UserRefreshToken.format(Some(user_id)),
        "user_abc-123-def_token_refreshToken"
    );
    assert_eq!(
        StorageKey::UserPrivateKey.format(Some(user_id)),
        "user_abc-123-def_crypto_privateKey"
    );
    assert_eq!(
        StorageKey::UserKdfConfig.format(Some(user_id)),
        "user_abc-123-def_kdf_config"
    );
}

#[test]
#[should_panic(expected = "UserAccessToken requires user_id")]
fn test_user_key_without_user_id_panics() {
    StorageKey::UserAccessToken.format(None);
}

#[test]
fn test_requires_user_id() {
    // Global keys
    assert!(!StorageKey::StateVersion.requires_user_id());
    assert!(!StorageKey::GlobalAppId.requires_user_id());
    assert!(!StorageKey::GlobalAccounts.requires_user_id());
    assert!(!StorageKey::GlobalActiveAccountId.requires_user_id());
    assert!(!StorageKey::DeviceId.requires_user_id());

    // User keys
    assert!(StorageKey::UserAccessToken.requires_user_id());
    assert!(StorageKey::UserRefreshToken.requires_user_id());
    assert!(StorageKey::UserPrivateKey.requires_user_id());
    assert!(StorageKey::UserKdfConfig.requires_user_id());
}
```

### AccountManager Tests (account.rs)

```rust
#[tokio::test]
async fn test_no_active_user_initially() {
    let (manager, _temp) = create_test_account_manager().await;
    let user_id = manager.get_active_user_id().await.unwrap();
    assert!(user_id.is_none());
}

#[tokio::test]
async fn test_set_and_get_active_user() {
    let (manager, _temp) = create_test_account_manager().await;
    manager.set_active_user_id("test-user-123").await.unwrap();
    let user_id = manager.get_active_user_id().await.unwrap();
    assert_eq!(user_id, Some("test-user-123".to_string()));
}

#[tokio::test]
async fn test_clear_active_account() {
    let (manager, _temp) = create_test_account_manager().await;
    manager.set_active_user_id("test-user-123").await.unwrap();
    manager.clear_active_account().await.unwrap();
    let user_id = manager.get_active_user_id().await.unwrap();
    assert!(user_id.is_none());
}

#[tokio::test]
async fn test_register_and_get_account() {
    let (manager, _temp) = create_test_account_manager().await;
    manager.register_account("user-123", "test@example.com").await.unwrap();
    let account = manager.get_account("user-123").await.unwrap();
    assert!(account.is_some());
    let account = account.unwrap();
    assert_eq!(account.email, "test@example.com");
    assert!(account.email_verified);
}

#[tokio::test]
async fn test_get_all_accounts() {
    let (manager, _temp) = create_test_account_manager().await;
    manager.register_account("user-1", "user1@example.com").await.unwrap();
    manager.register_account("user-2", "user2@example.com").await.unwrap();
    let accounts = manager.get_all_accounts().await.unwrap();
    assert_eq!(accounts.len(), 2);
    assert!(accounts.contains_key("user-1"));
    assert!(accounts.contains_key("user-2"));
}

#[tokio::test]
async fn test_remove_account() {
    let (manager, _temp) = create_test_account_manager().await;
    manager.register_account("user-1", "user1@example.com").await.unwrap();
    let removed = manager.remove_account("user-1").await.unwrap();
    assert!(removed);
    let account = manager.get_account("user-1").await.unwrap();
    assert!(account.is_none());
}

#[tokio::test]
async fn test_is_not_logged_in_without_active_account() {
    let (manager, _temp) = create_test_account_manager().await;
    let logged_in = manager.is_logged_in().await.unwrap();
    assert!(!logged_in);
}
```

## Build Verification

### Release Build Output
```
cargo build --release
   Compiling bw-core v0.1.0 (/Users/bgentry/Source/repos/bwcli-rs/crates/bw-core)
warning: field `storage` is never read
warning: methods `save_tokens` and `clear_tokens` are never used
warning: field `sdk_client` is never read (2x)
    Finished `release` profile [optimized] target(s) in 1m 01s
```

Note: All warnings are pre-existing and unrelated to this enhancement.

## Clippy Analysis

Pre-existing warnings only:
- `dead_code` warnings for unused fields/methods in api/client.rs, token_manager.rs, cipher_service.rs, totp_service.rs
- `module_inception` in models/send/mod.rs
- `if_same_then_else` in services/api/errors.rs
- `collapsible_else_if` in services/vault/search_service.rs

No new warnings introduced by this enhancement.
