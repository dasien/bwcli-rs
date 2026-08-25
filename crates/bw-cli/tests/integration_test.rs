use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "A secure and free password manager",
        ))
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_status_response_format() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(&["status", "--response"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r#"{"success":true"#))
        .stdout(predicate::str::contains("status"));
}

#[test]
fn test_quiet_flag() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(&["status", "--quiet"]);

    cmd.assert().success().stdout(predicate::str::is_empty());
}

#[test]
fn test_pretty_flag() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(&["status", "--response", "--pretty"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("  \"success\": true"));
}

#[test]
fn test_env_var_session() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.env("BW_SESSION", "test_session_key")
        .args(&["status", "--response"]);

    // Should accept session from env var without error
    cmd.assert().success();
}

#[test]
fn test_env_var_quiet() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.env("BW_QUIET", "true").arg("status");

    cmd.assert().success().stdout(predicate::str::is_empty());
}

#[test]
fn test_all_auth_commands_exist() {
    for cmd_name in &["login", "logout", "lock", "unlock"] {
        let mut cmd = Command::cargo_bin("bw").unwrap();
        cmd.args(&[cmd_name, "--help"]);
        cmd.assert().success();
    }
}

#[test]
fn test_all_vault_commands_exist() {
    for cmd_name in &[
        "list", "get", "create", "edit", "delete", "restore", "move", "confirm",
    ] {
        let mut cmd = Command::cargo_bin("bw").unwrap();
        cmd.args(&[cmd_name, "--help"]);
        cmd.assert().success();
    }
}

#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.arg("nonexistent");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

/// `bw restore item <id>` is the TypeScript CLI's shape. Ours took the id
/// directly, so any script written against the TS CLI failed with a clap usage
/// error before touching the vault.
#[test]
fn restore_takes_an_object_argument() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(["restore", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("item"));
}

/// The bare `bw restore <id>` form must now be rejected rather than silently
/// meaning something different from the TS CLI.
#[test]
fn restore_rejects_a_bare_id() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(["restore", "11111111-1111-4111-8111-111111111111"]);

    cmd.assert().failure();
}

/// `bw move` is the TypeScript CLI's org-share command:
/// `bw move <id> <organizationId> [encodedJson]`.
#[test]
fn move_shares_into_an_organization() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(["move", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("<ORGANIZATION_ID>"))
        .stdout(predicate::str::contains("[ENCODED_JSON]"))
        .stdout(predicate::str::contains("[FOLDER_ID]").not());
}

/// `share` is the TypeScript CLI's deprecated alias for `move`.
#[test]
fn share_is_an_alias_for_move() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(["share", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("<ORGANIZATION_ID>"));
}

/// Folder moves live under their own name now, with the folder optional so an
/// item can be removed from all folders — which `move_many(ids, None)` supports.
#[test]
fn move_to_folder_accepts_a_missing_folder() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(["move-to-folder", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("[FOLDER_ID]"));
}

/// The old two-positional folder form must not silently keep working under
/// `move`: that is the collision this rename exists to remove. A folder id in
/// the organization slot has to fail, not be accepted as an organization.
#[test]
fn move_rejects_a_bare_item_and_folder_pair() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args([
        "move",
        "11111111-1111-4111-8111-111111111111",
        "22222222-2222-4222-8222-222222222222",
    ]);

    // Reaches the vault (locked, or an invalid-collection error) rather than
    // being parsed as a folder move.
    cmd.assert()
        .stdout(predicate::str::contains("folder").not())
        .stderr(predicate::str::contains("folder").not());
}

/// `bw export` without `--output` must put the document on stdout and nothing
/// else, or redirecting it produces a file no parser accepts. Checked here on the
/// locked-vault error path, which is as far as this can get without credentials:
/// the point is that the status line never shares stdout with a payload.
#[test]
fn export_keeps_status_off_stdout() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(["export", "--format", "json"]);

    cmd.assert()
        .stdout(predicate::str::contains("Exported").not())
        .stdout(predicate::str::contains("item(s)").not());
}

