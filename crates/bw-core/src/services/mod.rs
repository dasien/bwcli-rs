mod container;
mod crypto;
pub mod sdk_session;
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
pub use send_repository::JsonSendRepository;
pub use sdk::{
    Client, ClientSettings, DeviceType, create_sdk_client, create_sdk_client_with_state,
    get_device_type,
};
