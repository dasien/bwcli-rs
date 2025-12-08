// Send service module
//
// This module provides Send operations including:
// - Creating and managing Sends
// - Encrypting/decrypting Send content
// - Accessing public Sends
//
// TODO: Full implementation requires:
// 1. Send encryption service using Bitwarden SDK or rust-crypto
// 2. Send API client methods
// 3. Send service business logic
// 4. Key management for Send encryption

mod errors;

pub use errors::SendError;

// Re-export Send models for convenience
pub use crate::models::send::{Send, SendAccess, SendType};
