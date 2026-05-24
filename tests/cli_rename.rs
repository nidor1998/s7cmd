//! Process-level CLI tests for the `rename` subcommand.
//! These run without AWS credentials or network access.

use assert_cmd::Command;
use predicates::prelude::*;

fn s7cmd() -> Command {
    Command::cargo_bin("s7cmd").unwrap()
}

// Express One Zone bucket name usable in no-AWS tests: passes validate() because
// the bucket ends with --x-s3, and source/target share the same bucket name.
const EXPR_BUCKET: &str = "s3://fake-bucket--apne1-az4--x-s3";

#[test]
fn rename_help_succeeds_and_lists_option_groups() {
    s7cmd()
        .args(["rename", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("AWS Configuration"))
        .stdout(predicate::str::contains("Conditional Checks"))
        .stdout(predicate::str::contains("--source-if-match"))
        .stdout(predicate::str::contains("--target-if-none-match"))
        .stdout(predicate::str::contains("--dry-run"));
}

#[test]
fn rename_top_level_help_lists_rename() {
    s7cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("rename"));
}

#[test]
fn rename_missing_args_exits_2() {
    s7cmd().arg("rename").assert().failure().code(2);
}

#[test]
fn rename_missing_target_exits_2() {
    s7cmd()
        .args(["rename", &format!("{EXPR_BUCKET}/src-key")])
        .assert()
        .failure()
        .code(2);
}

// NOTE: s3util-rs's auto_complete_shell_short_circuits_without_positional_args test
// is omitted — s7cmd hides the per-subcommand --auto-complete-shell flag (see
// cli_command() in src/cli.rs). The top-level `s7cmd --auto-complete-shell bash`
// form is tested in tests/cli_help.rs.

#[test]
fn rename_source_bucket_only_exits_2() {
    // s3://bucket with no key → validate() → source_bucket_key() error → exit 2
    s7cmd()
        .args([
            "rename",
            "s3://fake-bucket--apne1-az4--x-s3",
            &format!("{EXPR_BUCKET}/dst"),
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn rename_target_bucket_only_exits_2() {
    // source is valid, but target has no key → exit 2
    s7cmd()
        .args([
            "rename",
            &format!("{EXPR_BUCKET}/src"),
            "s3://fake-bucket--apne1-az4--x-s3",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn rename_non_express_onezone_bucket_exits_2() {
    s7cmd()
        .args([
            "rename",
            "s3://regular-bucket/src-key",
            "s3://regular-bucket/dst-key",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("Express").or(predicate::str::contains("x-s3")));
}

#[test]
fn rename_different_buckets_exits_2() {
    s7cmd()
        .args([
            "rename",
            "s3://bucket-a--apne1-az4--x-s3/src-key",
            "s3://bucket-b--apne1-az4--x-s3/dst-key",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("same bucket"));
}

#[test]
fn rename_source_access_key_without_secret_exits_2() {
    s7cmd()
        .args([
            "rename",
            &format!("{EXPR_BUCKET}/src"),
            &format!("{EXPR_BUCKET}/dst"),
            "--source-access-key",
            "AKIAIOSFODNN7EXAMPLE",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn rename_source_profile_conflicts_with_access_key_exits_2() {
    s7cmd()
        .args([
            "rename",
            &format!("{EXPR_BUCKET}/src"),
            &format!("{EXPR_BUCKET}/dst"),
            "--source-profile",
            "myprofile",
            "--source-access-key",
            "AKID",
            "--source-secret-access-key",
            "SECRET",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn rename_source_if_match_and_source_if_none_match_are_mutually_exclusive() {
    s7cmd()
        .args([
            "rename",
            &format!("{EXPR_BUCKET}/src"),
            &format!("{EXPR_BUCKET}/dst"),
            "--source-if-match",
            "\"abc123\"",
            "--source-if-none-match",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn rename_target_if_match_and_target_if_none_match_are_mutually_exclusive() {
    s7cmd()
        .args([
            "rename",
            &format!("{EXPR_BUCKET}/src"),
            &format!("{EXPR_BUCKET}/dst"),
            "--target-if-match",
            "\"abc123\"",
            "--target-if-none-match",
        ])
        .assert()
        .failure()
        .code(2);
}
