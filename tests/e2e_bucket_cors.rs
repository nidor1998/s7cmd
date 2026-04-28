//! Process-level e2e tests for bucket-cors subcommands.

#![cfg(e2e_test)]

mod common;

use std::io::Write;
use std::process::Stdio;

use common::{
    REGION, TestHelper, create_temp_dir, create_test_file, generate_bucket_name, run, s7cmd_cmd,
};

fn sample_cors_json() -> &'static str {
    r#"{
      "CORSRules": [
        {
          "ID": "r1",
          "AllowedMethods": ["GET", "HEAD"],
          "AllowedOrigins": ["*"],
          "AllowedHeaders": ["*"],
          "MaxAgeSeconds": 3000
        }
      ]
    }"#
}

#[tokio::test]
async fn put_bucket_cors_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let config_path = create_test_file(&local_dir, "cors.json", sample_cors_json().as_bytes());

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-cors",
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
        "put-bucket-cors must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn put_bucket_cors_via_stdin_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let mut child = s7cmd_cmd()
        .args([
            "put-bucket-cors",
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
        .write_all(sample_cors_json().as_bytes())
        .unwrap();
    let out = child.wait_with_output().expect("wait s7cmd");

    assert_eq!(
        out.status.code(),
        Some(0),
        "put-bucket-cors via stdin must exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_bucket_cors_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let config_path = create_test_file(&local_dir, "cors.json", sample_cors_json().as_bytes());
    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "put-bucket-cors",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
        config_path.to_str().unwrap(),
    ]));
    assert_eq!(code, Some(0));

    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-bucket-cors",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "get-bucket-cors must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("CORSRules"),
        "stdout must contain CORSRules; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn get_bucket_cors_dispatch_not_found() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-cors",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(4),
        "get-bucket-cors on bucket without CORS must exit 4"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_bucket_cors_dispatch_bucket_not_found() {
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-cors",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));
    assert_eq!(
        code,
        Some(4),
        "get-bucket-cors on missing bucket must exit 4"
    );
}

#[tokio::test]
async fn put_bucket_cors_dispatch_bucket_not_found() {
    // PUT on a missing bucket: api::put_bucket_cors propagates the SDK error
    // through `?`, run_put_bucket_cors returns Result::Err, and main.rs maps
    // it to EXIT_CODE_ERROR (1). This is distinct from the GET arms, which
    // route NoSuchBucket through HeadError and exit 4.
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");

    let local_dir = create_temp_dir();
    let config_path = create_test_file(&local_dir, "cors.json", sample_cors_json().as_bytes());

    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-cors",
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
        "put-bucket-cors on missing bucket must exit 1; stderr={stderr}"
    );

    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn delete_bucket_cors_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let config_path = create_test_file(&local_dir, "cors.json", sample_cors_json().as_bytes());
    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "put-bucket-cors",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
        config_path.to_str().unwrap(),
    ]));
    assert_eq!(code, Some(0));

    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "delete-bucket-cors",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "delete-bucket-cors must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn delete_bucket_cors_dispatch_bucket_not_found() {
    // DELETE uses Result<()> (not HeadError), so NoSuchBucket → exit 1.
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");

    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "delete-bucket-cors",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(1),
        "delete-bucket-cors on missing bucket must exit 1; stderr={stderr}"
    );
}
