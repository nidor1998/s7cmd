//! Process-level CLI tests for the `head-object` subcommand.
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
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["head-object", "--help"]));
    assert!(ok, "head-object --help must succeed");
    assert!(stdout.contains("AWS Configuration"));
    assert!(stdout.contains("Retry Options"));
    assert!(stdout.contains("Timeout Options"));
}

#[test]
fn missing_positional_exits_2() {
    let (ok, _stdout, stderr, code) = run(s7cmd().arg("head-object"));
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
        "head-object",
        "s3://bucket/key",
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
fn bucket_only_path_no_key_exits_1() {
    let (ok, _stdout, stderr, code) = run(s7cmd().args(["head-object", "s3://bucket"]));
    assert!(!ok);
    assert_eq!(code, Some(1), "bucket-only path should exit 1 (validation)");
    assert!(
        !stderr.is_empty(),
        "should have an error message on stderr; got empty"
    );
}

#[test]
fn target_no_sign_request_conflicts_with_target_profile() {
    let (ok, _stdout, stderr, code) = run(s7cmd().args([
        "head-object",
        "s3://bucket/key",
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

#[test]
fn help_mentions_enable_additional_checksum() {
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["head-object", "--help"]));
    assert!(ok);
    assert!(
        stdout.contains("enable-additional-checksum"),
        "help should list --enable-additional-checksum; got: {stdout}"
    );
}

#[test]
fn help_mentions_source_version_id() {
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["head-object", "--help"]));
    assert!(ok);
    assert!(
        stdout.contains("source-version-id"),
        "help should list --source-version-id; got: {stdout}"
    );
}
