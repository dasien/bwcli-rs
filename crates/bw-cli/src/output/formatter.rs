use super::CommandOutput;
use crate::GlobalArgs;
use serde_json::{Value, json};
use std::io::Write;

/// Print a command's output. The only place a success payload reaches stdout.
pub fn print_output(output: CommandOutput, args: &GlobalArgs) {
    if args.quiet {
        return;
    }

    // Bytes are the payload itself, so they bypass formatting entirely — there
    // is no meaningful JSON wrapping of an arbitrary binary file. `--response`
    // together with `--raw` on an attachment is contradictory; the bytes win,
    // because that is what the caller redirected stdout for.
    if let CommandOutput::Bytes(data) = &output {
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(data);
        let _ = stdout.flush();
        return;
    }

    if args.response {
        print_wire_document(&output, args.pretty);
        return;
    }

    match output {
        // Bare in both modes. A string payload is never JSON-encoded: it is the
        // difference between `bw get password x` yielding `hunter2` and yielding
        // `"hunter2"` — quotes and all — inside `$(...)`. See BUGLIST C26.
        CommandOutput::Plain(text) => println!("{}", text),

        CommandOutput::Message { human, raw } => {
            // An explicit raw form wins: `--raw` is for machine consumption, and
            // the human text may be prose wrapped around the value (C28).
            if args.raw {
                println!("{}", raw.unwrap_or(human));
            } else {
                println!("{}", human);
            }
        }

        // Compact unless `--pretty`, and **`--raw` has no say in it**. That is
        // what the TypeScript CLI does: `base-program.ts`'s `getJson` switches on
        // `BW_PRETTY` alone, and its `--pretty` help — "JSON is tabbed with two
        // spaces" — says what the default is not. See BUGLIST C36.
        //
        // Rendering the whole value as one document also fixes C39: `--raw` used
        // to walk an array and print each element on its own line, so
        // `bw list items --raw` emitted twelve JSON objects rather than one JSON
        // array, and `| jq length` failed on output that was not a document at
        // all.
        CommandOutput::Object(value) => print_json(&value, args.pretty),

        CommandOutput::Bytes(_) => unreachable!("handled above"),
    }
}

/// Build the `--response` document.
///
/// Shape matches the TypeScript CLI's `Response`: `success` plus at most one of
/// `data` / `message`. Constructing it here rather than in each command is what
/// stops the wire format drifting between commands.
fn print_wire_document(output: &CommandOutput, pretty: bool) {
    let document = match output {
        CommandOutput::Plain(text) => json!({ "success": true, "data": text }),
        CommandOutput::Object(value) => json!({ "success": true, "data": value }),
        CommandOutput::Message { human, .. } => json!({ "success": true, "message": human }),
        CommandOutput::Bytes(_) => json!({ "success": true }),
    };

    let rendered = if pretty {
        serde_json::to_string_pretty(&document)
    } else {
        serde_json::to_string(&document)
    };
    match rendered {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error formatting response: {}", e),
    }
}

/// Print a command failure.
///
/// The single place a failure is rendered. Handlers return `Err` and never
/// format an error themselves, so `--response`, `--quiet` and the exit code
/// cannot disagree with each other.
///
/// Matches the TypeScript CLI's `base-program.ts:24-32`:
///
/// - `--quiet` prints nothing at all
/// - `--response` writes the `{"success":false,...}` document to **stdout**,
///   because it is the command's machine-readable answer
/// - otherwise the **bare message** goes to stderr
///
/// That last case is deliberately not prefixed with `Error:`. The TypeScript CLI
/// emits `chalk.redBright(response.message)` and nothing more, so printing a
/// prefix is a parity divergence, not a courtesy (`BUGLIST.md` C33).
pub fn print_error(error: &anyhow::Error, args: &GlobalArgs) {
    if args.quiet {
        return;
    }

    // `{:#}` renders the whole `anyhow` context chain, not just the outermost
    // error, so a wrapped cause is not lost.
    let message = format!("{:#}", error);

    if args.response {
        let document = json!({ "success": false, "message": message });
        let rendered = if args.pretty {
            serde_json::to_string_pretty(&document)
        } else {
            serde_json::to_string(&document)
        };
        match rendered {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Error formatting response: {}", e),
        }
        return;
    }

    eprintln!("{}", message);
}

/// Serialize a value and print it, honouring `--pretty` only.
fn print_json(value: &Value, pretty: bool) {
    let rendered = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };
    match rendered {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error formatting response: {}", e),
    }
}
