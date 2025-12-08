# Detailed Test Results

## Test Execution Summary

### Command: `cargo test --all`

```
running 88 tests (bw-core lib)
...
test result: ok. 88 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 9 tests (auth_service_tests)
test result: FAILED. 1 passed; 8 failed; 0 ignored
```

## Complete Unit Test List (88 tests)

### Models (14 tests)

#### auth/device.rs (2 tests)
- `test_device_info_creation` - PASS
- `test_device_info_with_existing_id` - PASS

#### auth/session.rs (5 tests)
- `test_session_key_generation` - PASS
- `test_session_key_encoding` - PASS
- `test_session_key_roundtrip` - PASS
- `test_session_key_invalid_base64` - PASS
- `test_session_key_invalid_length` - PASS

#### auth/two_factor.rs (2 tests)
- `test_two_factor_method_display_names` - PASS
- `test_two_factor_method_provider_codes` - PASS

#### state/kdf.rs (4 tests) - **SDK INTEGRATION**
- `test_pbkdf2_conversion` - PASS
- `test_pbkdf2_default_iterations` - PASS
- `test_argon2id_conversion` - PASS
- `test_argon2id_default_params` - PASS

#### send/send.rs (1 test)
- `test_send_type_from_str` - PASS

### Services (74 tests)

#### api/environment.rs (5 tests)
- `test_default_cloud_environment` - PASS
- `test_custom_base_url` - PASS
- `test_https_validation` - PASS
- `test_localhost_http_allowed` - PASS
- `test_trailing_slash_removal` - PASS

#### auth/errors.rs (6 tests) - **NEW - SDK INTEGRATION**
- `test_crypto_error_invalid_key_conversion` - PASS
- `test_crypto_error_invalid_mac_conversion` - PASS
- `test_crypto_error_insufficient_kdf_conversion` - PASS
- `test_user_message_invalid_password` - PASS
- `test_user_message_kdf_error` - PASS
- `test_user_message_crypto_operation_failed` - PASS

#### auth/session_manager.rs (4 tests)
- `test_generate_session_key` - PASS
- `test_format_for_export` - PASS
- `test_validate_session_key_invalid` - PASS
- `test_device_id_persistence` - PASS

#### container.rs (1 test)
- `test_service_container_creation` - PASS

#### crypto.rs (4 tests) - **SDK INTEGRATION**
- `test_derive_master_key_pbkdf2` - PASS
- `test_derive_master_key_argon2id` - PASS
- `test_email_normalization` - PASS
- `test_decrypt_user_key_invalid_format` - PASS

#### generator/passphrase.rs (9 tests)
- `test_default_passphrase_generation` - PASS
- `test_custom_word_count` - PASS
- `test_custom_separator` - PASS
- `test_capitalization` - PASS
- `test_include_number` - PASS
- `test_validation_word_count_too_low` - PASS
- `test_validation_word_count_too_high` - PASS
- `test_passphrase_randomness` - PASS
- `test_passphrase_uses_valid_words` - PASS

#### generator/password.rs (8 tests)
- `test_default_password_generation` - PASS
- `test_custom_length` - PASS
- `test_only_numbers` - PASS
- `test_excluded_characters` - PASS
- `test_minimum_requirements` - PASS
- `test_validation_invalid_length` - PASS
- `test_validation_no_character_sets` - PASS
- `test_validation_requirements_exceed_length` - PASS

#### generator/wordlist.rs (3 tests)
- `test_wordlist_size` - PASS
- `test_wordlist_contains_valid_words` - PASS
- `test_no_empty_words` - PASS

#### sdk.rs (3 tests) - **SDK INTEGRATION**
- `test_create_sdk_client_defaults` - PASS
- `test_create_sdk_client_custom_urls` - PASS
- `test_get_device_type` - PASS

#### storage/atomic.rs (3 tests)
- `test_temp_file_path` - PASS
- `test_atomic_write` - PASS
- `test_overwrite_existing_file` - PASS

#### storage/json_storage.rs (6 tests)
- `test_new_storage` - PASS
- `test_get_set_string` - PASS
- `test_nested_keys` - PASS
- `test_has` - PASS
- `test_persistence` - PASS
- `test_remove` - PASS

#### storage/path.rs (4 tests)
- `test_custom_path` - PASS
- `test_directory_creation` - PASS
- `test_env_var_override` - PASS
- `test_is_writable` - PASS

#### vault/validation_service.rs (18 tests)
- `test_validate_cipher_create_success` - PASS
- `test_validate_cipher_update_with_id_success` - PASS
- `test_validate_cipher_update_missing_id` - PASS
- `test_validate_cipher_missing_name` - PASS
- `test_validate_cipher_name_too_long` - PASS
- `test_validate_notes_too_long` - PASS
- `test_validate_uri_too_long` - PASS
- `test_validate_card_type_mismatch` - PASS
- `test_validate_identity_type_mismatch` - PASS
- `test_validate_secure_note_type_mismatch` - PASS
- `test_validate_folder_name_success` - PASS
- `test_validate_folder_name_empty` - PASS
- `test_validate_folder_name_too_long` - PASS
- `test_validate_valid_uuid` - PASS
- `test_validate_invalid_uuid` - PASS
- `test_validate_invalid_organization_uuid` - PASS
- `test_validate_totp_valid_format` - PASS
- `test_validate_totp_invalid_format` - PASS

## Integration Test Failures (Pre-existing)

### test_login_with_password_success
**Error**: `KdfError { message: "Failed to fetch KDF config: Resource not found: /api/identity/accounts/prelogin" }`
**Root Cause**: API client uses wrong base URL for prelogin endpoint

### test_login_with_password_invalid_credentials
**Error**: `Expected InvalidCredentials error, got: KdfError`
**Root Cause**: Same API routing issue - prelogin fails before login can attempt

### test_login_with_api_key_success
**Error**: `Failed to write storage file .../test_data.json/data.lock: No such file or directory`
**Root Cause**: Test creates file as storage path, but storage expects directory

### test_unlock_success
**Error**: `assertion failed: login_result.is_ok()`
**Root Cause**: Login fails due to API routing issue

### test_unlock_wrong_password
**Error**: `assertion failed: login_result.is_ok()`
**Root Cause**: Login fails due to API routing issue

### test_lock
**Error**: `assertion failed: result.is_ok()`
**Root Cause**: Lock requires logged-in state, setup fails

### test_logout_success
**Error**: `assertion failed: login_result.is_ok()`
**Root Cause**: Login fails due to API routing issue

### test_session_key_format
**Error**: `KdfError { message: "Failed to fetch KDF config: Resource not found: /api/identity/accounts/prelogin" }`
**Root Cause**: Same API routing issue

### test_unlock_not_logged_in
**Status**: PASS
**Note**: This test doesn't require login, so it passes

## Build Output

```
cargo build --all
   Compiling bw-core v0.1.0
   Compiling bw-cli v0.1.0
    Finished `dev` profile [unoptimized + debuginfo]
```

Warnings (unrelated to SDK):
- `storage` field never read in `BitwardenApiClient`
- `save_tokens` and `clear_tokens` methods never used in `TokenManager`
- `sdk_client` field never read in `CipherService` and `TotpService`

## Clippy Output

No errors. Minor warnings for:
- `let_and_return` in main.rs (unrelated)
- `collapsible_if` in prompts.rs (unrelated)
