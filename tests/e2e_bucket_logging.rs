//! Process-level e2e tests for bucket-logging subcommands.

#![cfg(e2e_test)]

mod common;

use std::io::Write;
use std::process::Stdio;

use common::{
    REGION, TestHelper, create_temp_dir, create_test_file, generate_bucket_name, run, s7cmd_cmd,
};

fn sample_logging_disabled_json() -> &'static str {
    // Empty config disables logging on the bucket. Avoids needing a target
    // bucket with the right permissions to test a real logging setup.
    r#"{}"#
}

#[tokio::test]
async fn put_bucket_logging_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let config_path = create_test_file(
        &local_dir,
        "logging.json",
        sample_logging_disabled_json().as_bytes(),
    );
    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-logging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
        config_path.to_str().unwrap(),
    ]));

    assert_eq!(
        code,
        Some(0),
        "put-bucket-logging must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn put_bucket_logging_dispatch_bucket_not_found() {
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");

    let local_dir = create_temp_dir();
    let config_path = create_test_file(
        &local_dir,
        "logging.json",
        sample_logging_disabled_json().as_bytes(),
    );

    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-logging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
        config_path.to_str().unwrap(),
    ]));

    assert_eq!(
        code,
        Some(1),
        "put-bucket-logging on missing bucket must exit 1; stderr={stderr}"
    );

    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn put_bucket_logging_via_stdin_dispatch_success() {
    // Covers the `if config_arg == "-"` branch that reads from stdin.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let mut child = s7cmd_cmd()
        .args([
            "put-bucket-logging",
            "--target-profile",
            "s7cmd-e2e-test",
            "--target-region",
            REGION,
            &target,
            "-",
        ])
        .stdin(Stdio::piped())
        .spawn()
        .expect("spawn s7cmd");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(sample_logging_disabled_json().as_bytes())
        .unwrap();
    let out = child.wait_with_output().expect("wait s7cmd");

    assert_eq!(
        out.status.code(),
        Some(0),
        "put-bucket-logging via stdin must exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_bucket_logging_dispatch_success_unconfigured() {
    // No logging configured → empty body → exit 0 with no stdout (matches
    // empty-body semantics of `get-bucket-versioning`).
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-bucket-logging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "get-bucket-logging on unconfigured bucket must exit 0; stderr={stderr}"
    );
    assert!(
        stdout.trim().is_empty(),
        "stdout must be empty for unconfigured logging; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_bucket_logging_dispatch_bucket_not_found() {
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-logging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));
    assert_eq!(
        code,
        Some(4),
        "get-bucket-logging on missing bucket must exit 4"
    );
}

#[tokio::test]
async fn put_bucket_logging_invalid_json_exits_1() {
    // Covers the JSON-parse with_context branch. Parsing fails before any
    // SDK call, so this test does not need a real bucket.
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");

    let local_dir = create_temp_dir();
    let config_path = create_test_file(&local_dir, "logging.json", b"not valid json");

    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-logging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
        config_path.to_str().unwrap(),
    ]));

    assert_eq!(
        code,
        Some(1),
        "put-bucket-logging with invalid JSON must exit 1"
    );
    assert!(
        stderr.contains("parsing JSON from"),
        "stderr must contain the JSON-parse context message; stderr={stderr}"
    );

    let _ = std::fs::remove_dir_all(&local_dir);
}
