# Detailed Test Results

## Test Run Information

- **Date**: 2025-12-09
- **Platform**: Darwin 25.1.0
- **Rust Version**: (cargo build succeeded)
- **Enhancement**: 11-encrypt-decrypt

---

## Full Test Output: bw-core Unit Tests

```
running 115 tests
test models::auth::device::tests::test_device_info_with_existing_id ... ok
test models::auth::device::tests::test_device_info_creation ... ok
test models::auth::session::tests::test_session_key_invalid_base64 ... ok
test models::auth::session::tests::test_session_key_invalid_length ... ok
test models::auth::session::tests::test_session_key_generation ... ok
test models::auth::session::tests::test_session_key_encoding ... ok
test models::auth::session::tests::test_session_key_roundtrip ... ok
test models::auth::two_factor::tests::test_two_factor_method_display_names ... ok
test models::auth::two_factor::tests::test_two_factor_method_provider_codes ... ok
test models::api::auth::tests::test_password_login_request_form_encoding ... ok
test models::send::send::tests::test_send_type_from_str ... ok
test models::state::kdf::tests::test_argon2id_conversion ... ok
test models::state::kdf::tests::test_argon2id_default_params ... ok
test models::state::kdf::tests::test_pbkdf2_conversion ... ok
test models::state::kdf::tests::test_pbkdf2_default_iterations ... ok
test models::state::kdf::tests::test_typescript_cli_format_deserialization ... ok
test services::api::environment::tests::test_default_cloud_environment ... ok
test services::auth::errors::tests::test_crypto_error_insufficient_kdf_conversion ... ok
test services::api::environment::tests::test_localhost_http_allowed ... ok
test services::api::environment::tests::test_custom_base_url ... ok
test services::api::environment::tests::test_trailing_slash_removal ... ok
test services::auth::errors::tests::test_crypto_error_invalid_key_conversion ... ok
test services::api::environment::tests::test_https_validation ... ok
test services::auth::errors::tests::test_crypto_error_invalid_mac_conversion ... ok
test services::auth::errors::tests::test_user_message_crypto_operation_failed ... ok
test services::auth::errors::tests::test_user_message_invalid_password ... ok
test services::auth::errors::tests::test_user_message_kdf_error ... ok
test services::auth::session_manager::tests::test_validate_session_key_invalid ... ok
test services::auth::session_manager::tests::test_format_for_export ... ok
test services::auth::session_manager::tests::test_generate_session_key ... ok
test services::generator::passphrase::tests::test_capitalization ... ok
test services::generator::passphrase::tests::test_custom_separator ... ok
test services::generator::passphrase::tests::test_custom_word_count ... ok
test services::generator::passphrase::tests::test_default_passphrase_generation ... ok
test services::generator::passphrase::tests::test_include_number ... ok
test services::container::tests::test_service_container_creation ... ok
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
test services::generator::wordlist::tests::test_new_wordlist_custom ... ok
test services::generator::wordlist::tests::test_new_wordlist_default ... ok
test services::generator::wordlist::tests::test_new_wordlist_eff_large ... ok
test services::generator::wordlist::tests::test_sample_words ... ok
test services::key_service::tests::test_clear_user_key ... ok
test services::key_service::tests::test_invalid_session_key ... ok
test services::key_service::tests::test_no_active_user ... ok
test services::key_service::tests::test_store_and_retrieve_user_key ... ok
test services::key_service::tests::test_user_key_not_found ... ok
test services::storage::json_storage::tests::test_contains_key ... ok
test services::storage::json_storage::tests::test_get_set ... ok
test services::storage::json_storage::tests::test_is_writable ... ok
test services::storage::protected_storage::tests::test_encrypt_decrypt_string_roundtrip ... ok
test services::storage::protected_storage::tests::test_invalid_session_key ... ok
test services::storage::protected_storage::tests::test_make_protected_key ... ok
test services::storage::protected_storage::tests::test_protected_storage_key_format ... ok
test services::storage::protected_storage::tests::test_session_key_roundtrip ... ok
test services::storage::protected_storage::tests::test_user_key_protected_storage_key ... ok
test services::storage::protected_storage::tests::test_encrypt_decrypt_user_key_roundtrip ... ok
test services::storage::protected_storage::tests::test_wrong_key_fails_decryption ... ok
test services::vault::validation_service::tests::test_validate_card_type_mismatch ... ok
test services::vault::validation_service::tests::test_validate_cipher_create_success ... ok
test services::vault::validation_service::tests::test_validate_cipher_missing_name ... ok
test services::vault::validation_service::tests::test_validate_cipher_name_too_long ... ok
test services::vault::validation_service::tests::test_validate_cipher_update_missing_id ... ok
test services::vault::validation_service::tests::test_validate_cipher_update_with_id_success ... ok
test services::vault::validation_service::tests::test_validate_folder_name_empty ... ok
test services::vault::validation_service::tests::test_validate_folder_name_success ... ok
test services::storage::json_storage::tests::test_remove ... ok
test services::vault::validation_service::tests::test_validate_folder_name_too_long ... ok
test services::vault::validation_service::tests::test_validate_identity_type_mismatch ... ok
test services::vault::validation_service::tests::test_validate_invalid_organization_uuid ... ok
test services::vault::validation_service::tests::test_validate_invalid_uuid ... ok
test services::vault::validation_service::tests::test_validate_notes_too_long ... ok
test services::vault::validation_service::tests::test_validate_secure_note_type_mismatch ... ok
test services::vault::validation_service::tests::test_validate_totp_invalid_format ... ok
test services::vault::validation_service::tests::test_validate_totp_valid_format ... ok
test services::vault::validation_service::tests::test_validate_uri_too_long ... ok
test services::vault::validation_service::tests::test_validate_valid_uuid ... ok
test services::crypto::tests::test_decrypt_user_key_invalid_format ... ok
test services::crypto::tests::test_derive_master_key_pbkdf2 ... ok
test services::crypto::tests::test_derive_master_key_argon2id ... ok
test services::crypto::tests::test_email_normalization ... ok
test services::crypto::tests::test_derive_master_key_pbkdf2_600k ... ok

test result: ok. 115 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.48s
```

