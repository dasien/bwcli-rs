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
    /// Suppress human and raw output entirely; see [`Response::silent`].
    /// Not part of the wire format — `--response` output is unaffected.
    #[serde(skip)]
    pub silent: bool,
    /// What `--raw` should print instead of `data`; see [`Response::with_raw`].
    /// Not part of the wire format.
    #[serde(skip)]
    pub raw: Option<String>,
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
            silent: false,
            raw: None,
        })
    }

    /// Create a success response with just a message
    pub fn success_message(message: impl Into<String>) -> Self {
        Response::Success(SuccessResponse {
            success: true,
            data: None,
            message: Some(message.into()),
            silent: false,
            raw: None,
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
            silent: false,
            raw: None,
        })
    }

    /// A success that prints nothing.
    ///
    /// For commands that have already written their payload to stdout and must
    /// not add anything after it — `bw export` without `--output`, where the
    /// export document is the whole of stdout.
    pub fn silent() -> Self {
        Response::Success(SuccessResponse {
            success: true,
            data: None,
            message: None,
            silent: true,
            raw: None,
        })
    }

    /// Give `--raw` something different to print.
    ///
    /// Some commands are prose for a human and one value for a script: `unlock`
    /// explains how to export `BW_SESSION` but, under `--raw`, must print only
    /// the key so `export BW_SESSION=$(bw unlock --raw)` works. This mirrors the
    /// TypeScript CLI's `MessageResponse.raw`.
    pub fn with_raw(self, raw: impl Into<String>) -> Self {
        match self {
            Response::Success(s) => Response::Success(SuccessResponse {
                raw: Some(raw.into()),
                ..s
            }),
            error => error,
        }
    }

    /// Create a success response with JSON data
    /// Convenience method for when you already have a JSON Value
    pub fn success_json(data: Value) -> Self {
        Response::Success(SuccessResponse {
            success: true,
            data: Some(data),
            message: None,
            silent: false,
            raw: None,
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
