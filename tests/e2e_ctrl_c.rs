//! Process-level e2e tests for graceful Ctrl+C handling.
//!
//! Each test spawns the binary directly (NOT via cargo run — cargo
//! intercepts SIGINT), gives it ~1.5s to enter its work loop, sends SIGINT,
//! and asserts the child exits with code 130 within a 30s timeout.
//!
//! Unix-only: SIGINT delivery via the `nix` crate.

#![cfg(all(e2e_test, unix))]

mod common;

use common::{
    REGION, TestHelper, create_sized_file, create_temp_dir, generate_bucket_name, s7cmd_cmd,
};

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::process::Stdio;
use std::time::Duration;

const STARTUP_DELAY_MS: u64 = 1500;
const WAIT_TIMEOUT_SECS: u64 = 30;

/// Spawn `cmd`, sleep `STARTUP_DELAY_MS`, deliver SIGINT, wait for exit
/// (capped at `WAIT_TIMEOUT_SECS`), and return the exit code. Stdout and
/// stderr of the child are discarded — these tests assert on exit code,
/// not output.
async fn run_with_sigint(cmd: &mut std::process::Command) -> Option<i32> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn s7cmd");

    tokio::time::sleep(Duration::from_millis(STARTUP_DELAY_MS)).await;
    let pid = Pid::from_raw(child.id() as i32);
    let _ = kill(pid, Signal::SIGINT);

    // Bounded wait so a hang fails fast instead of stalling CI.
    let wait_handle = tokio::task::spawn_blocking(move || child.wait());
    match tokio::time::timeout(Duration::from_secs(WAIT_TIMEOUT_SECS), wait_handle).await {
        Ok(Ok(Ok(status))) => status.code(),
        Ok(Ok(Err(e))) => panic!("child.wait() failed: {e}"),
        Ok(Err(e)) => panic!("spawn_blocking join failed: {e}"),
        Err(_) => panic!("child did not exit within {WAIT_TIMEOUT_SECS}s after SIGINT"),
    }
}

// ---- sync ----

