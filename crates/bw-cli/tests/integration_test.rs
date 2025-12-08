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
        .stdout(predicate::str::contains(r#"{"success":false"#))
        .stdout(predicate::str::contains("Not yet implemented"));
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
        .stdout(predicate::str::contains("  \"success\": false"));
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
