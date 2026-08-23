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

/// `bw move <id>` with no folder removes the item from all folders. The bulk
/// move endpoint takes `None` for exactly this, but the argument used to be
/// required, so there was no way to ask for it.
#[test]
fn move_accepts_a_missing_folder() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(["move", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("[FOLDER_ID]"));
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
