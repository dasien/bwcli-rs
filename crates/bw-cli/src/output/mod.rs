use serde::Serialize;
use serde_json::Value;

mod formatter;
pub use formatter::{print_error, print_output};

/// What a command produced.
///
/// **Commands never write to stdout.** They return one of these and the renderer
/// in [`formatter`] does all the printing, so `--raw`, `--response`, `--pretty`
/// and `--quiet` are interpreted in exactly one place.
///
/// This is an enum rather than a struct of options on purpose. The previous
/// `Response` carried `data`, `message`, `silent` and `raw` as four independent
/// `Option`s, most combinations of which were meaningless — and the meaningless
/// ones were where the bugs lived:
///
/// - **C26** — a string payload was JSON-quoted, so `$(bw get password x)`
///   captured `"hunter2"` with the quotes.
/// - **C28** — `unlock --raw` printed prose instead of just the session key.
/// - **C35** — `data: None, message: Some("")` plus a hand-rolled `println!` in
///   the command emitted a *trailing blank line* under `--raw`.
///
/// There is deliberately no error variant; failure is `Err`. See
/// [`crate::output::CommandResult`] and `BUGLIST.md` C31.
pub enum CommandOutput {
    /// A string that *is* the payload — `get password`, `generate`, `encode`.
    ///
    /// Printed bare in both human and raw mode, never JSON-encoded. That
    /// equivalence is the point: a command cannot make the two disagree, which
    /// is how C26 and C35 happened.
    Plain(String),

    /// Structured data — `get item`, `list items`.
    ///
    /// Pretty-printed for humans, compact under `--raw`.
    Object(Value),

    /// Prose for a human, optionally with a different value for scripts.
    ///
    /// `unlock` explains how to export `BW_SESSION`, but under `--raw` must emit
    /// only the key so `export BW_SESSION=$(bw unlock --raw)` works. That is
    /// C28, and `raw` is where its fix lives.
    Message { human: String, raw: Option<String> },

    /// Arbitrary bytes straight to stdout — an attachment downloaded without
    /// `--output`, or `bw export` without one.
    ///
    /// Also covers "the command's payload is already the whole of stdout":
    /// `bw export --format json > vault.json` must emit the document and nothing
    /// else, or no JSON parser will accept the file.
    ///
    /// Stays bytes and must never become a `String`: attachment contents are
    /// arbitrary binary, and the live round-trip test covers a 4 KiB random file
    /// precisely to prove nothing on this path treats it as text.
    Bytes(Vec<u8>),

}

impl CommandOutput {
    /// Structured or string output, whichever the value turns out to be.
    ///
    /// A value that serializes to a JSON string becomes [`Self::Plain`] so it
    /// prints bare — that is the TypeScript CLI's behaviour (`base-program.ts`:
    /// for a `string` response, `out = data`) and what makes
    /// `bw encode | bw move` work.
    pub fn success(data: impl Serialize) -> Self {
        // Serialization failure is not reachable for the derived types used
        // here; `Null` keeps it non-panicking rather than silently claiming
        // success with no payload, as the previous `.ok()` did.
        match serde_json::to_value(data) {
            Ok(Value::String(text)) => CommandOutput::Plain(text),
            Ok(value) => CommandOutput::Object(value),
            Err(_) => CommandOutput::Object(Value::Null),
        }
    }

    /// A human-readable message with no machine payload.
    pub fn success_message(message: impl Into<String>) -> Self {
        CommandOutput::Message {
            human: message.into(),
            raw: None,
        }
    }

    /// A string that is the payload. See [`Self::Plain`].
    pub fn success_raw(data: impl Into<String>) -> Self {
        CommandOutput::Plain(data.into())
    }

    /// Already-built JSON. See [`Self::Object`].
    pub fn success_json(data: Value) -> Self {
        CommandOutput::Object(data)
    }

    /// Raw bytes to stdout. See [`Self::Bytes`].
    pub fn bytes(data: Vec<u8>) -> Self {
        CommandOutput::Bytes(data)
    }

    /// Give `--raw` something different to print than the human text.
    ///
    /// Only meaningful on [`Self::Message`]; on any other variant the human and
    /// raw forms are already the same thing, so this is a no-op rather than a
    /// silently ignored field.
    pub fn with_raw(self, raw: impl Into<String>) -> Self {
        match self {
            CommandOutput::Message { human, .. } => CommandOutput::Message {
                human,
                raw: Some(raw.into()),
            },
            other => other,
        }
    }
}

/// What a command handler returns: output on success, an error on failure.
///
/// Failure has exactly one representation. See [`CommandOutput`].
pub type CommandResult = anyhow::Result<CommandOutput>;
