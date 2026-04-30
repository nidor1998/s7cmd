//! Process-level CLI tests for the `get-object-tagging` subcommand.
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
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["get-object-tagging", "--help"]));
    assert!(ok, "get-object-tagging --help must succeed");
    assert!(stdout.contains("AWS Configuration"));
    assert!(stdout.contains("Retry Options"));
    assert!(stdout.contains("Timeout Options"));
}

#[test]
fn missing_positional_exits_2() {
    let (ok, _stdout, stderr, code) = run(s7cmd().arg("get-object-tagging"));
    assert!(!ok);
    assert_eq!(code, Some(2), "clap missing-arg should exit 2");
    assert!(stderr.to_lowercase().contains("required") || stderr.to_lowercase().contains("usage"));
}

#[test]
fn bucket_only_path_no_key_exits_1() {
    let (ok, _stdout, stderr, code) = run(s7cmd().args(["get-object-tagging", "s3://bucket"]));
    assert!(!ok);
    assert_eq!(code, Some(1), "bucket-only path should exit 1 (validation)");
    assert!(
        !stderr.is_empty(),
        "should have an error message on stderr; got empty"
    );
}

// NOTE: s3util-rs's auto_complete_shell_short_circuits_without_target test
// is omitted — s7cmd hides the per-subcommand --auto-complete-shell flag
// and exposes only the top-level form (covered by tests/cli_help.rs).

#[test]
fn target_access_key_without_secret_exits_non_zero() {
    let (ok, _stdout, stderr, code) = run(s7cmd().args([
        "get-object-tagging",
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
fn help_mentions_source_version_id() {
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["get-object-tagging", "--help"]));
    assert!(ok);
    assert!(
        stdout.contains("source-version-id"),
        "help should list --source-version-id; got: {stdout}"
    );
}
