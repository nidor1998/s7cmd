//! Process-level e2e tests for bucket-accelerate-configuration subcommands.

#![cfg(e2e_test)]

mod common;

use common::{REGION, TestHelper, generate_bucket_name, run, s7cmd_cmd};

const PROFILE: &str = "s7cmd-e2e-test";

/// Round-trip: put Enabled → get → put Suspended → get.
/// Bucket name must not contain dots (S3 Transfer Acceleration restriction —
/// `generate_bucket_name()` produces names without dots).
#[tokio::test]
async fn accelerate_enable_get_suspend_get_round_trip() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let bucket_arg = format!("s3://{bucket}");

    // 1. Enable accelerate
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-accelerate-configuration",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
        "--enabled",
    ]));
    assert_eq!(
        code,
        Some(0),
        "put-bucket-accelerate --enabled should succeed; stderr: {stderr}"
    );
    assert_eq!(
        stdout.trim(),
        "",
        "put-bucket-accelerate-configuration must produce no stdout"
    );

    // 2. Get accelerate — expect Status=Enabled
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-bucket-accelerate-configuration",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));
    assert_eq!(
        code,
        Some(0),
        "get-bucket-accelerate should succeed; stderr: {stderr}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("get-bucket-accelerate stdout must be JSON");
    assert_eq!(
        json.get("Status").and_then(|v| v.as_str()),
        Some("Enabled"),
        "expected Status=Enabled; got: {json}"
    );

    // 3. Suspend
    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-accelerate-configuration",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
        "--suspended",
    ]));
    assert_eq!(
        code,
        Some(0),
        "put-bucket-accelerate --suspended should succeed; stderr: {stderr}"
    );

    // 4. Get — expect Status=Suspended
    let (code, stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-accelerate-configuration",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));
    assert_eq!(code, Some(0));
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("get-bucket-accelerate stdout must be JSON");
    assert_eq!(
        json.get("Status").and_then(|v| v.as_str()),
        Some("Suspended"),
        "expected Status=Suspended; got: {json}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

/// On a freshly-created bucket that has never had Transfer Acceleration
/// configured, get-bucket-accelerate-configuration should emit nothing on
/// stdout, matching `aws s3api get-bucket-accelerate-configuration --output json`.
#[tokio::test]
async fn get_accelerate_on_unconfigured_bucket_yields_no_output() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let bucket_arg = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-bucket-accelerate-configuration",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));

    helper.delete_bucket_with_cascade(&bucket).await;

    assert_eq!(
        code,
        Some(0),
        "get-bucket-accelerate on unconfigured bucket should succeed; stderr: {stderr}"
    );
    assert!(
        stdout.trim().is_empty(),
        "expected empty stdout for unconfigured bucket; got: {stdout}"
    );
}

/// put-bucket-accelerate on a non-existent bucket should fail with exit 1.
#[tokio::test]
async fn put_accelerate_on_missing_bucket_exits_1() {
    let bucket = generate_bucket_name();
    let bucket_arg = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "put-bucket-accelerate-configuration",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
        "--enabled",
    ]));

    assert_eq!(code, Some(1));
}

/// get-bucket-accelerate on a non-existent bucket should exit 4 (NoSuchBucket).
#[tokio::test]
async fn get_accelerate_on_missing_bucket_exits_4() {
    let bucket = generate_bucket_name();
    let bucket_arg = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-accelerate-configuration",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));

    assert_eq!(code, Some(4), "missing bucket must exit 4 (NoSuchBucket)");
}
