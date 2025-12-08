use super::Response;
use crate::GlobalArgs;
use serde_json::Value;

/// Print response according to global args (--response, --pretty, --quiet, --raw)
pub fn print_response(response: Response, args: &GlobalArgs) {
    // Quiet mode: suppress all output
    if args.quiet {
        return;
    }

    // Response mode: JSON output
    if args.response {
        print_json(&response, args.pretty);
        return;
    }

    // Raw mode: minimal output
    if args.raw {
        print_raw(&response);
        return;
    }

    // Default: human-readable output
    print_human(&response);
}

fn print_json(response: &Response, pretty: bool) {
    if pretty {
        match serde_json::to_string_pretty(response) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Error formatting response: {}", e),
        }
    } else {
        match serde_json::to_string(response) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Error formatting response: {}", e),
        }
    }
}

fn print_raw(response: &Response) {
    match response {
        Response::Success(s) => {
            if let Some(data) = &s.data {
                print_raw_value(data);
            } else if let Some(msg) = &s.message {
                println!("{}", msg);
            }
        }
        Response::Error(e) => {
            eprintln!("{}", e.message);
        }
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
    match response {
        Response::Success(s) => {
            if let Some(data) = &s.data {
                // Pretty-print data by default in human mode
                match serde_json::to_string_pretty(data) {
                    Ok(json) => println!("{}", json),
                    Err(e) => eprintln!("Error formatting response: {}", e),
                }
            } else if let Some(msg) = &s.message {
                println!("{}", msg);
            } else {
                println!("Success");
            }
        }
        Response::Error(e) => {
            eprintln!("Error: {}", e.message);
        }
    }
}
