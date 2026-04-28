//! Process-level e2e tests for bucket lifecycle subcommands.

#![cfg(e2e_test)]

mod common;

use common::{REGION, TestHelper, generate_bucket_name, run, s7cmd_cmd};

// ---- create-bucket ----

#[tokio::test]
async fn create_bucket_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");

    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "create-bucket",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "create-bucket must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(helper.is_bucket_exist(&bucket).await);

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn create_bucket_dispatch_with_tagging() {
    // Exercises the `Some(raw_tagging) =>` arm that parses the tag string,
    // builds a Tagging payload, and issues PutBucketTagging after the bucket
    // is created.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");

    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "create-bucket",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--tagging",
        "owner=team-a&env=test",
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "create-bucket --tagging must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(helper.is_bucket_exist(&bucket).await);

    helper.delete_bucket_with_cascade(&bucket).await;
}

// ---- head-bucket ----

#[tokio::test]
async fn head_bucket_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "head-bucket",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "head-bucket must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn head_bucket_dispatch_not_found() {
    // Don't create the bucket — assert NotFound.
    let bucket = generate_bucket_name();
    let target = format!("s3://{bucket}");

    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "head-bucket",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(code, Some(4), "head-bucket on missing bucket must exit 4");
}

// ---- delete-bucket ----

#[tokio::test]
async fn delete_bucket_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "delete-bucket",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "delete-bucket must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    // Don't SDK-verify the bucket is gone: HeadBucket against a just-deleted
    // bucket can briefly return 200 due to S3's DNS/routing eventual
    // consistency window. The exit-0 assertion above already proves the
    // dispatch reached delete-bucket and the API call succeeded.
}

#[tokio::test]
async fn delete_bucket_dispatch_error_not_empty() {
    // S3 returns BucketNotEmpty when delete-bucket runs against a bucket
    // with objects. The dispatch arm maps any non-NotFound runtime error
    // to EXIT_CODE_ERROR (1).
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper
        .put_object(&bucket, "blocker.txt", b"blocks delete".to_vec())
        .await;

    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "delete-bucket",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(1),
        "delete-bucket on non-empty bucket must exit 1"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}
