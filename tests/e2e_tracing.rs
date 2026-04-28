//! Process-level e2e tests for tracing direction.
//!
//! Each test runs a command past `init_tracing` against real AWS and asserts
//! tracing markers appear on the documented stream and not the other:
//!
//! - `sync` → stdout (default), stderr with `--tracing-stderr`.
//! - everything else → stderr.
//!
//! The actual AWS work performed by these commands is minimal — a single
//! `head-bucket` (or an `ls` of a known empty bucket) is enough; the test
//! only cares about the trace direction, not the AWS result.

#![cfg(e2e_test)]

mod common;

use common::{REGION, TestHelper, generate_bucket_name, run, s7cmd_cmd};
use std::process::Command;

const TRACING_ENV: &[&str] = &[
    "RUST_LOG",
    "NO_COLOR",
    "CLICOLOR",
    "JSON_TRACING",
    "TRACING_STDERR",
    "AWS_SDK_TRACING",
];

/// Strip tracing-related env vars off a Command so user shell config
/// (RUST_LOG, NO_COLOR, etc.) cannot perturb tracing output during tests.
fn scrub_env(cmd: &mut Command) {
    for var in TRACING_ENV {
        cmd.env_remove(var);
    }
}

fn trace_marker_present(s: &str) -> bool {
    // tracing-subscriber's compact formatter emits a level marker per line.
    // Match the leading space so we don't trip on file paths containing
    // "TRACE" / "DEBUG" etc. (unlikely but keeps the heuristic clean).
    s.contains(" TRACE ")
        || s.contains(" DEBUG ")
        || s.contains(" INFO ")
        || s.contains(" WARN ")
        || s.contains(" ERROR ")
        || s.contains("config =")
}

// ---- Default streams ----

#[tokio::test]
async fn sync_tracing_goes_to_stdout_by_default() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}/");
    let mut cmd = s7cmd_cmd();
    cmd.args([
        "sync",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "-vvv",
        "--disable-color-tracing",
        // Empty source dir keeps the upload trivially small — we only want
        // the trace to fire.
        ".",
        &target,
    ]);
    scrub_env(&mut cmd);
    let (_code, stdout, stderr) = run(&mut cmd);

    assert!(
        trace_marker_present(&stdout),
        "sync tracing must appear on stdout; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        !trace_marker_present(&stderr),
        "sync tracing must NOT leak to stderr; stderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn ls_tracing_goes_to_stderr() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}/");
    let mut cmd = s7cmd_cmd();
    cmd.args([
        "ls",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "-vvv",
        "--disable-color-tracing",
        &target,
    ]);
    scrub_env(&mut cmd);
    let (_code, stdout, stderr) = run(&mut cmd);

    assert!(
        trace_marker_present(&stderr),
        "ls tracing must appear on stderr; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        !trace_marker_present(&stdout),
        "ls tracing must NOT leak to stdout; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn clean_tracing_goes_to_stderr() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}/");
    let mut cmd = s7cmd_cmd();
    cmd.args([
        "clean",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--force",
        "-vvv",
        "--disable-color-tracing",
        &target,
    ]);
    scrub_env(&mut cmd);
    let (_code, stdout, stderr) = run(&mut cmd);

    assert!(
        trace_marker_present(&stderr),
        "clean tracing must appear on stderr; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        !trace_marker_present(&stdout),
        "clean tracing must NOT leak to stdout; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn head_bucket_tracing_goes_to_stderr() {
    // Representative util_bin command. head-bucket is the smallest possible
    // AWS round-trip that still goes through start_tracing_if_necessary +
    // trace_config_summary in main.rs.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let mut cmd = s7cmd_cmd();
    cmd.args([
        "head-bucket",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "-vvv",
        "--disable-color-tracing",
        &target,
    ]);
    scrub_env(&mut cmd);
    let (_code, stdout, stderr) = run(&mut cmd);

    assert!(
        trace_marker_present(&stderr),
        "head-bucket tracing must appear on stderr; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        !trace_marker_present(&stdout),
        "head-bucket tracing must NOT leak to stdout; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

// ---- --tracing-stderr flips sync ----

#[tokio::test]
async fn sync_tracing_stderr_flag_flips_to_stderr() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}/");
    let mut cmd = s7cmd_cmd();
    cmd.args([
        "sync",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--tracing-stderr",
        "-vvv",
        "--disable-color-tracing",
        ".",
        &target,
    ]);
    scrub_env(&mut cmd);
    let (_code, stdout, stderr) = run(&mut cmd);

    assert!(
        trace_marker_present(&stderr),
        "sync --tracing-stderr must put tracing on stderr; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        !trace_marker_present(&stdout),
        "sync --tracing-stderr must NOT leak tracing to stdout; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

// ---- --json-tracing produces JSON ----

#[tokio::test]
async fn ls_json_tracing_emits_json() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}/");
    let mut cmd = s7cmd_cmd();
    cmd.args([
        "ls",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--json-tracing",
        "-vvv",
        "--disable-color-tracing",
        &target,
    ]);
    scrub_env(&mut cmd);
    let (_code, _stdout, stderr) = run(&mut cmd);

    assert!(
        stderr.contains(r#""level":"#),
        "expected JSON-tracing 'level' field on stderr; stderr={stderr}"
    );
    assert!(
        stderr.contains(r#""timestamp":"#),
        "expected JSON-tracing 'timestamp' field on stderr; stderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}
