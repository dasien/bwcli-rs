use super::Response;
use crate::GlobalArgs;
use serde_json::{Value, json};

/// Print a successful response according to `--response`, `--pretty`,
/// `--quiet` and `--raw`.
pub fn print_response(response: Response, args: &GlobalArgs) {
    if args.quiet {
        return;
    }

    if args.response {
        print_json(&response, args.pretty);
        return;
    }

    // A payload already went to stdout; adding to it would corrupt it.
    if response.silent {
        return;
    }

    if args.raw {
        print_raw(&response);
        return;
    }

    print_human(&response);
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
/// emits `chalk.redBright(response.message)` and nothing more, so no existing
/// script can be relying on a prefix — printing one is a parity divergence, not
/// a courtesy.
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

fn print_json(response: &Response, pretty: bool) {
    let rendered = if pretty {
        serde_json::to_string_pretty(response)
    } else {
        serde_json::to_string(response)
    };
    match rendered {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error formatting response: {}", e),
    }
}

fn print_raw(response: &Response) {
    // An explicit raw form wins: `--raw` is for machine consumption, and the
    // human payload may be prose wrapped around the value.
    if let Some(raw) = &response.raw {
        println!("{}", raw);
    } else if let Some(data) = &response.data {
        print_raw_value(data);
    } else if let Some(msg) = &response.message {
        println!("{}", msg);
    }
}

fn print_raw_value(value: &Value) {
    match value {
        Value::String(s) => println!("{}", s),
        Value::Number(n) => println!("{}", n),
        Value::Bool(b) => println!("{}", b),
        Value::Null => println!("null"),
        Value::Array(arr) => {
            for item in arr {
                print_raw_value(item);
            }
        }
        Value::Object(_) => {
            // For objects, print compact JSON
            if let Ok(json) = serde_json::to_string(value) {
                println!("{}", json);
            }
        }
    }
}

fn print_human(response: &Response) {
    if let Some(data) = &response.data {
        match data {
            // A string payload is printed bare, never JSON-encoded. This is what
            // the TypeScript CLI does (`base-program.ts`: for a `string`
            // response, `out = data`), and it is the difference between
            // `bw get password <id>` yielding `hunter2` and yielding
            // `"hunter2"` — quotes and all — inside `$(...)`. It is also what
            // makes `bw encode | bw move` work.
            Value::String(text) => println!("{}", text),
            // Everything else is a document, and pretty-printing it is the point
            // of human mode.
            other => match serde_json::to_string_pretty(other) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("Error formatting response: {}", e),
            },
        }
    } else if let Some(msg) = &response.message {
        println!("{}", msg);
    } else {
        println!("Success");
    }
}
