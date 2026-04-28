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
async fn cancel_sync_sigint_exits_130() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    // Seed a 30 MiB object so the s3→local download is slow enough to
    // SIGINT mid-stream even on fast networks (with the rate limit below).
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

    let code = run_with_sigint(&mut cmd).await;
    assert_eq!(code, Some(130), "sync SIGINT must exit 130; got {code:?}");

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

// ---- ls ----

#[tokio::test]
async fn cancel_ls_sigint_exits_130() {
    // Seed many small objects so the recursive listing keeps the work loop
    // active long enough for SIGINT to land. --rate-limit-api throttles
    // ListObjectsV2 calls.
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

    let code = run_with_sigint(&mut cmd).await;
    assert_eq!(code, Some(130), "ls SIGINT must exit 130; got {code:?}");

    helper.delete_bucket_with_cascade(&bucket).await;
}

// ---- clean ----

#[tokio::test]
async fn cancel_clean_sigint_exits_130() {
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
        "--rate-limit-objects",
        "1",
        &target,
    ]);

    let code = run_with_sigint(&mut cmd).await;
    assert_eq!(code, Some(130), "clean SIGINT must exit 130; got {code:?}");

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
