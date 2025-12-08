use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Device information for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device type code (8 = CLI)
    pub device_type: u8,
    /// Device name identifier
    pub device_name: String,
    /// Persistent device identifier
    pub device_identifier: Uuid,
}

impl DeviceInfo {
    /// Create new device info with CLI defaults
    ///
    /// Device ID should be loaded from storage if available,
    /// or generated and persisted for first-time use.
    pub fn new(device_id: Option<Uuid>) -> Self {
        Self {
            device_type: 8, // CLI device type
            device_name: "rust-cli".to_string(),
            device_identifier: device_id.unwrap_or_else(Uuid::new_v4),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_creation() {
        let device = DeviceInfo::new(None);
        assert_eq!(device.device_type, 8);
        assert_eq!(device.device_name, "rust-cli");
    }

    #[test]
    fn test_device_info_with_existing_id() {
        let existing_id = Uuid::new_v4();
        let device = DeviceInfo::new(Some(existing_id));
        assert_eq!(device.device_identifier, existing_id);
    }
}