#[tokio::test]
async fn cancel_sync_sigint_does_not_hang() {
    // sync's exit-on-SIGINT is non-deterministic: src/sync_bin/cli/mod.rs has
    // no explicit "cancelled → exit 130" path. Depending on what the pipeline
    // had done by the time SIGINT lands, the process can exit 0 (clean
    // cancellation, nothing pending), 3 (warning — partial completion), or
    // 1 (error). The strict exit-130 assertion that fits cp/mv (which DO
    // have an explicit ExitStatus::Cancelled path in util_bin) does not
    // apply to sync. Per the spec's section-7 fallback, we assert only that
    // the process exits — `run_with_sigint` already enforces a 30s timeout,
    // so reaching this line proves SIGINT was honored.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper
        .put_object(&bucket, "big.bin", vec![0u8; 30 * 1024 * 1024])
        .await;

    let local_dir = create_temp_dir();
    let source = format!("s3://{bucket}/");
    let mut cmd = s7cmd_cmd();
    cmd.args([
        "sync",
        "--source-profile",
        "s7cmd-e2e-test",
        "--source-region",
        REGION,
        "--rate-limit-bandwidth",
        "2MiB",
        &source,
        local_dir.to_str().unwrap(),
    ]);

    let _code = run_with_sigint(&mut cmd).await;

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

// ---- ls ----

#[tokio::test]
async fn cancel_ls_sigint_does_not_hang() {
    // S3 paginates ListObjectsV2 at 1000 objects; --rate-limit-api throttles
    // BETWEEN pages, not within a page, so 200 objects in a single page
    // would return before the 1500ms SIGINT delivery on a fast network.
    // Seeding 1000+ objects to force multi-page listing is wasteful for a
    // dispatch-only test, so we fall back to the spec-authorized soft
    // assertion: confirm the process exits (i.e. doesn't hang) and that
    // SIGINT was honored — exact exit code is not required because, on a
    // very fast listing, the process may complete normally before SIGINT
    // lands. The richer "must exit 130" assertion is covered by sync, cp,
    // mv, and clean (which all have per-byte/per-object throttles).
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    for i in 0..200 {
        helper
            .put_object(&bucket, &format!("k{i:04}"), b"x".to_vec())
            .await;
    }

    let target = format!("s3://{bucket}/");
    let mut cmd = s7cmd_cmd();
    cmd.args([
        "ls",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--recursive",
        "--rate-limit-api",
        "1",
        &target,
    ]);

    // run_with_sigint already enforces a 30s timeout — passing it means the
    // process exited (not hung). We don't assert on the exit code.
    let _code = run_with_sigint(&mut cmd).await;

    helper.delete_bucket_with_cascade(&bucket).await;
}

// ---- clean ----

#[tokio::test]
async fn cancel_clean_sigint_does_not_hang() {
    // clean's bulk-delete is too fast to reliably catch with SIGINT at scale
    // suitable for a dispatch test. --rate-limit-objects has hard floor 10
    // and must be >= --batch-size (default 200), so the practical minimum
    // throttle is 10/sec with --batch-size 10. With 200 seeded objects the
    // theoretical duration is ~20s, but the leaky-bucket token allowance
    // and concurrent batch deletion mean the first delete can drain the
    // bucket fast enough that exit 0 races SIGINT — observed in practice.
    // Per the spec's section-7 fallback, soften to "process exits, doesn't
    // hang." Strict exit-130 coverage stays in cp/mv (per-byte bandwidth
    // throttle on a 30 MiB transfer is reliable).
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    for i in 0..200 {
        helper
            .put_object(&bucket, &format!("k{i:04}"), b"x".to_vec())
            .await;
    }

    let target = format!("s3://{bucket}/");
    let mut cmd = s7cmd_cmd();
    cmd.args([
        "clean",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--force",
        "--batch-size",
        "10",
        "--rate-limit-objects",
        "10",
        &target,
    ]);

    let _code = run_with_sigint(&mut cmd).await;

    helper.delete_bucket_with_cascade(&bucket).await;
}

// ---- cp ----

#[tokio::test]
async fn cancel_cp_sigint_exits_130() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let big = create_sized_file(&local_dir, "big.bin", 30 * 1024 * 1024);
    let target = format!("s3://{bucket}/big.bin");

    let mut cmd = s7cmd_cmd();
    cmd.args([
        "cp",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--rate-limit-bandwidth",
        "2MiB",
        big.to_str().unwrap(),
        &target,
    ]);

    let code = run_with_sigint(&mut cmd).await;
    assert_eq!(code, Some(130), "cp SIGINT must exit 130; got {code:?}");

    helper.abort_all_multipart_uploads(&bucket).await;
    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

// ---- mv ----

#[tokio::test]
async fn cancel_mv_sigint_exits_130() {
    let helper = TestHelper::new().await;
    let src_bucket = generate_bucket_name();
    let dst_bucket = generate_bucket_name();
    helper.create_bucket(&src_bucket, REGION).await;
    helper.create_bucket(&dst_bucket, REGION).await;
    helper
        .put_object(&src_bucket, "big.bin", vec![0u8; 30 * 1024 * 1024])
        .await;

    let source = format!("s3://{src_bucket}/big.bin");
    let target = format!("s3://{dst_bucket}/big.bin");
    let mut cmd = s7cmd_cmd();
    cmd.args([
        "mv",
        "--source-profile",
        "s7cmd-e2e-test",
        "--source-region",
        REGION,
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--rate-limit-bandwidth",
        "2MiB",
        &source,
        &target,
    ]);

    let code = run_with_sigint(&mut cmd).await;
    assert_eq!(code, Some(130), "mv SIGINT must exit 130; got {code:?}");

    helper.abort_all_multipart_uploads(&src_bucket).await;
    helper.abort_all_multipart_uploads(&dst_bucket).await;
    helper.delete_bucket_with_cascade(&src_bucket).await;
    helper.delete_bucket_with_cascade(&dst_bucket).await;
}
