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

// ---- create-bucket --if-not-exists (s3util-rs 1.2.0 idempotency flag) ----

#[tokio::test]
async fn create_bucket_if_not_exists_with_existing_bucket_skips() {
    // Bucket already exists → HeadBucket pre-flight reports OK → skip
    // branch returns ExitStatus::Success without issuing CreateBucket.
    // Bucket remains intact (no rename, no error, idempotent re-run).
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    assert!(helper.is_bucket_exist(&bucket).await);

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "create-bucket",
        "--if-not-exists",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "create-bucket --if-not-exists on existing bucket must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        helper.is_bucket_exist(&bucket).await,
        "bucket must still exist after the no-op create"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn create_bucket_if_not_exists_with_missing_bucket_creates() {
    // Bucket doesn't exist → HeadBucket returns BucketNotFound → falls
    // through to the normal CreateBucket flow and the bucket is created.
    //
    // Pre-flight `assert!(!helper.is_bucket_exist(...))` is intentionally
    // omitted: `generate_bucket_name()` is UUID-unique so the bucket
    // genuinely cannot exist beforehand, and a pre-flight HeadBucket on
    // the test's persistent SDK client primes S3's bucket-NotFound
    // negative cache, causing the post-create HeadBucket to read stale
    // and return false even though CreateBucket succeeded.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "create-bucket",
        "--if-not-exists",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "create-bucket --if-not-exists on missing bucket must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        helper.is_bucket_exist(&bucket).await,
        "bucket must exist after fall-through CreateBucket"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn create_bucket_if_not_exists_skips_tagging_on_existing_bucket() {
    // Per the upstream rationale: when the bucket already exists, the
    // --tagging branch is intentionally skipped. We do not retroactively
    // tag a bucket this invocation didn't create. Verified by issuing
    // create-bucket --if-not-exists --tagging against a pre-existing,
    // un-tagged bucket and then asserting (via `s7cmd get-bucket-tagging`)
    // that no tag set was added — exit code 4 = NotFound = NoSuchTagSet.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "create-bucket",
        "--if-not-exists",
        "--tagging",
        "owner=team-a&env=test",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));
    assert_eq!(
        code,
        Some(0),
        "create-bucket --if-not-exists --tagging on existing bucket must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    let (tag_code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-tagging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));
    assert_eq!(
        tag_code,
        Some(4),
        "get-bucket-tagging must report NotFound (4): the existing bucket must not have been retroactively tagged"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn create_bucket_if_not_exists_with_missing_bucket_applies_tagging() {
    // Counterpart to the skip-tagging test: when the bucket is freshly
    // created (i.e. the fall-through CreateBucket path runs), --tagging
    // IS applied as usual. (Pre-flight existence check skipped — see
    // the notes on create_bucket_if_not_exists_with_missing_bucket_creates.)
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "create-bucket",
        "--if-not-exists",
        "--tagging",
        "stage=fresh&team=sre",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));
    assert_eq!(
        code,
        Some(0),
        "create-bucket --if-not-exists --tagging on missing bucket must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    let (tag_code, tag_stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-tagging",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));
    assert_eq!(
        tag_code,
        Some(0),
        "get-bucket-tagging must succeed on the freshly-created bucket"
    );
    assert!(
        tag_stdout.contains("stage") && tag_stdout.contains("fresh"),
        "expected seeded tag in get-bucket-tagging output: {tag_stdout}"
    );

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
