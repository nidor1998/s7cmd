//! Process-level e2e tests for the restore-object subcommand.

#![cfg(e2e_test)]

mod common;

use common::{REGION, TestHelper, generate_bucket_name, run, s7cmd_cmd};

const PROFILE: &str = "s7cmd-e2e-test";

/// restore-object on a Standard-storage object should fail with
/// `InvalidObjectState` (object is not in an archive tier) — exit 1.
#[tokio::test]
async fn restore_standard_class_object_exits_1() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let key = "test-object.txt";
    helper.put_object(&bucket, key, b"hello".to_vec()).await;

    let object_arg = format!("s3://{bucket}/{key}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "restore-object",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        "--days",
        "1",
        "--tier",
        "Standard",
        &object_arg,
    ]));

    helper.delete_bucket_with_cascade(&bucket).await;

    assert_eq!(
        code,
        Some(1),
        "restore-object on Standard-class object should fail (InvalidObjectState)"
    );
}

/// restore-object on a non-existent object should exit 4
/// (S3 returns NoSuchKey → NotFound).
#[tokio::test]
async fn restore_missing_object_exits_4() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let object_arg = format!("s3://{bucket}/nonexistent-key");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "restore-object",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        "--days",
        "1",
        &object_arg,
    ]));

    helper.delete_bucket_with_cascade(&bucket).await;

    assert_eq!(code, Some(4), "missing object must exit 4 (NoSuchKey)");
}

/// restore-object on a non-existent bucket should exit 4 (NoSuchBucket).
#[tokio::test]
async fn restore_on_missing_bucket_exits_4() {
    let bucket = generate_bucket_name();
    let object_arg = format!("s3://{bucket}/key");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "restore-object",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        "--days",
        "1",
        &object_arg,
    ]));

    assert_eq!(code, Some(4), "missing bucket must exit 4 (NoSuchBucket)");
}

/// restore-object accepts all three tiers (Standard, Bulk, Expedited) at
/// the parse stage. Server-side rejection (InvalidObjectState on Standard
/// storage) is the same regardless of tier — but we want to confirm none
/// of the tier values trigger a clap parse error.
#[tokio::test]
async fn restore_accepts_each_tier_value() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let key = "test-object.txt";
    helper.put_object(&bucket, key, b"hello".to_vec()).await;

    let object_arg = format!("s3://{bucket}/{key}");

    for tier in ["Standard", "Bulk", "Expedited"] {
        let (code, _stdout, stderr) = run(s7cmd_cmd().args([
            "restore-object",
            "--target-profile",
            PROFILE,
            "--target-region",
            REGION,
            "--days",
            "1",
            "--tier",
            tier,
            &object_arg,
        ]));
        // Exit 1 is expected (InvalidObjectState) — we just need to
        // confirm the tier value parses (i.e. exit code is not 2).
        assert_ne!(
            code,
            Some(2),
            "tier {tier} should not trigger clap parse error; stderr: {stderr}"
        );
    }

    helper.delete_bucket_with_cascade(&bucket).await;
}
