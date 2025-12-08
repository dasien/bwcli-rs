mod container;
mod crypto;
mod sdk;

// Storage module
pub mod storage;

// API client module
pub mod api;

// Authentication module
pub mod auth;

// Vault module
pub mod vault;

// Generator module
pub mod generator;

// Send module
pub mod send;

// Import/Export module
pub mod import_export;

pub use container::ServiceContainer;
pub use crypto::{decrypt_user_key, derive_master_key, hash_password_for_auth};
pub use sdk::{Client, ClientSettings, DeviceType, create_sdk_client, get_device_type};
