use serde::Serialize;
use serde_json::Value;

mod formatter;
pub use formatter::{print_error, print_response};

/// A command's successful output.
///
/// **There is no error variant, and that is the point.** A command reports
/// failure by returning `Err` from its handler; success is the only thing this
/// type can express.
///
/// It used to be an enum with an `Error` variant, so a handler signalled failure
/// with `Ok(Response::error(..))` — an `Ok` that meant "it failed". `main`
/// matched on `Result::Err` alone and returned `ExitCode::SUCCESS` for the whole
/// `Ok` arm, so every failing command exited 0 (`BUGLIST.md` C31). That bug is
/// fixed, but making the error variant unrepresentable is what stops the 65th
/// call site from reintroducing it.
#[derive(Debug, Clone, Serialize)]
pub struct Response {
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

impl Response {
    fn new(data: Option<Value>, message: Option<String>) -> Self {
        Response {
            success: true,
            data,
            message,
            silent: false,
            raw: None,
        }
    }

    /// Create a success response with data
    pub fn success(data: impl Serialize) -> Self {
        Self::new(serde_json::to_value(data).ok(), None)
    }

    /// Create a success response with just a message
    pub fn success_message(message: impl Into<String>) -> Self {
        Self::new(None, Some(message.into()))
    }

    /// Create a success response with raw string data
    /// Used for commands that output plain text (like generate, encode)
    pub fn success_raw(data: impl Into<String>) -> Self {
        Self::new(Some(Value::String(data.into())), None)
    }

    /// A success that prints nothing.
    ///
    /// For commands that have already written their payload to stdout and must
    /// not add anything after it — `bw export` without `--output`, where the
    /// export document is the whole of stdout.
    pub fn silent() -> Self {
        Response {
            silent: true,
            ..Self::new(None, None)
        }
    }

    /// Give `--raw` something different to print.
    ///
    /// Some commands are prose for a human and one value for a script: `unlock`
    /// explains how to export `BW_SESSION` but, under `--raw`, must print only
    /// the key so `export BW_SESSION=$(bw unlock --raw)` works. This mirrors the
    /// TypeScript CLI's `MessageResponse.raw`.
    pub fn with_raw(self, raw: impl Into<String>) -> Self {
        Response {
            raw: Some(raw.into()),
            ..self
        }
    }

    /// Create a success response with JSON data
    /// Convenience method for when you already have a JSON Value
    pub fn success_json(data: Value) -> Self {
        Self::new(Some(data), None)
    }
}

/// What a command handler returns: output on success, an error on failure.
///
/// Failure has exactly one representation. See [`Response`].
pub type CommandResult = anyhow::Result<Response>;
