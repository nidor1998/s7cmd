//! Process-level CLI tests for the `put-bucket-policy` subcommand.
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
fn help_shows_both_positionals() {
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["put-bucket-policy", "--help"]));
    assert!(ok, "put-bucket-policy --help must succeed");
    // Both positionals must appear in usage
    assert!(
        stdout.contains("TARGET") || stdout.contains("BUCKET"),
        "expected TARGET or BUCKET in help; got: {stdout}"
    );
    assert!(
        stdout.contains("POLICY"),
        "expected POLICY in help; got: {stdout}"
    );
    assert!(stdout.contains("AWS Configuration"));
    assert!(stdout.contains("Retry Options"));
    assert!(stdout.contains("Timeout Options"));
}

#[test]
fn missing_both_positionals_exits_2() {
    let (ok, _stdout, stderr, code) = run(s7cmd().arg("put-bucket-policy"));
    assert!(!ok);
    assert_eq!(
        code,
        Some(2),
        "clap missing-arg should exit 2; stderr: {stderr}"
    );
    assert!(
        stderr.to_lowercase().contains("required") || stderr.to_lowercase().contains("usage"),
        "expected 'required' or 'usage' in stderr; got: {stderr}"
    );
}

#[test]
fn missing_policy_positional_exits_2() {
    let (ok, _stdout, stderr, code) =
        run(s7cmd().args(["put-bucket-policy", "s3://example-bucket"]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(2),
        "missing policy positional should exit 2; stderr: {stderr}"
    );
    assert!(
        stderr.to_lowercase().contains("required") || stderr.to_lowercase().contains("usage"),
        "expected 'required' or 'usage' in stderr; got: {stderr}"
    );
}

#[test]
fn nonexistent_policy_file_exits_1() {
    let (ok, _stdout, stderr, code) = run(s7cmd().args([
        "put-bucket-policy",
        "s3://example-bucket",
        "/nonexistent/path/policy-xyz-does-not-exist.json",
    ]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(1),
        "reading non-existent file must exit 1; got {code:?}; stderr: {stderr}"
    );
    assert!(
        stderr.to_lowercase().contains("no such file")
            || stderr.to_lowercase().contains("not found")
            || stderr.to_lowercase().contains("os error"),
        "expected file-not-found error in stderr; got: {stderr}"
    );
}

// NOTE: s3util-rs's auto_complete_shell_short_circuits_without_target test
// is omitted — s7cmd hides the per-subcommand --auto-complete-shell flag
// and exposes only the top-level form (covered by tests/cli_help.rs).

#[test]
fn target_access_key_without_secret_exits_non_zero() {
    // Create a temp file so the positionals parse correctly
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let (ok, _stdout, stderr, code) = run(s7cmd().args([
        "put-bucket-policy",
        "s3://example",
        tmp.path().to_str().unwrap(),
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
            || stderr.to_lowercase().contains("--target-secret-access-key"),
        "expected clap error about missing secret key; got: {stderr}"
    );
}
