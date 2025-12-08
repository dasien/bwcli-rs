use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Device information for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device type code (6=Windows, 7=macOS, 8=Linux)
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
            device_type: get_device_type(),
            device_name: get_device_name(),
            device_identifier: device_id.unwrap_or_else(Uuid::new_v4),
        }
    }
}

/// Get device type code for current platform
/// Based on Bitwarden's DeviceType enum for CLI:
/// - 23 = WindowsCLI
/// - 24 = MacOsCLI
/// - 25 = LinuxCLI
fn get_device_type() -> u8 {
    #[cfg(target_os = "windows")]
    {
        23
    }
    #[cfg(target_os = "macos")]
    {
        24
    }
    #[cfg(target_os = "linux")]
    {
        25
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        25 // Default to Linux CLI
    }
}

/// Get device name string
fn get_device_name() -> String {
    let os = std::env::consts::OS;
    format!("Bitwarden CLI on {}", os)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_creation() {
        let device = DeviceInfo::new(None);
        // Device type should match current platform (CLI device types)
        #[cfg(target_os = "windows")]
        assert_eq!(device.device_type, 23); // WindowsCLI
        #[cfg(target_os = "macos")]
        assert_eq!(device.device_type, 24); // MacOsCLI
        #[cfg(target_os = "linux")]
        assert_eq!(device.device_type, 25); // LinuxCLI
        // Device name should contain OS
        assert!(device.device_name.contains("Bitwarden CLI"));
    }

    #[test]
    fn test_device_info_with_existing_id() {
        let existing_id = Uuid::new_v4();
        let device = DeviceInfo::new(Some(existing_id));
        assert_eq!(device.device_identifier, existing_id);
    }
}
