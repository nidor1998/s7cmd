//! Process-level CLI tests for the `presign` subcommand.
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
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["presign", "--help"]));
    assert!(ok, "presign --help must succeed");
    assert!(stdout.contains("AWS Configuration"));
    assert!(stdout.contains("Retry Options"));
    assert!(stdout.contains("Timeout Options"));
}

#[test]
fn help_mentions_expires_in_and_default() {
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["presign", "--help"]));
    assert!(ok);
    assert!(
        stdout.contains("--expires-in"),
        "help must mention --expires-in; got: {stdout}"
    );
    assert!(
        stdout.contains("3600"),
        "help must surface the 3600s default; got: {stdout}"
    );
    assert!(
        stdout.contains("604800"),
        "help must mention the 604800s (1 week) max; got: {stdout}"
    );
}

#[test]
fn help_does_not_offer_dry_run() {
    // presign issues no S3 API call and mutates nothing; --dry-run does not
    // apply. Pin its absence so a future copy/paste from a mutating command
    // doesn't accidentally introduce one.
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["presign", "--help"]));
    assert!(ok);
    assert!(
        !stdout.contains("--dry-run"),
        "presign must not expose --dry-run; got: {stdout}"
    );
}

#[test]
fn help_does_not_offer_source_version_id() {
    // AWS CLI's `aws s3 presign` is GET-only and does not accept --version-id.
    // Pin parity here so we don't drift from the documented behaviour.
    let (ok, stdout, _stderr, _code) = run(s7cmd().args(["presign", "--help"]));
    assert!(ok);
    assert!(
        !stdout.contains("--source-version-id"),
        "presign must not expose --source-version-id; got: {stdout}"
    );
}

#[test]
fn missing_positional_exits_2() {
    let (ok, _stdout, stderr, code) = run(s7cmd().arg("presign"));
    assert!(!ok);
    assert_eq!(code, Some(2), "clap missing-arg should exit 2");
    assert!(stderr.to_lowercase().contains("required") || stderr.to_lowercase().contains("usage"));
}

// NOTE: upstream s3util has a `--auto-complete-shell bash` happy-path test on
// the presign subcommand, but s7cmd intentionally strips the long form of
// `--auto-complete-shell` from every subcommand (see `cli::cli_command`) and
// requires the top-level `s7cmd --auto-complete-shell SHELL` form instead.
// That top-level path is covered by `tests/cli_help.rs::top_level_auto_complete_shell_runs`.

#[test]
fn bucket_only_path_no_key_exits_1() {
    // presign requires a key — `s3://bucket` with no key fails the
    // post-parse `bucket_key()` validation, which run_presign maps to
    // an anyhow error → EXIT_CODE_ERROR (1).
    let (ok, _stdout, stderr, code) = run(s7cmd().args(["presign", "s3://bucket"]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(1),
        "bucket-only path should exit 1 (validation); stderr: {stderr}"
    );
}

#[test]
fn bucket_with_trailing_slash_exits_1() {
    let (ok, _stdout, _stderr, code) = run(s7cmd().args(["presign", "s3://bucket/"]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(1),
        "trailing-slash path with empty key should exit 1 (validation)"
    );
}

#[test]
fn local_path_target_exits_1() {
    // `check_storage_path` accepts local paths at parse time (cp/mv share
    // the same value_parser); presign's `bucket_key()` rejects them
    // post-parse → run_presign returns anyhow → EXIT_CODE_ERROR (1).
    let (ok, _stdout, _stderr, code) = run(s7cmd().args(["presign", "/tmp/local"]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(1),
        "non-s3 target should fail bucket_key() validation → exit 1"
    );
}

#[test]
fn unsupported_scheme_exits_2() {
    // A URL with a non-s3, non-stdio scheme is rejected by `check_storage_path`
    // at parse time → clap exit 2.
    let (ok, _stdout, _stderr, code) = run(s7cmd().args(["presign", "http://example.com/key"]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(2),
        "unsupported scheme should be rejected by clap value_parser → exit 2"
    );
}

#[test]
fn expires_in_zero_exits_2() {
    let (ok, _stdout, stderr, code) =
        run(s7cmd().args(["presign", "s3://bucket/key", "--expires-in", "0"]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(2),
        "--expires-in=0 must be rejected by clap value_parser → exit 2; stderr: {stderr}"
    );
}

#[test]
fn expires_in_over_one_week_exits_2() {
    let (ok, _stdout, stderr, code) =
        run(s7cmd().args(["presign", "s3://bucket/key", "--expires-in", "604801"]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(2),
        "--expires-in over 604800 must be rejected → exit 2; stderr: {stderr}"
    );
    assert!(
        stderr.contains("604800"),
        "rejection message should cite the 604800s ceiling; got: {stderr}"
    );
}

#[test]
fn expires_in_negative_exits_2() {
    // Caught by clap's u64 parser before ever reaching parse_expires_in.
    // We pin the exit code anyway so a future signed type wouldn't silently
    // accept negatives.
    let (ok, _stdout, _stderr, code) =
        run(s7cmd().args(["presign", "s3://bucket/key", "--expires-in", "-1"]));
    assert!(!ok);
    assert_eq!(code, Some(2), "negative --expires-in must exit 2");
}

#[test]
fn expires_in_non_numeric_exits_2() {
    let (ok, _stdout, _stderr, code) =
        run(s7cmd().args(["presign", "s3://bucket/key", "--expires-in", "many"]));
    assert!(!ok);
    assert_eq!(
        code,
        Some(2),
        "non-numeric --expires-in must be rejected by clap → exit 2"
    );
}

#[test]
fn target_access_key_without_secret_exits_2() {
    let (ok, _stdout, stderr, code) =
        run(s7cmd().args(["presign", "s3://bucket/key", "--target-access-key", "AKIA"]));
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
        "presign",
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
