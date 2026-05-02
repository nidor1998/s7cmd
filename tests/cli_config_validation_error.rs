//! Process-level test: invalid args that survive clap's own parsing but fail
//! `Config::try_from` must be re-raised through clap's error machinery so the
//! user sees the validation message on stderr and the process exits non-zero.
//!
//! Covers `src/main.rs` — the `Err(error_message)` arm
//! of `match Config::try_from(cp_args)`.
//!
//! Doesn't require AWS: the error fires before any S3 call.

use std::process::{Command, Stdio};

#[test]
fn both_local_paths_exit_non_zero_with_validation_message_on_stderr() {
    let bin = env!("CARGO_BIN_EXE_s7cmd");

    // Two local paths are valid per clap's per-arg value_parser (check_storage_path)
    // but rejected by Config::try_from's check_both_local guard. That error is
    // re-wrapped as clap::ErrorKind::ValueValidation and printed by .exit().
    let output = Command::new(bin)
        .args(["cp", "/tmp/s7cmd_e2e_src", "/tmp/s7cmd_e2e_dst"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to spawn s7cmd binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success(),
        "both-local-paths invocation must exit non-zero.\n\
         status: {:?}\n--- stderr ---\n{stderr}\n--- stdout ---\n{stdout}",
        output.status.code()
    );

    assert!(
        stderr.contains("source and target cannot both be local paths"),
        "expected the check_both_local validation message on stderr.\n\
         --- stderr ---\n{stderr}"
    );
}

#[test]
fn source_no_sign_request_env_var_triggers_conflict_at_parse_time() {
    // Regression guard for the `env` attribute on `--source-no-sign-request`.
    //
    // Rather than mutate the test process's env (which races with parallel
    // tests that parse CpArgs), we isolate the env var to a child `s7cmd`
    // invocation. If clap reads SOURCE_NO_SIGN_REQUEST, combining it with
    // --source-profile will trip the `conflicts_with_all` at parse time and
    // the command exits non-zero with "cannot be used with" on stderr.
    // If clap ever silently drops the env binding, --source-profile alone
    // would be accepted and the command would proceed — a regression we
    // want to catch.
    let bin = env!("CARGO_BIN_EXE_s7cmd");

    let output = Command::new(bin)
        .args([
            "cp",
            "s3://b/k",
            "/tmp/out",
            "--source-profile",
            "myprofile",
        ])
        .env("SOURCE_NO_SIGN_REQUEST", "true")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to spawn s7cmd binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success(),
        "SOURCE_NO_SIGN_REQUEST + --source-profile must exit non-zero.\n\
         status: {:?}\n--- stderr ---\n{stderr}\n--- stdout ---\n{stdout}",
        output.status.code()
    );

    assert!(
        stderr.contains("cannot be used with"),
        "expected clap conflict message on stderr.\n\
         --- stderr ---\n{stderr}"
    );
}
