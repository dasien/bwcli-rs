# SDK Vault Bridge Test Details

## Test File Location

Tests are located in:
- `crates/bw-core/src/services/vault/sdk_bridge.rs` (inline tests module)

## New Tests Added

31 new unit tests were added to verify the SDK vault bridge implementation:

### KdfConfig Tests

```rust
// Test PBKDF2 conversion with explicit iterations
test_to_sdk_kdf_pbkdf2_with_iterations

// Test PBKDF2 with default 600,000 iterations
test_to_sdk_kdf_pbkdf2_default_iterations

// Test Argon2id with all parameters (note: memory is converted to KiB)
test_to_sdk_kdf_argon2id_with_params

// Test Argon2id with default parameters
test_to_sdk_kdf_argon2id_default_params
```

### SdkVaultBridge Tests

```rust
// Verify bridge creation and initial crypto state
test_sdk_vault_bridge_creation
```

### CipherType Conversion Tests

```rust
// Individual type conversions
test_cipher_type_conversion_login
test_cipher_type_conversion_secure_note
test_cipher_type_conversion_card
test_cipher_type_conversion_identity
test_cipher_type_conversion_ssh_key

// Comprehensive roundtrip test
test_sdk_cipher_type_to_cli_all_types
```

### UriMatchType Tests

```rust
// Bidirectional conversion for all 6 match types
test_uri_match_type_conversion_roundtrip
```

### View Conversion Tests

```rust
// Login view conversions
test_login_view_to_sdk_with_all_fields
test_login_view_to_sdk_with_empty_uris

// Card view conversion
test_card_view_conversion

// Identity view conversion
test_identity_view_conversion

// Field view conversions (all types)
test_field_view_conversion_text
test_field_view_conversion_hidden
test_field_view_conversion_boolean
test_field_view_conversion_linked
test_field_view_conversion_unknown_defaults_to_text

// SDK to CLI field conversion
test_sdk_field_view_to_cli
test_sdk_field_view_to_cli_no_name
```

### JSON Serialization Tests

```rust
// Structure preservation tests
test_cipher_json_serialization_preserves_structure
test_folder_json_serialization_preserves_structure
test_collection_json_serialization_preserves_structure

// Error handling
test_json_conversion_error_handling
```

### CipherView Conversion Tests

```rust
// Full view conversions
test_cli_cipher_view_to_sdk_login
test_cli_cipher_view_to_sdk_secure_note
test_cli_cipher_view_to_sdk_with_fields
```

### Error Tests

```rust
// Error message verification
test_vault_error_crypto_init_failed
```

## Test Output

```
running 31 tests
test services::vault::sdk_bridge::tests::test_cipher_type_conversion_card ... ok
test services::vault::sdk_bridge::tests::test_cipher_type_conversion_identity ... ok
test services::vault::sdk_bridge::tests::test_cipher_type_conversion_login ... ok
test services::vault::sdk_bridge::tests::test_cipher_type_conversion_secure_note ... ok
test services::vault::sdk_bridge::tests::test_cipher_type_conversion_ssh_key ... ok
test services::vault::sdk_bridge::tests::test_card_view_conversion ... ok
test services::vault::sdk_bridge::tests::test_field_view_conversion_boolean ... ok
test services::vault::sdk_bridge::tests::test_collection_json_serialization_preserves_structure ... ok
test services::vault::sdk_bridge::tests::test_field_view_conversion_hidden ... ok
test services::vault::sdk_bridge::tests::test_field_view_conversion_linked ... ok
test services::vault::sdk_bridge::tests::test_cli_cipher_view_to_sdk_with_fields ... ok
test services::vault::sdk_bridge::tests::test_cli_cipher_view_to_sdk_secure_note ... ok
test services::vault::sdk_bridge::tests::test_cipher_json_serialization_preserves_structure ... ok
test services::vault::sdk_bridge::tests::test_cli_cipher_view_to_sdk_login ... ok
test services::vault::sdk_bridge::tests::test_field_view_conversion_text ... ok
test services::vault::sdk_bridge::tests::test_field_view_conversion_unknown_defaults_to_text ... ok
test services::vault::sdk_bridge::tests::test_folder_json_serialization_preserves_structure ... ok
test services::vault::sdk_bridge::tests::test_identity_view_conversion ... ok
test services::vault::sdk_bridge::tests::test_login_view_to_sdk_with_all_fields ... ok
test services::vault::sdk_bridge::tests::test_login_view_to_sdk_with_empty_uris ... ok
test services::vault::sdk_bridge::tests::test_sdk_cipher_type_to_cli_all_types ... ok
test services::vault::sdk_bridge::tests::test_sdk_field_view_to_cli ... ok
test services::vault::sdk_bridge::tests::test_sdk_field_view_to_cli_no_name ... ok
test services::vault::sdk_bridge::tests::test_json_conversion_error_handling ... ok
test services::vault::sdk_bridge::tests::test_to_sdk_kdf_argon2id_default_params ... ok
test services::vault::sdk_bridge::tests::test_to_sdk_kdf_argon2id_with_params ... ok
test services::vault::sdk_bridge::tests::test_to_sdk_kdf_pbkdf2_default_iterations ... ok
test services::vault::sdk_bridge::tests::test_to_sdk_kdf_pbkdf2_with_iterations ... ok
test services::vault::sdk_bridge::tests::test_uri_match_type_conversion_roundtrip ... ok
test services::vault::sdk_bridge::tests::test_vault_error_crypto_init_failed ... ok
test services::vault::sdk_bridge::tests::test_sdk_vault_bridge_creation ... ok

test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 103 filtered out
```

## Design Notes

### JSON Conversion Testing

The JSON conversion functions (`cli_cipher_to_sdk_via_json`, `cli_folder_to_sdk_via_json`, `cli_collection_to_sdk_via_json`) require valid EncString values. The SDK performs strict validation during deserialization:

```
Error: DecryptionError("Failed to convert to SDK cipher: EncString(InvalidBase64(NotB64EncodedError))")
```

This is expected behavior as the SDK protects against malformed encrypted data. For full JSON conversion testing, integration tests with real encrypted vault data should be used.

### Memory Conversion for Argon2id

The implementation correctly converts Argon2 memory from MB (CLI format) to KiB (SDK format):

```rust
memory: std::num::NonZeroU32::new(self.memory.unwrap_or(64) * 1024).unwrap()
```

This is verified in `test_to_sdk_kdf_argon2id_with_params`:
```rust
assert_eq!(memory.get(), 64 * 1024); // 64 MB * 1024 = 65536 KiB
```
