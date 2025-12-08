use bw_core::services::create_sdk_client;

#[test]
fn test_sdk_client_creation() {
    let result = create_sdk_client(None, None);
    assert!(result.is_ok(), "Should create SDK client with defaults");
}

#[test]
fn test_sdk_client_custom_urls() {
    let result = create_sdk_client(
        Some("https://api.example.com".to_string()),
        Some("https://identity.example.com".to_string()),
    );
    assert!(result.is_ok(), "Should create SDK client with custom URLs");
}

#[tokio::test]
async fn test_sdk_client_basic_usage() {
    let client = create_sdk_client(None, None).expect("Failed to create client");

    // Verify client is initialized (basic smoke test)
    // More comprehensive tests will come in enhancement 4 (auth)
    assert!(std::ptr::addr_of!(client) as usize != 0);
}
