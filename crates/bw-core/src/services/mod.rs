mod container;
mod crypto;
pub mod key_service;
pub mod send_repository;
mod sdk;

// Storage module
pub mod storage;

// API client module
pub mod api;

// Authentication module
pub mod auth;

// Vault module
pub mod vault;

// Send module
pub mod send;

// Import/Export module
pub mod import_export;

pub use container::ServiceContainer;
pub use crypto::{decrypt_user_key, derive_master_key, hash_password_for_auth};
pub use key_service::{KeyService, KeyServiceError};
pub use send_repository::JsonSendRepository;
pub use sdk::{
    Client, ClientSettings, DeviceType, create_sdk_client, create_sdk_client_with_state, create_sdk_client_with_tokens,
    get_device_type,
};
