//! Process-level CLI tests for the `create-bucket` subcommand.
//! These run without AWS credentials or network access.

use std::process::{Command, Stdio};

fn s7cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_s7cmd"))
}

fn run(cmd: &mut Command) -> (bool, String, String, Option<i32>) {
    let output = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to spawn s7cmd binary");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (
        output.status.success(),
        stdout,
        stderr,
        output.status.code(),
    )
}

#[test]
fn help_succeeds_and_lists_option_groups() {
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["create-bucket", "--help"]));
    assert!(ok, "create-bucket --help must succeed");
    assert!(stdout.contains("AWS Configuration"));
    assert!(stdout.contains("Retry Options"));
    assert!(stdout.contains("Timeout Options"));
}

#[test]
fn help_mentions_tagging_option() {
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["create-bucket", "--help"]));
    assert!(ok);
    assert!(
        stdout.contains("--tagging"),
        "expected --tagging in help output: {stdout}"
    );
}

#[test]
fn help_mentions_if_not_exists_option() {
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["create-bucket", "--help"]));
    assert!(ok);
    assert!(
        stdout.contains("--if-not-exists"),
        "expected --if-not-exists in help output: {stdout}"
    );
}

#[test]
fn missing_target_exits_non_zero() {
    let (ok, _stdout, stderr, code) = run(s7cmd().arg("create-bucket"));
    assert!(!ok);
    assert_eq!(code, Some(2), "clap missing-arg should exit 2");
    assert!(stderr.to_lowercase().contains("required") || stderr.to_lowercase().contains("usage"));
}

// NOTE: s3util-rs has a `auto_complete_shell_short_circuits_without_target`
// test for each subcommand. s7cmd intentionally hides the per-subcommand
// `--auto-complete-shell` flag (see src/cli.rs cli_command()) and exposes
// only the top-level `s7cmd --auto-complete-shell <SHELL>` form, which is
// covered by tests/cli_help.rs::top_level_auto_complete_shell_runs.

#[test]
fn target_access_key_without_secret_exits_non_zero() {
    let (ok, _stdout, stderr, code) = run(s7cmd().args([
        "create-bucket",
        "s3://example",
        "--target-access-key",
        "AKIA",
    ]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(2),
        "clap missing-arg should exit 2; stderr: {stderr}"
    );
    assert!(
        stderr.to_lowercase().contains("required")
            || stderr.to_lowercase().contains("--target-secret-access-key")
    );
}

#[test]
fn target_no_sign_request_conflicts_with_target_profile() {
    let (ok, _stdout, stderr, code) = run(s7cmd().args([
        "create-bucket",
        "s3://example",
        "--target-no-sign-request",
        "--target-profile",
        "default",
    ]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(2),
        "clap conflict should exit 2; stderr: {stderr}"
    );
    assert!(
        stderr.to_lowercase().contains("cannot be used")
            || stderr.to_lowercase().contains("conflict"),
        "expected clap conflict message; got: {stderr}"
    );
}
