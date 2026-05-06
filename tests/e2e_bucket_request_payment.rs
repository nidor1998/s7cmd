//! Process-level e2e tests for bucket-request-payment subcommands.

#![cfg(e2e_test)]

mod common;

use common::{REGION, TestHelper, generate_bucket_name, run, s7cmd_cmd};

const PROFILE: &str = "s7cmd-e2e-test";

/// Round-trip: get default → put Requester → get → put BucketOwner → get.
#[tokio::test]
async fn request_payment_round_trip() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let bucket_arg = format!("s3://{bucket}");

    // 1. Get default — should be BucketOwner
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-bucket-request-payment",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));
    assert_eq!(
        code,
        Some(0),
        "get-bucket-request-payment should succeed; stderr: {stderr}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("get-bucket-request-payment stdout must be JSON");
    assert_eq!(
        json.get("Payer").and_then(|v| v.as_str()),
        Some("BucketOwner"),
        "expected default Payer=BucketOwner; got: {json}"
    );

    // 2. Put Requester
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-request-payment",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
        "--requester",
    ]));
    assert_eq!(
        code,
        Some(0),
        "put --requester should succeed; stderr: {stderr}"
    );
    assert_eq!(
        stdout.trim(),
        "",
        "put-bucket-request-payment must produce no stdout"
    );

    // 3. Get — expect Requester
    let (code, stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-request-payment",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));
    assert_eq!(code, Some(0));
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        json.get("Payer").and_then(|v| v.as_str()),
        Some("Requester")
    );

    // 4. Put BucketOwner (back to default)
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "put-bucket-request-payment",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
        "--bucket-owner",
    ]));
    assert_eq!(code, Some(0));

    // 5. Get — expect BucketOwner
    let (code, stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-request-payment",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));
    assert_eq!(code, Some(0));
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        json.get("Payer").and_then(|v| v.as_str()),
        Some("BucketOwner")
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

/// put-bucket-request-payment on a non-existent bucket should fail with exit 1.
#[tokio::test]
async fn put_request_payment_on_missing_bucket_exits_1() {
    let bucket = generate_bucket_name();
    let bucket_arg = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "put-bucket-request-payment",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
        "--requester",
    ]));

    assert_eq!(code, Some(1));
}

/// get-bucket-request-payment on a non-existent bucket should exit 4 (NoSuchBucket).
#[tokio::test]
async fn get_request_payment_on_missing_bucket_exits_4() {
    let bucket = generate_bucket_name();
    let bucket_arg = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-request-payment",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));

    assert_eq!(code, Some(4), "missing bucket must exit 4 (NoSuchBucket)");
}