---

## Full Test Output: auth_service_tests

```
running 9 tests
test test_unlock_not_logged_in ... ok
test test_lock ... FAILED
test test_login_with_api_key_success ... ok
test test_logout_success ... FAILED
test test_unlock_success ... FAILED
test test_unlock_wrong_password ... FAILED
test test_session_key_format ... FAILED
test test_login_with_password_invalid_credentials ... ok
test test_login_with_password_success ... FAILED

failures:

---- test_lock stdout ----
thread 'test_lock' panicked at crates/bw-core/tests/auth_service_tests.rs:385:5:
assertion failed: result.is_ok()

---- test_logout_success stdout ----
thread 'test_logout_success' panicked at crates/bw-core/tests/auth_service_tests.rs:441:5:
assertion failed: login_result.is_ok()

---- test_unlock_success stdout ----
thread 'test_unlock_success' panicked at crates/bw-core/tests/auth_service_tests.rs:273:5:
assertion failed: login_result.is_ok()

---- test_unlock_wrong_password stdout ----
thread 'test_unlock_wrong_password' panicked at crates/bw-core/tests/auth_service_tests.rs:361:5:
assertion failed: login_result.is_ok()

---- test_session_key_format stdout ----
thread 'test_session_key_format' panicked at crates/bw-core/tests/auth_service_tests.rs:517:10:
called `Result::unwrap()` on an `Err` value: CryptoOperationFailed { message: "Invalid encryption key" }

---- test_login_with_password_success stdout ----
thread 'test_login_with_password_success' panicked at crates/bw-core/tests/auth_service_tests.rs:105:5:
Login should succeed: Some(CryptoOperationFailed { message: "Invalid encryption key" })


failures:
    test_lock
    test_login_with_password_success
    test_logout_success
    test_session_key_format
    test_unlock_success
    test_unlock_wrong_password

test result: FAILED. 3 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## Full Test Output: vault_write_service_tests

```
running 12 tests
test test_create_cipher_rejects_invalid_uuid ... FAILED
test test_create_cipher_rejects_field_too_long ... FAILED
test test_create_cipher_rejects_invalid_input ... FAILED
test test_create_folder_rejects_name_too_long ... FAILED
test test_create_card_without_card_data_fails ... FAILED
test test_create_login_without_login_data_fails ... FAILED
test test_create_identity_without_identity_data_fails ... FAILED
test test_create_folder_rejects_empty_name ... FAILED
test test_create_secure_note_without_secure_note_data_fails ... FAILED
test test_validate_folder_exists_returns_error_when_not_found ... ok
test test_update_folder_rejects_empty_name ... FAILED
test test_validate_cipher_exists_returns_error_when_not_found ... FAILED

