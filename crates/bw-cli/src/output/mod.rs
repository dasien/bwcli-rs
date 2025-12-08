use serde::{Deserialize, Serialize};
use serde_json::Value;

mod formatter;
pub use formatter::print_response;

/// Response types matching TypeScript CLI Response class
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    Success(SuccessResponse),
    Error(ErrorResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub message: String,
}

impl Response {
    /// Create a success response with data
    pub fn success(data: impl Serialize) -> Self {
        Response::Success(SuccessResponse {
            success: true,
            data: serde_json::to_value(data).ok(),
            message: None,
        })
    }

    /// Create a success response with just a message
    pub fn success_message(message: impl Into<String>) -> Self {
        Response::Success(SuccessResponse {
            success: true,
            data: None,
            message: Some(message.into()),
        })
    }

    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Response::Error(ErrorResponse {
            success: false,
            message: message.into(),
        })
    }

    /// Create a success response with raw string data
    /// Used for commands that output plain text (like generate, encode)
    pub fn success_raw(data: impl Into<String>) -> Self {
        Response::Success(SuccessResponse {
            success: true,
            data: Some(Value::String(data.into())),
            message: None,
        })
    }

    /// Create a success response with JSON data
    /// Convenience method for when you already have a JSON Value
    pub fn success_json(data: Value) -> Self {
        Response::Success(SuccessResponse {
            success: true,
            data: Some(data),
            message: None,
        })
    }

    /// Check if this is a success response
    pub fn is_success(&self) -> bool {
        matches!(self, Response::Success(_))
    }

    /// Extract data as a specific type
    pub fn data<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        match self {
            Response::Success(s) => s
                .data
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            Response::Error(_) => None,
        }
    }
}
