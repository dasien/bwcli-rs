mod account;
mod atomic;
mod errors;
mod json_storage;
mod keys;
mod path;
pub mod protected_storage;
mod traits;

// Public exports
pub use account::{AccountInfo, AccountManager};
pub use errors::StorageError;
pub use json_storage::JsonFileStorage;
pub use keys::{SUPPORTED_STATE_VERSION, StorageKey};
pub use protected_storage::{
    ProtectedStorageError, decrypt_protected_bytes, decrypt_protected_string, decrypt_user_key,
    encrypt_protected_bytes, encrypt_protected_string, encrypt_user_key, format_session_key,
    generate_session_key, make_protected_key, parse_session_key, user_key_protected_storage_key,
};
pub use traits::Storage;
