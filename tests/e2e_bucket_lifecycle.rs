//! Process-level e2e tests for bucket-lifecycle-configuration subcommands.

#![cfg(e2e_test)]

mod common;

use std::io::Write;
use std::process::Stdio;

use common::{
    REGION, TestHelper, create_temp_dir, create_test_file, generate_bucket_name, run, s7cmd_cmd,
};

fn sample_lifecycle_json() -> &'static str {
    r#"{
      "Rules": [
        {
          "ID": "ExpireOldLogs",
          "Status": "Enabled",
          "Filter": { "Prefix": "logs/" },
          "Expiration": { "Days": 365 }
        }
      ]
    }"#
}

#[tokio::test]
async fn put_bucket_lifecycle_configuration_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let config_path = create_test_file(
        &local_dir,
        "lifecycle.json",
        sample_lifecycle_json().as_bytes(),
    );
    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-lifecycle-configuration",
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
        "put-bucket-lifecycle-configuration must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn put_bucket_lifecycle_configuration_via_stdin_dispatch_success() {
    // Covers the `if config_arg == "-"` branch that reads from stdin.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let mut child = s7cmd_cmd()
        .args([
            "put-bucket-lifecycle-configuration",
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
        .write_all(sample_lifecycle_json().as_bytes())
        .unwrap();
    let out = child.wait_with_output().expect("wait s7cmd");

    assert_eq!(
        out.status.code(),
        Some(0),
        "put-bucket-lifecycle-configuration via stdin must exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_bucket_lifecycle_configuration_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let config_path = create_test_file(
        &local_dir,
        "lifecycle.json",
        sample_lifecycle_json().as_bytes(),
    );
    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "put-bucket-lifecycle-configuration",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
        config_path.to_str().unwrap(),
    ]));
    assert_eq!(code, Some(0));

    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-bucket-lifecycle-configuration",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "get-bucket-lifecycle-configuration must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("ExpireOldLogs"),
        "stdout must contain seeded rule ID; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn get_bucket_lifecycle_configuration_dispatch_not_found() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-lifecycle-configuration",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(4),
        "get-bucket-lifecycle-configuration on bucket without configuration must exit 4"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_bucket_lifecycle_configuration_dispatch_bucket_not_found() {
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-lifecycle-configuration",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));
    assert_eq!(
        code,
        Some(4),
        "get-bucket-lifecycle-configuration on missing bucket must exit 4"
    );
}

#[tokio::test]
async fn put_bucket_lifecycle_configuration_dispatch_bucket_not_found() {
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");

    let local_dir = create_temp_dir();
    let config_path = create_test_file(
        &local_dir,
        "lifecycle.json",
        sample_lifecycle_json().as_bytes(),
    );

    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-lifecycle-configuration",
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
        "put-bucket-lifecycle-configuration on missing bucket must exit 1; stderr={stderr}"
    );

    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn delete_bucket_lifecycle_configuration_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let config_path = create_test_file(
        &local_dir,
        "lifecycle.json",
        sample_lifecycle_json().as_bytes(),
    );
    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "put-bucket-lifecycle-configuration",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
        config_path.to_str().unwrap(),
    ]));
    assert_eq!(code, Some(0));

    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "delete-bucket-lifecycle-configuration",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "delete-bucket-lifecycle-configuration must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn delete_bucket_lifecycle_configuration_dispatch_bucket_not_found() {
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");

    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "delete-bucket-lifecycle-configuration",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(1),
        "delete-bucket-lifecycle-configuration on missing bucket must exit 1; stderr={stderr}"
    );
}