failures:

---- test_create_cipher_rejects_invalid_uuid stdout ----
thread 'test_create_cipher_rejects_invalid_uuid' panicked at crates/bw-core/tests/vault_write_service_tests.rs:170:5:
assertion failed: matches!(result.unwrap_err(), VaultError::ValidationError(_))

---- test_create_cipher_rejects_field_too_long stdout ----
thread 'test_create_cipher_rejects_field_too_long' panicked at crates/bw-core/tests/vault_write_service_tests.rs:204:5:
assertion failed: matches!(result.unwrap_err(), VaultError::ValidationError(_))

---- test_create_cipher_rejects_invalid_input stdout ----
thread 'test_create_cipher_rejects_invalid_input' panicked at crates/bw-core/tests/vault_write_service_tests.rs:136:5:
assertion failed: matches!(result.unwrap_err(), VaultError::ValidationError(_))

[...additional similar failures...]

test result: FAILED. 1 passed; 11 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## Analysis: Why Tests Fail

### auth_service_tests Failures

The mock server responses use plain strings for the `Key` field:
```json
"Key": "mock_encrypted_user_key"
```

But the real Bitwarden SDK expects an EncString format like:
```
2.{base64_iv}|{base64_ciphertext}|{base64_mac}
```

When `decrypt_user_key()` is called with this invalid format, it returns `CryptoOperationFailed { message: "Invalid encryption key" }`.

### vault_write_service_tests Failures

The WriteService methods now call `get_user_key(session)` as the FIRST operation:

```rust
pub async fn create_cipher(
    &self,
    mut cipher_view: CipherView,
    session: &str,
) -> Result<Cipher, VaultError> {
    // Get user key for encryption
    let user_key = self.get_user_key(session).await?;  // <-- This fails first!

    // 1. Validate input structure
    self.validation_service
        .validate_cipher_create(&cipher_view)?;
    // ...
}
```

Since tests pass `"dummy"` as the session key without setting up:
1. An active user account
2. A protected storage entry with the user key

The `get_user_key()` call fails before validation can be tested.

---

## Suggested Test Fixes

### For auth_service_tests

Option 1: Create proper EncString test fixtures
```rust
// Generate a real encrypted key for testing
let master_key = derive_master_key("password", "test@example.com", &kdf);
let user_key = SymmetricCryptoKey::generate();
let encrypted_key = encrypt_user_key(&user_key, &master_key);
// Use encrypted_key in mock response
```

Option 2: Mark tests as integration tests requiring real server

### For vault_write_service_tests

Option 1: Reorder WriteService to validate first
```rust
pub async fn create_cipher(&self, cipher_view: CipherView, session: &str) {
    // Validate first (doesn't need encryption)
    self.validation_service.validate_cipher_create(&cipher_view)?;

    // Then get user key (only if validation passed)
    let user_key = self.get_user_key(session).await?;
    // ...
}
```

Option 2: Set up proper test fixtures
```rust
async fn setup_test_with_session() -> (WriteService, String) {
    // Set up storage with active user
    // Store encrypted user key in protected storage
    // Return valid session key
}
```
