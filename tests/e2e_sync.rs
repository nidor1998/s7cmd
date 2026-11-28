//! Process-level e2e tests for the `sync` fixes that arrived with
//! s3sync 1.64.1 (nidor1998/s3sync#267), against real S3.
//!
//! The same fixes are pinned without AWS in `cli_sync_fake_s3.rs` and
//! `cli_sync_local.rs`; these tests confirm that real S3 agrees with the
//! fake endpoint used there (literal keys, `If-Match` on `CopyObject`,
//! single-part ETags, delete markers, origin metadata).
//!
//! Gated by cfg(e2e_test) — hits real AWS via the s7cmd-e2e-test profile.

#![cfg(e2e_test)]

mod common;

use common::{
    PROFILE_NAME, REGION, TestHelper, create_temp_dir, create_test_file, generate_bucket_name, run,
    s7cmd_cmd,
};

const MIB: usize = 1024 * 1024;

/// Run `s7cmd sync -v <args>`. `-v` prints the `sync summary` line the
/// tests read the statistics from.
fn run_sync(args: &[&str]) -> (Option<i32>, String, String) {
    run(s7cmd_cmd().args(["sync", "-v"]).args(args))
}

/// Value of `name=` in the `sync summary` log line.
fn summary_value(stdout: &str, name: &str) -> u64 {
    let summary = stdout
        .lines()
        .find(|line| line.contains("sync summary"))
        .unwrap_or_else(|| panic!("no sync summary line; stdout={stdout}"));
    let field = format!(" {name}=");
    let start = summary
        .find(&field)
        .unwrap_or_else(|| panic!("no {name} in summary: {summary}"))
        + field.len();
    summary[start..]
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap_or_else(|_| panic!("{name} is not a number in summary: {summary}"))
}

fn dir_arg(dir: &std::path::Path) -> String {
    format!("{}/", dir.to_str().unwrap())
}

/// The prefix of an S3 path is used exactly as given: `?`, `#` and percent
/// escapes all end up in the key, in both directions.
#[tokio::test]
async fn sync_uses_s3_prefix_exactly_as_given() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let prefix = "data#old?x/%20/";
    // What the prefix was truncated or decoded to before the fix. An object
    // here must never be touched.
    helper
        .put_object(&bucket, "data/decoy.txt", b"decoy".to_vec())
        .await;

    let upload_dir = create_temp_dir();
    create_test_file(&upload_dir, "a.txt", b"hello a");
    let s3_path = format!("s3://{bucket}/{prefix}");

    let (code, stdout, stderr) = run_sync(&[
        "--target-profile",
        PROFILE_NAME,
        "--target-region",
        REGION,
        &dir_arg(&upload_dir),
        &s3_path,
    ]);
    assert_eq!(
        code,
        Some(0),
        "upload must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    let key = format!("{prefix}a.txt");
    assert!(
        helper.is_object_exist(&bucket, &key, None).await,
        "the object must be stored under the literal key {key}"
    );
    assert_eq!(
        helper.get_object_bytes(&bucket, &key, None).await,
        b"hello a"
    );

    let download_dir = create_temp_dir();
    let (code, stdout, stderr) = run_sync(&[
        "--source-profile",
        PROFILE_NAME,
        "--source-region",
        REGION,
        &s3_path,
        &dir_arg(&download_dir),
    ]);
    assert_eq!(
        code,
        Some(0),
        "download must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert_eq!(
        summary_value(&stdout, "transferred_object"),
        1,
        "stdout={stdout}"
    );
    assert_eq!(
        std::fs::read(download_dir.join("a.txt")).unwrap(),
        b"hello a"
    );
    assert!(!download_dir.join("decoy.txt").exists());

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&upload_dir);
    let _ = std::fs::remove_dir_all(&download_dir);
}

