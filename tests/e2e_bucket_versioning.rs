//! Process-level e2e tests for bucket-versioning subcommands.

#![cfg(e2e_test)]

mod common;

use common::{REGION, TestHelper, generate_bucket_name, run, s7cmd_cmd};

#[tokio::test]
async fn put_bucket_versioning_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-versioning",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--enabled",
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "put-bucket-versioning must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_bucket_versioning_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper.enable_bucket_versioning(&bucket).await;

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-bucket-versioning",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "get-bucket-versioning must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("Enabled"),
        "stdout must contain 'Enabled'; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_bucket_versioning_dispatch_unconfigured_bucket() {
    // Bucket exists but versioning was never set — S3 returns no `Status`
    // and the empty-object branch logs `Bucket versioning not configured.`
    // Exits 0 (Success) and prints nothing on stdout.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-bucket-versioning",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "get-bucket-versioning on unconfigured bucket must exit 0; stderr={stderr}"
    );
    assert!(
        stdout.trim().is_empty(),
        "stdout must be empty when versioning is not configured; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_bucket_versioning_dispatch_bucket_not_found() {
    // Hits the `BucketNotFound | NotFound` arm.
    let bucket = generate_bucket_name();

    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-versioning",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(4),
        "get-bucket-versioning on missing bucket must exit 4"
    );
}
