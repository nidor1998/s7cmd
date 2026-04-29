//! Process-level e2e tests for bucket-encryption subcommands.

#![cfg(e2e_test)]

mod common;

use std::io::Write;
use std::process::Stdio;

use common::{
    REGION, TestHelper, create_temp_dir, create_test_file, generate_bucket_name, run, s7cmd_cmd,
};

fn sample_encryption_json() -> &'static str {
    r#"{
      "Rules": [
        {
          "ApplyServerSideEncryptionByDefault": { "SSEAlgorithm": "AES256" }
        }
      ]
    }"#
}

#[tokio::test]
async fn put_bucket_encryption_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let config_path = create_test_file(
        &local_dir,
        "encryption.json",
        sample_encryption_json().as_bytes(),
    );
    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-encryption",
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
        "put-bucket-encryption must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn put_bucket_encryption_via_stdin_dispatch_success() {
    // Covers the `if config_arg == "-"` branch that reads from stdin.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let mut child = s7cmd_cmd()
        .args([
            "put-bucket-encryption",
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
        .write_all(sample_encryption_json().as_bytes())
        .unwrap();
    let out = child.wait_with_output().expect("wait s7cmd");

    assert_eq!(
        out.status.code(),
        Some(0),
        "put-bucket-encryption via stdin must exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_bucket_encryption_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let config_path = create_test_file(
        &local_dir,
        "encryption.json",
        sample_encryption_json().as_bytes(),
    );
    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "put-bucket-encryption",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
        config_path.to_str().unwrap(),
    ]));
    assert_eq!(code, Some(0));

    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-bucket-encryption",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "get-bucket-encryption must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("AES256"),
        "stdout must contain AES256; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn get_bucket_encryption_dispatch_bucket_not_found() {
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-encryption",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));
    assert_eq!(
        code,
        Some(4),
        "get-bucket-encryption on missing bucket must exit 4"
    );
}

#[tokio::test]
async fn put_bucket_encryption_missing_config_file_exits_1() {
    // Covers the with_context closure body that renders the file-read error.
    // The runner reads the config file before any SDK call, so this test does
    // not need a real bucket — file-read fails first and propagates as exit 1.
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");
    let nonexistent = "/tmp/s7cmd-nonexistent-encryption-config.json";

    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-encryption",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
        nonexistent,
    ]));

    assert_eq!(
        code,
        Some(1),
        "put-bucket-encryption with missing config file must exit 1"
    );
    assert!(
        stderr.contains("reading server-side-encryption configuration"),
        "stderr must contain the file-read context message; stderr={stderr}"
    );
}

#[tokio::test]
async fn put_bucket_encryption_dispatch_bucket_not_found() {
    // PUT uses Result<()>; NoSuchBucket from the SDK propagates through `?`
    // and is mapped to EXIT_CODE_ERROR (1) in main.rs.
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");

    let local_dir = create_temp_dir();
    let config_path = create_test_file(
        &local_dir,
        "encryption.json",
        sample_encryption_json().as_bytes(),
    );

    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-encryption",
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
        "put-bucket-encryption on missing bucket must exit 1; stderr={stderr}"
    );

    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn delete_bucket_encryption_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let config_path = create_test_file(
        &local_dir,
        "encryption.json",
        sample_encryption_json().as_bytes(),
    );
    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "put-bucket-encryption",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
        config_path.to_str().unwrap(),
    ]));
    assert_eq!(code, Some(0));

    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "delete-bucket-encryption",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "delete-bucket-encryption must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn delete_bucket_encryption_dispatch_bucket_not_found() {
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");

    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "delete-bucket-encryption",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(1),
        "delete-bucket-encryption on missing bucket must exit 1; stderr={stderr}"
    );
}

#[tokio::test]
async fn put_bucket_encryption_invalid_json_exits_1() {
    // Covers the JSON-parse with_context branch. Parsing fails before any
    // SDK call, so this test does not need a real bucket.
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");

    let local_dir = create_temp_dir();
    let config_path = create_test_file(&local_dir, "encryption.json", b"not valid json");

    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-encryption",
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
        "put-bucket-encryption with invalid JSON must exit 1"
    );
    assert!(
        stderr.contains("parsing JSON from"),
        "stderr must contain the JSON-parse context message; stderr={stderr}"
    );

    let _ = std::fs::remove_dir_all(&local_dir);
}
