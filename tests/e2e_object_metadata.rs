//! Process-level e2e tests for object-metadata subcommands.

#![cfg(e2e_test)]

mod common;

use common::{REGION, TestHelper, generate_bucket_name, run, s7cmd_cmd};

// ---- head-object ----

#[tokio::test]
async fn head_object_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper
        .put_object(&bucket, "head.txt", b"head body".to_vec())
        .await;

    let target = format!("s3://{bucket}/head.txt");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "head-object",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "head-object must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn head_object_dispatch_not_found() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}/does-not-exist.txt");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "head-object",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(code, Some(4), "head-object on missing key must exit 4");

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn head_object_dispatch_not_found_with_version_id() {
    // Hits the `Some(v) =>` arm that logs `(versionId=...) not found`.
    // S3 rejects malformed version-ids with `InvalidArgument` (Other/exit 1)
    // rather than `NoSuchVersion`, so we have to use a real version-id whose
    // version we then permanently delete — then S3 returns 404 on lookup.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper.enable_bucket_versioning(&bucket).await;

    let key = "key.txt";
    let version_id = helper
        .put_object_returning_version_id(&bucket, key, b"v1".to_vec())
        .await;
    helper
        .delete_object(&bucket, key, Some(version_id.clone()))
        .await;

    let target = format!("s3://{bucket}/{key}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "head-object",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--source-version-id",
        &version_id,
        &target,
    ]));

    assert_eq!(
        code,
        Some(4),
        "head-object on a deleted version-id must exit 4"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn head_object_dispatch_bucket_not_found() {
    // Hits the `BucketNotFound` arm that logs `bucket … not found`.
    let bucket = generate_bucket_name(); // never created

    let target = format!("s3://{bucket}/key.txt");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "head-object",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(4),
        "head-object against missing bucket must exit 4"
    );
}

// ---- put-object-tagging ----

#[tokio::test]
async fn put_object_tagging_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper
        .put_object(&bucket, "tag.txt", b"tag body".to_vec())
        .await;

    let target = format!("s3://{bucket}/tag.txt");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "put-object-tagging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--tagging",
        "k1=v1&k2=v2",
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "put-object-tagging must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

// ---- get-object-tagging error paths ----

#[tokio::test]
async fn get_object_tagging_dispatch_object_not_found() {
    // Object missing — hits the `NotFound => None` arm.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}/never-existed.txt");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-object-tagging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(4),
        "get-object-tagging on missing object must exit 4"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_object_tagging_dispatch_object_not_found_with_version_id() {
    // Hits the `Some(v) =>` arm with `(versionId=...) not found`.
    // S3 rejects malformed version-ids with `InvalidArgument` (mapped to
    // `Other`, exit 1) rather than `NoSuchVersion`, so we have to use a
    // real version-id whose version we then permanently delete.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper.enable_bucket_versioning(&bucket).await;

    let key = "tag.txt";
    let version_id = helper
        .put_object_returning_version_id(&bucket, key, b"v1".to_vec())
        .await;
    helper
        .delete_object(&bucket, key, Some(version_id.clone()))
        .await;

    let target = format!("s3://{bucket}/{key}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-object-tagging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--source-version-id",
        &version_id,
        &target,
    ]));

    assert_eq!(
        code,
        Some(4),
        "get-object-tagging on a deleted version-id must exit 4"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_object_tagging_dispatch_bucket_not_found() {
    // Hits the `BucketNotFound` arm.
    let bucket = generate_bucket_name();

    let target = format!("s3://{bucket}/key.txt");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-object-tagging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(4),
        "get-object-tagging against missing bucket must exit 4"
    );
}

// ---- get-object-tagging ----

#[tokio::test]
async fn get_object_tagging_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper
        .put_object(&bucket, "tag.txt", b"tag body".to_vec())
        .await;
    helper
        .put_object_tagging(&bucket, "tag.txt", &[("seed-key", "seed-value")])
        .await;

    let target = format!("s3://{bucket}/tag.txt");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-object-tagging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "get-object-tagging must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("seed-key") && stdout.contains("seed-value"),
        "stdout must contain seeded tag; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

// ---- delete-object-tagging ----

#[tokio::test]
async fn delete_object_tagging_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper
        .put_object(&bucket, "tag.txt", b"tag body".to_vec())
        .await;
    helper
        .put_object_tagging(&bucket, "tag.txt", &[("k", "v")])
        .await;

    let target = format!("s3://{bucket}/tag.txt");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "delete-object-tagging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "delete-object-tagging must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}