/// The TypeScript CLI's object name is `organization` (`getObjects` in
/// `vault.program.ts`); ours shipped as `org` only, so TS-compatible scripts hit
/// a clap error. Both must work now.
#[test]
fn get_accepts_the_typescript_organization_object() {
    for object in ["organization", "org"] {
        let mut cmd = Command::cargo_bin("bw").unwrap();
        cmd.args(["get", object, "--help"]);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Usage:"));
    }
}

/// The TypeScript CLI's `encode` takes no argument at all — "Base 64 encode
/// stdin" — and its own docs pipe it into `create`, `edit` and `move`. Ours
/// required a positional, so the documented pipeline could not run.
#[test]
fn encode_reads_stdin() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.arg("encode").write_stdin(r#"["abc"]"#);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("WyJhYmMiXQ=="));
}

/// The positional form stays, since this CLI shipped with it as the only form.
#[test]
fn encode_still_accepts_an_argument() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(["encode", r#"["abc"]"#]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("WyJhYmMiXQ=="));
}

/// A string payload must be printed bare, not JSON-encoded. The TypeScript CLI
/// does this (`base-program.ts`: for a `string` response, `out = data`), and it
/// is the difference between `PASS=$(bw get password <id>)` yielding `hunter2`
/// and yielding `"hunter2"` — quotes included. It is also what makes
/// `bw encode | bw move` work at all.
#[test]
fn string_output_is_not_json_quoted() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.arg("encode").write_stdin("abc");

    cmd.assert()
        .success()
        .stdout(predicate::eq("YWJj\n"));
}

/// `bw generate` is the other heavily scripted string output.
#[test]
fn generate_output_is_not_json_quoted() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.arg("generate");

    cmd.assert()
        .success()
        .stdout(predicate::str::starts_with("\"").not());
}

/// Documents must still be pretty-printed JSON; the bare-string rule is only for
/// string payloads.
#[test]
fn document_output_is_still_json() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::starts_with("{"));
}

/// `bw unlock --raw` must print only the session key. The TypeScript CLI's help
/// says so outright ("Pass `--raw` option to only return the session key"), and
/// `export BW_SESSION=$(bw unlock --raw)` is the documented way to use it. Ours
/// printed the whole instructional blurb, so the capture was unusable.
///
/// Checked on the not-logged-in path, which is as far as this goes without
/// credentials: the point is that the blurb is no longer what `--raw` carries.
#[test]
fn unlock_raw_does_not_emit_the_instructional_blurb() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(["unlock", "--raw", "--nointeraction"]);

    cmd.assert()
        .stdout(predicate::str::contains("BW_SESSION").not())
        .stdout(predicate::str::contains("export").not());
}

/// A handled failure must exit non-zero.
///
/// Commands signal a failure by returning `Ok(Response::error(..))`, so the
/// `Result` is `Ok` and only the *response* says it failed. `main` used to
/// return `ExitCode::SUCCESS` for the whole `Ok` arm, which meant every failing
/// command exited 0 and `bw get item nope && deploy` ran `deploy`. No test
/// caught it because none asserted an exit code on a failure path.
#[test]
fn a_locked_vault_failure_exits_nonzero() {
    // `get item` needs an unlocked vault; with no session this fails early and
    // needs no network or state.
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.env_remove("BW_SESSION")
        .env_remove("BW_CLEANEXIT")
        .args(["get", "item", "nonexistent"]);

    cmd.assert().failure();
}

#[test]
fn an_unknown_template_type_exits_nonzero() {
    // `get template` needs no session, so this exercises the `Ok(error)` path
    // specifically rather than the pre-flight session check.
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.env_remove("BW_CLEANEXIT")
        .args(["get", "template", "not-a-real-template"]);

    cmd.assert().failure();
}

#[test]
fn cleanexit_forces_success_on_a_handled_failure() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.env_remove("BW_CLEANEXIT")
        .args(["get", "template", "not-a-real-template", "--cleanexit"]);

    cmd.assert().success();
}
