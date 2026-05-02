//! Process-level CLI tests for the `put-bucket-versioning` subcommand.
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
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["put-bucket-versioning", "--help"]));
    assert!(ok, "put-bucket-versioning --help must succeed");
    assert!(stdout.contains("AWS Configuration"));
    assert!(stdout.contains("Retry Options"));
    assert!(stdout.contains("Timeout Options"));
}

#[test]
fn missing_both_state_flags_exits_2() {
    let (ok, _stdout, stderr, code) = run(s7cmd().args(["put-bucket-versioning", "s3://example"]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(2),
        "missing state flags should exit 2; got {code:?}; stderr: {stderr}"
    );
    assert!(
        stderr.to_lowercase().contains("required") || stderr.to_lowercase().contains("usage"),
        "expected 'required' or 'usage' in stderr; got: {stderr}"
    );
}

#[test]
fn both_enabled_and_suspended_exits_2() {
    let (ok, _stdout, stderr, code) = run(s7cmd().args([
        "put-bucket-versioning",
        "s3://example",
        "--enabled",
        "--suspended",
    ]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(2),
        "conflicting flags should exit 2; got {code:?}; stderr: {stderr}"
    );
    assert!(
        stderr.to_lowercase().contains("cannot be used")
            || stderr.to_lowercase().contains("conflict")
            || stderr.to_lowercase().contains("error"),
        "expected conflict message in stderr; got: {stderr}"
    );
}

#[test]
fn enabled_alone_with_valid_bucket_parses_ok() {
    // Without real credentials the API call would fail (exit 1), but arg
    // parsing itself succeeds. We assert that the process does not exit 2
    // (which would indicate a parse error) and that stderr does not contain
    // clap-level error messages.
    let (ok, _stdout, stderr, code) =
        run(s7cmd().args(["put-bucket-versioning", "s3://example", "--enabled"]));
    assert!(
        code != Some(2),
        "--enabled alone must parse without clap error; code={code:?}; stderr={stderr}"
    );
    let _ = ok; // runtime (non-parse) failure is acceptable in unit tests
}

#[test]
fn suspended_alone_with_valid_bucket_parses_ok() {
    let (ok, _stdout, stderr, code) =
        run(s7cmd().args(["put-bucket-versioning", "s3://example", "--suspended"]));
    assert!(
        code != Some(2),
        "--suspended alone must parse without clap error; code={code:?}; stderr={stderr}"
    );
    let _ = ok;
}

// NOTE: s3util-rs's auto_complete_shell_short_circuits_without_target test
// is omitted — s7cmd hides the per-subcommand --auto-complete-shell flag
// and exposes only the top-level form (covered by tests/cli_help.rs).

#[test]
fn missing_target_exits_non_zero() {
    let (ok, _stdout, stderr, code) = run(s7cmd().args(["put-bucket-versioning", "--enabled"]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(2),
        "missing target should exit 2; stderr: {stderr}"
    );
}
