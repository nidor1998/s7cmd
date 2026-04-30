use assert_cmd::Command;
use predicates::prelude::*;

/// Empty stdin → exits 0, prints summary "0 ok, 0 failed".
#[test]
fn batch_run_empty_stdin_succeeds() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run"])
        .write_stdin("")
        .assert()
        .success()
        .stderr(predicate::str::contains("0 ok, 0 failed"));
}

#[test]
fn batch_run_no_summary_suppresses_summary() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--no-summary"])
        .write_stdin("")
        .assert()
        .success()
        .stderr(predicate::str::contains("batch-run:").not());
}

#[test]
fn batch_run_rejects_nested_batch_run_line() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run"])
        .write_stdin("batch-run\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("nested batch-run"));
}

#[test]
fn batch_run_rejects_per_line_tracing_flag() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run"])
        .write_stdin("head-bucket --aws-sdk-tracing s3://b\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("tracing flags are not allowed"));
}

#[test]
fn batch_run_rejects_stdio_cp_target() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run"])
        .write_stdin("cp s3://bucket/key -\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("stdin/stdout"));
}

#[test]
fn batch_run_parses_blank_and_comment_lines() {
    // The lines below all skip (blank, comment) — net result: 0 commands run.
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run"])
        .write_stdin("\n# this is a comment\n   \n# another\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("0 ok, 0 failed"));
}

#[test]
fn batch_run_parse_error_includes_line_number() {
    // Malformed quoting → parse error mentions line 2.
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run"])
        .write_stdin("# ok\ncp \"unterminated\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("line 2"));
}

#[test]
fn batch_run_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--parallel"))
        .stdout(predicate::str::contains("--streaming"))
        .stdout(predicate::str::contains("--continue-on-error"));
}

#[test]
fn top_level_help_lists_batch_run() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("batch-run"));
}

// ---- streaming-mode coverage ----

#[test]
fn batch_run_streaming_empty_stdin_succeeds() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--streaming"])
        .write_stdin("")
        .assert()
        .success()
        .stderr(predicate::str::contains("0 ok, 0 failed"));
}

#[test]
fn batch_run_streaming_rejects_per_line_tracing_flag() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--streaming"])
        .write_stdin("head-bucket --aws-sdk-tracing s3://b\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("tracing flags are not allowed"));
}

#[test]
fn batch_run_streaming_parses_blank_and_comment_lines() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--streaming"])
        .write_stdin("\n# comment\n   \n")
        .assert()
        .success()
        .stderr(predicate::str::contains("0 ok, 0 failed"));
}

// ---- invalid-config safety: catch bad subcommand configs without running
// earlier valid lines or killing the process mid-batch ----

/// A malformed `sync` line (local-to-local without
/// `--allow-both-local-storage`) must be caught at validate time so that
/// read-all mode bails before any earlier valid line in the batch is
/// executed. Regression test for the previous behavior where
/// `validate::validate` swallowed `s3sync::Config::try_from` errors,
/// letting line 1 run before line 2's bad config was discovered.
#[test]
fn batch_run_invalid_sync_config_aborts_before_running_earlier_lines() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run"])
        .write_stdin(concat!(
            "create-bucket --dry-run s3://b1\n",
            "sync /tmp/nonexistent-src /tmp/nonexistent-dst\n",
            "create-bucket --dry-run s3://b3\n",
        ))
        .assert()
        .failure()
        .stderr(predicate::str::contains("line 2"))
        // Validate-time bail means the executor never started, so no
        // summary is printed and no line is reported as "ok".
        .stderr(predicate::str::contains("1 ok").not());
}

// ---- 16 KiB per-line cap ----

const MAX_LINE_LEN: usize = 16 * 1024;

/// A line at exactly the 16 KiB cap (a comment of MAX_LINE_LEN bytes plus
/// `\n`) must be accepted. Comments are skipped, so the run finishes with
/// `0 ok, 0 failed`.
#[test]
fn batch_run_accepts_line_at_16kib_cap() {
    let mut input = String::with_capacity(MAX_LINE_LEN + 1);
    input.push('#');
    input.extend(std::iter::repeat_n('x', MAX_LINE_LEN - 1));
    input.push('\n');
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::contains("0 ok, 0 failed"));
}

/// A line one byte over the cap must be rejected with a read error
/// pointing at the offending line. Read-all mode bails before any
/// dispatch, so no summary is printed.
#[test]
fn batch_run_rejects_line_over_16kib_cap_read_all() {
    let mut input = String::with_capacity(MAX_LINE_LEN + 3);
    input.push('#');
    input.extend(std::iter::repeat_n('x', MAX_LINE_LEN));
    input.push('\n');
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run"])
        .write_stdin(input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("line 1"))
        .stderr(predicate::str::contains("exceeds"));
}

/// Same in streaming mode. The reader returns `Err`, the executor
/// drains, and the summary line is still printed.
#[test]
fn batch_run_rejects_line_over_16kib_cap_streaming() {
    let mut input = String::with_capacity(MAX_LINE_LEN + 3);
    input.push('#');
    input.extend(std::iter::repeat_n('x', MAX_LINE_LEN));
    input.push('\n');
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--streaming"])
        .write_stdin(input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("line 1"))
        .stderr(predicate::str::contains("exceeds"));
}

/// A malformed `ls` config (e.g. `--recursive` in bucket-listing mode)
/// must NOT call `std::process::exit` mid-batch. dispatch must convert it
/// to exit code 2 so the executor records the failure and prints a
/// summary. Regression test for the previous behavior where Ls/Clean
/// went through `load_config_exit_if_err`, killing the entire batch
/// process and bypassing the summary.
#[test]
fn batch_run_invalid_ls_config_does_not_kill_process() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--continue-on-error"])
        .write_stdin(concat!(
            "create-bucket --dry-run s3://b1\n",
            "ls --recursive\n",
            "create-bucket --dry-run s3://b3\n",
        ))
        .assert()
        .failure()
        // The summary line is the load-bearing assertion: its presence
        // proves the batch finished cleanly instead of being killed.
        .stderr(predicate::str::contains("2 ok, 1 failed"));
}
