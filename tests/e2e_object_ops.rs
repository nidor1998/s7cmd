//! Process-level e2e tests for object-operation subcommands.
//!
//! Gated by cfg(e2e_test) — hits real AWS via the s7cmd-e2e-test profile.

#![cfg(e2e_test)]

mod common;

use common::{
    REGION, TestHelper, create_temp_dir, create_test_file, generate_bucket_name, run, s7cmd_cmd,
};

// ---- sync ----

#[tokio::test]
async fn sync_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    create_test_file(&local_dir, "a.txt", b"hello a");
    create_test_file(&local_dir, "b.txt", b"hello b");

    let target = format!("s3://{bucket}/");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "sync",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        local_dir.to_str().unwrap(),
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "sync must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(helper.is_object_exist(&bucket, "a.txt", None).await);
    assert!(helper.is_object_exist(&bucket, "b.txt", None).await);

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn sync_dispatch_warning_etag_mismatch() {
    // Seed S3 with an object whose body differs from what we'll sync up.
    // sync --check-etag detects the mismatch and exits 3 (warning).
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper
        .put_object(&bucket, "a.txt", b"old contents".to_vec())
        .await;

    let local_dir = create_temp_dir();
    create_test_file(&local_dir, "a.txt", b"new contents");

    let target = format!("s3://{bucket}/");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "sync",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--check-etag",
        "--head-each-target",
        "--dry-run",
        local_dir.to_str().unwrap(),
        &target,
    ]));

    assert_eq!(
        code,
        Some(3),
        "sync --check-etag with mismatch must exit 3; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

// ---- ls ----

#[tokio::test]
async fn ls_dispatch_success_buckets() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "ls",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
    ]));

    assert_eq!(
        code,
        Some(0),
        "ls (buckets) must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(&bucket),
        "ls output must mention the test bucket {bucket}; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn ls_dispatch_success_objects() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper.put_object(&bucket, "a.txt", b"a".to_vec()).await;
    helper.put_object(&bucket, "b.txt", b"b".to_vec()).await;

    let target = format!("s3://{bucket}/");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "ls",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "ls (objects) must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("a.txt"));
    assert!(stdout.contains("b.txt"));

    helper.delete_bucket_with_cascade(&bucket).await;
}

// ---- clean ----

#[tokio::test]
async fn clean_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper.put_object(&bucket, "a.txt", b"a".to_vec()).await;
    helper.put_object(&bucket, "b.txt", b"b".to_vec()).await;

    let target = format!("s3://{bucket}/");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "clean",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--force",
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "clean must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(!helper.is_object_exist(&bucket, "a.txt", None).await);
    assert!(!helper.is_object_exist(&bucket, "b.txt", None).await);

    helper.delete_bucket_with_cascade(&bucket).await;
}

// ---- cp ----

#[tokio::test]
async fn cp_dispatch_success_local_to_s3() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let body = b"cp local body";
    let src = create_test_file(&local_dir, "cp.txt", body);
    let target = format!("s3://{bucket}/cp.txt");

    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "cp",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        src.to_str().unwrap(),
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "cp local→s3 must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(helper.is_object_exist(&bucket, "cp.txt", None).await);

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn cp_dispatch_success_s3_to_local() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    let body = b"cp s3 body".to_vec();
    helper.put_object(&bucket, "cp.txt", body.clone()).await;

    let local_dir = create_temp_dir();
    let dst = local_dir.join("cp.txt");
    let source = format!("s3://{bucket}/cp.txt");

    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "cp",
        "--source-profile",
        "s7cmd-e2e-test",
        "--source-region",
        REGION,
        &source,
        dst.to_str().unwrap(),
    ]));

    assert_eq!(
        code,
        Some(0),
        "cp s3→local must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    let downloaded = std::fs::read(&dst).unwrap();
    assert_eq!(downloaded, body);

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

// ---- mv ----

#[tokio::test]
async fn mv_dispatch_success() {
    let helper = TestHelper::new().await;
    let src_bucket = generate_bucket_name();
    let dst_bucket = generate_bucket_name();
    helper.create_bucket(&src_bucket, REGION).await;
    helper.create_bucket(&dst_bucket, REGION).await;
    helper
        .put_object(&src_bucket, "mv.txt", b"mv body".to_vec())
        .await;

    let source = format!("s3://{src_bucket}/mv.txt");
    let target = format!("s3://{dst_bucket}/mv.txt");

    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "mv",
        "--source-profile",
        "s7cmd-e2e-test",
        "--source-region",
        REGION,
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &source,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "mv must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(!helper.is_object_exist(&src_bucket, "mv.txt", None).await);
    assert!(helper.is_object_exist(&dst_bucket, "mv.txt", None).await);

    helper.delete_bucket_with_cascade(&src_bucket).await;
    helper.delete_bucket_with_cascade(&dst_bucket).await;
}

// ---- rm ----

#[tokio::test]
async fn rm_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper
        .put_object(&bucket, "rm.txt", b"rm body".to_vec())
        .await;

    let target = format!("s3://{bucket}/rm.txt");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "rm",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "rm must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(!helper.is_object_exist(&bucket, "rm.txt", None).await);

    helper.delete_bucket_with_cascade(&bucket).await;
}
