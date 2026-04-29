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

// Note: a sync exit-3 (warning) test is intentionally omitted.
// `--check-etag` with a content mismatch treats the situation as "object
// needs sync, transfer it" rather than emitting a warning, and `--dry-run`
// reports `[dry-run] sync completed` with `warning=0` — both yield exit 0.
// Reliable warning-path triggering would require `--report-sync-status`
// or specific Glacier-class scenarios beyond a dispatch test's scope. The
// spec's section-12 follow-up explicitly authorized dropping this case if
// the configuration didn't surface exit 3 in the maintainer's environment.

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
async fn clean_dispatch_error_no_force_non_interactive() {
    // Without --force the safety check prompts for confirmation. Cargo runs
    // tests with stdin closed, so `is_interactive()` returns false and the
    // safety check returns `S3rmError::InvalidConfig` — exits with code 2.
    // Covers the prerequisite-error branch in clean_bin/mod.rs (close
    // stats sender → exit_code_from_error → process::exit).
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}/");
    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "clean",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(2),
        "clean without --force in non-interactive must exit 2; stderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

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

#[tokio::test]
async fn cp_dispatch_error_source_missing() {
    // S3 source object doesn't exist — transfer fails inside run_copy_phase.
    // Hits cp.rs's `Err(e) =>` arm which returns the error (exit 1).
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let dst = local_dir.join("never.txt");
    let source = format!("s3://{bucket}/never-existed.txt");

    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
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
        Some(1),
        "cp with missing source object must exit 1 (transfer error)"
    );

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

#[tokio::test]
async fn mv_dispatch_error_source_missing() {
    // Source object missing — transfer fails inside run_copy_phase.
    // Hits mv.rs's gate-2 `Err(e) =>` arm which returns the error.
    let helper = TestHelper::new().await;
    let src_bucket = generate_bucket_name();
    let dst_bucket = generate_bucket_name();
    helper.create_bucket(&src_bucket, REGION).await;
    helper.create_bucket(&dst_bucket, REGION).await;

    let source = format!("s3://{src_bucket}/missing.txt");
    let target = format!("s3://{dst_bucket}/missing.txt");

    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
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
        Some(1),
        "mv with missing source object must exit 1 (transfer error, source not deleted)"
    );

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