/// `--metadata` keeps the origin metadata added by
/// `--put-last-modified-metadata`.
#[tokio::test]
async fn sync_metadata_option_keeps_last_modified_origin_metadata() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    create_test_file(&local_dir, "a.txt", b"hello a");

    let (code, stdout, stderr) = run_sync(&[
        "--target-profile",
        PROFILE_NAME,
        "--target-region",
        REGION,
        "--metadata",
        "key1=value1",
        "--put-last-modified-metadata",
        &dir_arg(&local_dir),
        &format!("s3://{bucket}/"),
    ]);
    assert_eq!(code, Some(0), "stdout={stdout}\nstderr={stderr}");

    let head = helper.head_object(&bucket, "a.txt", None).await;
    let metadata = head.metadata().expect("object metadata");
    assert_eq!(metadata.get("key1").map(String::as_str), Some("value1"));
    assert!(
        metadata.contains_key("s3sync_origin_last_modified"),
        "metadata: {metadata:?}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

/// With `--enable-versioning`, `--metadata` used to drop the origin version
/// ID, so the same versions were synced again on every run. The second run
/// must find every version already synced.
#[tokio::test]
async fn sync_versioning_with_metadata_option_does_not_resync() {
    let helper = TestHelper::new().await;
    let source_bucket = generate_bucket_name();
    let target_bucket = generate_bucket_name();
    helper.create_bucket(&source_bucket, REGION).await;
    helper.create_bucket(&target_bucket, REGION).await;
    helper.enable_bucket_versioning(&source_bucket).await;
    helper.enable_bucket_versioning(&target_bucket).await;

    helper
        .put_object(&source_bucket, "a.txt", b"hello a".to_vec())
        .await;
    helper
        .put_object(&source_bucket, "b.txt", b"hello b".to_vec())
        .await;

    let source = format!("s3://{source_bucket}/");
    let target = format!("s3://{target_bucket}/");
    for (run_number, expected_transferred) in [(1, 2), (2, 0)] {
        let (code, stdout, stderr) = run_sync(&[
            "--source-profile",
            PROFILE_NAME,
            "--target-profile",
            PROFILE_NAME,
            "--enable-versioning",
            "--metadata",
            "key1=value1",
            &source,
            &target,
        ]);
        assert_eq!(
            code,
            Some(0),
            "run {run_number}: stdout={stdout}\nstderr={stderr}"
        );
        assert_eq!(
            summary_value(&stdout, "transferred_object"),
            expected_transferred,
            "run {run_number}: stdout={stdout}"
        );

        let versions = helper
            .client
            .list_object_versions()
            .bucket(&target_bucket)
            .send()
            .await
            .expect("list_object_versions");
        assert_eq!(
            versions.versions().len(),
            2,
            "run {run_number}: no version may be synced twice"
        );

        let head = helper.head_object(&target_bucket, "a.txt", None).await;
        let metadata = head.metadata().expect("object metadata");
        assert_eq!(metadata.get("key1").map(String::as_str), Some("value1"));
        assert!(
            metadata.contains_key("s3sync_origin_version_id"),
            "run {run_number}: metadata: {metadata:?}"
        );
        assert!(
            metadata.contains_key("s3sync_origin_last_modified"),
            "run {run_number}: metadata: {metadata:?}"
        );
    }

    helper.delete_bucket_with_cascade(&source_bucket).await;
    helper.delete_bucket_with_cascade(&target_bucket).await;
}

/// `--if-match` with `--server-side-copy` now sends `If-Match` on the
/// single-part `CopyObject` request. With the current target ETag the
/// precondition holds, so the copy must succeed and replace the object.
#[tokio::test]
async fn sync_server_side_copy_with_if_match_replaces_target() {
    let helper = TestHelper::new().await;
    let source_bucket = generate_bucket_name();
    let target_bucket = generate_bucket_name();
    helper.create_bucket(&source_bucket, REGION).await;
    helper.create_bucket(&target_bucket, REGION).await;

    // The target object exists first, so the source object is newer.
    helper
        .put_object(&target_bucket, "a.txt", b"old".to_vec())
        .await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    helper
        .put_object(&source_bucket, "a.txt", b"new content".to_vec())
        .await;

    let (code, stdout, stderr) = run_sync(&[
        "--source-profile",
        PROFILE_NAME,
        "--target-profile",
        PROFILE_NAME,
        "--server-side-copy",
        "--if-match",
        &format!("s3://{source_bucket}/"),
        &format!("s3://{target_bucket}/"),
    ]);
    assert_eq!(code, Some(0), "stdout={stdout}\nstderr={stderr}");
    assert_eq!(
        summary_value(&stdout, "transferred_object"),
        1,
        "stdout={stdout}"
    );
    assert_eq!(
        helper.get_object_bytes(&target_bucket, "a.txt", None).await,
        b"new content"
    );

    helper.delete_bucket_with_cascade(&source_bucket).await;
    helper.delete_bucket_with_cascade(&target_bucket).await;
}

/// An object uploaded in a single part has a plain MD5 ETag even when it is
/// not smaller than the multipart threshold of the checking run.
/// `--check-etag --auto-chunksize` must digest the whole local file and
/// find the object unmodified.
#[tokio::test]
async fn sync_check_etag_auto_chunksize_skips_unmodified_single_part_object() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    // 9MiB: above the default 8MiB threshold. `put_object` uploads it in a
    // single part.
    let body = vec![b'x'; 9 * MIB];
    helper.put_object(&bucket, "big.dat", body.clone()).await;
    let local_dir = create_temp_dir();
    create_test_file(&local_dir, "big.dat", &body);

    let (code, stdout, stderr) = run_sync(&[
        "--target-profile",
        PROFILE_NAME,
        "--target-region",
        REGION,
        "--check-etag",
        "--auto-chunksize",
        &dir_arg(&local_dir),
        &format!("s3://{bucket}/"),
    ]);
    assert_eq!(code, Some(0), "stdout={stdout}\nstderr={stderr}");
    assert_eq!(
        summary_value(&stdout, "transferred_object"),
        0,
        "an unmodified object must not be uploaded; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

/// With `--enable-versioning`, a delete marker reaches the Lua filter
/// callback. Its checksum fields are `nil`; reading them used to panic.
#[tokio::test]
async fn sync_versioning_delete_marker_reaches_lua_filter_callback() {
    let helper = TestHelper::new().await;
    let source_bucket = generate_bucket_name();
    let target_bucket = generate_bucket_name();
    helper.create_bucket(&source_bucket, REGION).await;
    helper.create_bucket(&target_bucket, REGION).await;
    helper.enable_bucket_versioning(&source_bucket).await;
    helper.enable_bucket_versioning(&target_bucket).await;

    helper
        .put_object(&source_bucket, "gone.txt", b"deleted later".to_vec())
        .await;
    // No version ID: this adds a delete marker.
    helper.delete_object(&source_bucket, "gone.txt", None).await;
    helper
        .put_object(&source_bucket, "kept.txt", b"kept".to_vec())
        .await;

    let script_dir = create_temp_dir();
    let script = create_test_file(
        &script_dir,
        "filter.lua",
        br#"function filter(source_object)
    print("FILTER key=" .. source_object.key
        .. " is_delete_marker=" .. tostring(source_object.is_delete_marker)
        .. " checksum_algorithm=" .. tostring(source_object.checksum_algorithm)
        .. " checksum_type=" .. tostring(source_object.checksum_type))
    return true
end
"#,
    );

    let (code, stdout, stderr) = run_sync(&[
        "--source-profile",
        PROFILE_NAME,
        "--target-profile",
        PROFILE_NAME,
        "--enable-versioning",
        "--filter-callback-lua-script",
        script.to_str().unwrap(),
        &format!("s3://{source_bucket}/"),
        &format!("s3://{target_bucket}/"),
    ]);
    assert!(
        !stdout.contains("panicked") && !stderr.contains("panicked"),
        "stdout={stdout}\nstderr={stderr}"
    );
    assert_eq!(code, Some(0), "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains(
            "FILTER key=gone.txt is_delete_marker=true checksum_algorithm=nil checksum_type=nil"
        ),
        "the delete marker must reach the filter with nil checksum fields; stdout={stdout}"
    );

    // The delete marker is replayed: the key is gone in the target too.
    assert!(
        !helper
            .is_object_exist(&target_bucket, "gone.txt", None)
            .await
    );
    assert!(
        helper
            .is_object_exist(&target_bucket, "kept.txt", None)
            .await
    );

    helper.delete_bucket_with_cascade(&source_bucket).await;
    helper.delete_bucket_with_cascade(&target_bucket).await;
    let _ = std::fs::remove_dir_all(&script_dir);
}
