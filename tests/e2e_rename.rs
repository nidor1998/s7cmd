//! E2E tests for `s7cmd rename`.
//!
//! These tests hit real AWS and are reserved for the user to run. The
//! CI executor only compile-checks them (under `RUSTFLAGS="--cfg e2e_test"`)
//! to keep rename's end-to-end behavior covered without spending money or
//! touching production buckets in CI.
//!
//! Run with:
//! ```sh
//! RUSTFLAGS="--cfg e2e_test" cargo test --test e2e_rename -- --test-threads=1
//! ```
//!
//! Requires the `s7cmd-e2e-test` AWS profile to be configured.
//!
//! Coverage:
//! - Basic rename: source key moves to destination, source no longer exists.
//! - Content preservation: renamed object's bytes match the original.
//! - Source-not-found: exit 1, no crash.
//! - Dry run: no actual rename, exit 0, `[dry-run]` in stderr.
//! - Conditional `--source-if-match`: matching ETag succeeds, wrong ETag fails.
//! - Conditional `--source-if-none-match`: passing `*` means "rename only if
//!   source has no ETag"; object exists so the condition fails with 412
//!   (precondition failure → exit 1).
//! - Conditional `--target-if-none-match`: passing `*` means "rename only if
//!   destination has no ETag"; destination absent → succeeds;
//!   destination present → precondition failure → exit 1.
//! - Conditional `--target-if-match`: matching destination ETag succeeds;
//!   wrong ETag fails.
//! - Special characters in key: spaces, slashes, Unicode percent-encoded correctly.
//! - CLI validation: non-Express-One-Zone bucket name → exit 2.
//! - CLI validation: source and target in different buckets → exit 2.

#![cfg(e2e_test)]

mod common;

use common::{EXPRESS_ONE_ZONE_AZ, PROFILE_NAME, TestHelper};

use std::process::{Command, Output, Stdio};

/// Suffix appended to base bucket names to form Express One Zone directory bucket names.
const EXPRESS_ONE_ZONE_BUCKET_SUFFIX: &str = "--apne1-az4--x-s3";

const EXIT_CODE_SUCCESS: i32 = 0;
const EXIT_CODE_ERROR: i32 = 1;
const EXIT_CODE_CLAP_ARG_ERROR: i32 = 2;

fn run_s7cmd(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_s7cmd"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn s7cmd")
}

/// Create a fresh Express One Zone directory bucket, put a small object
/// into it with the SDK, and return `(bucket_name, etag)`.
async fn setup_bucket_with_object(helper: &TestHelper, key: &str, body: &[u8]) -> (String, String) {
    let bucket = format!(
        "s7e2e-{}{}",
        uuid::Uuid::new_v4(),
        EXPRESS_ONE_ZONE_BUCKET_SUFFIX
    );
    helper
        .create_directory_bucket(&bucket, EXPRESS_ONE_ZONE_AZ)
        .await;
    helper.put_object(&bucket, key, body.to_vec()).await;
    let head = helper.head_object(&bucket, key, None).await;
    let etag = head.e_tag().unwrap().to_string();
    (bucket, etag)
}

// ---------------------------------------------------------------
// Basic rename (3 tests)
// ---------------------------------------------------------------

/// Happy path: rename an object within the same Express One Zone bucket.
/// After rename: destination exists, source is gone.
#[tokio::test]
async fn rename_basic_source_gone_destination_present() {
    let helper = TestHelper::new().await;
    let src_key = "src/basic.txt";
    let dst_key = "dst/basic.txt";
    let body = b"rename basic test body";
    let (bucket, _) = setup_bucket_with_object(&helper, src_key, body).await;

    let src = format!("s3://{}/{}", bucket, src_key);
    let dst = format!("s3://{}/{}", bucket, dst_key);

    let output = run_s7cmd(&["rename", "--source-profile", PROFILE_NAME, &src, &dst]);

    assert_eq!(
        output.status.code(),
        Some(EXIT_CODE_SUCCESS),
        "rename must exit 0; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        !helper.is_object_exist(&bucket, src_key, None).await,
        "source must be gone after rename"
    );
    assert!(
        helper.is_object_exist(&bucket, dst_key, None).await,
        "destination must exist after rename"
    );

    helper.delete_directory_bucket_with_cascade(&bucket).await;
}

/// Rename preserves object content: bytes at destination match original.
#[tokio::test]
async fn rename_preserves_content() {
    let helper = TestHelper::new().await;
    let src_key = "preserve_src.txt";
    let dst_key = "preserve_dst.txt";
    let body = b"content to be preserved across rename";
    let (bucket, _) = setup_bucket_with_object(&helper, src_key, body).await;

    let src = format!("s3://{}/{}", bucket, src_key);
    let dst = format!("s3://{}/{}", bucket, dst_key);

    let output = run_s7cmd(&["rename", "--source-profile", PROFILE_NAME, &src, &dst]);

    assert_eq!(
        output.status.code(),
        Some(EXIT_CODE_SUCCESS),
        "rename must exit 0; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let dst_bytes = helper.get_object_bytes(&bucket, dst_key, None).await;
    assert_eq!(
        dst_bytes, body,
        "renamed object content must match original"
    );

    helper.delete_directory_bucket_with_cascade(&bucket).await;
}

/// Rename when the source key does not exist must exit 1 without panicking.
#[tokio::test]
async fn rename_missing_source_exits_error() {
    let helper = TestHelper::new().await;
    let bucket = format!(
        "s7e2e-{}{}",
        uuid::Uuid::new_v4(),
        EXPRESS_ONE_ZONE_BUCKET_SUFFIX
    );
    helper
        .create_directory_bucket(&bucket, EXPRESS_ONE_ZONE_AZ)
        .await;

    let src = format!("s3://{}/nonexistent.txt", bucket);
    let dst = format!("s3://{}/dest.txt", bucket);

    let output = run_s7cmd(&["rename", "--source-profile", PROFILE_NAME, &src, &dst]);

    assert_eq!(
        output.status.code(),
        Some(EXIT_CODE_ERROR),
        "rename of nonexistent source must exit 1; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    helper.delete_directory_bucket_with_cascade(&bucket).await;
}

// ---------------------------------------------------------------
// Dry run (1 test)
// ---------------------------------------------------------------

/// `--dry-run` must exit 0, emit `[dry-run]` in stderr, and leave both
/// source present and destination absent (no actual rename performed).
#[tokio::test]
async fn rename_dry_run_no_side_effects() {
    let helper = TestHelper::new().await;
    let src_key = "dryrun_src.txt";
    let dst_key = "dryrun_dst.txt";
    let body = b"dry run body";
    let (bucket, _) = setup_bucket_with_object(&helper, src_key, body).await;

    let src = format!("s3://{}/{}", bucket, src_key);
    let dst = format!("s3://{}/{}", bucket, dst_key);

    let output = run_s7cmd(&[
        "rename",
        "--source-profile",
        PROFILE_NAME,
        "--dry-run",
        &src,
        &dst,
    ]);

    assert_eq!(
        output.status.code(),
        Some(EXIT_CODE_SUCCESS),
        "rename --dry-run must exit 0; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[dry-run]"),
        "stderr must contain [dry-run] prefix; got: {stderr}"
    );
    assert!(
        helper.is_object_exist(&bucket, src_key, None).await,
        "source must still exist after --dry-run"
    );
    assert!(
        !helper.is_object_exist(&bucket, dst_key, None).await,
        "destination must not exist after --dry-run"
    );

    helper.delete_directory_bucket_with_cascade(&bucket).await;
}

// ---------------------------------------------------------------
// Conditional checks — source-if-match (2 tests)
// ---------------------------------------------------------------

/// `--source-if-match <correct ETag>` must succeed and rename the object.
#[tokio::test]
async fn rename_source_if_match_correct_etag_succeeds() {
    let helper = TestHelper::new().await;
    let src_key = "sim_src.txt";
    let dst_key = "sim_dst.txt";
    let body = b"source-if-match test";
    let (bucket, etag) = setup_bucket_with_object(&helper, src_key, body).await;

    let src = format!("s3://{}/{}", bucket, src_key);
    let dst = format!("s3://{}/{}", bucket, dst_key);

    let output = run_s7cmd(&[
        "rename",
        "--source-profile",
        PROFILE_NAME,
        "--source-if-match",
        &etag,
        &src,
        &dst,
    ]);

    assert_eq!(
        output.status.code(),
        Some(EXIT_CODE_SUCCESS),
        "rename with correct --source-if-match must exit 0; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(!helper.is_object_exist(&bucket, src_key, None).await);
    assert!(helper.is_object_exist(&bucket, dst_key, None).await);

    helper.delete_directory_bucket_with_cascade(&bucket).await;
}

/// `--source-if-match <wrong ETag>` must fail (412 precondition) and
/// leave the source object intact.
#[tokio::test]
async fn rename_source_if_match_wrong_etag_fails() {
    let helper = TestHelper::new().await;
    let src_key = "sim_fail_src.txt";
    let dst_key = "sim_fail_dst.txt";
    let body = b"source-if-match fail test";
    let (bucket, _) = setup_bucket_with_object(&helper, src_key, body).await;

    let src = format!("s3://{}/{}", bucket, src_key);
    let dst = format!("s3://{}/{}", bucket, dst_key);

    let output = run_s7cmd(&[
        "rename",
        "--source-profile",
        PROFILE_NAME,
        "--source-if-match",
        "\"aaaabbbbccccdddd0000111122223333\"",
        &src,
        &dst,
    ]);

    assert_ne!(
        output.status.code(),
        Some(EXIT_CODE_SUCCESS),
        "rename with wrong --source-if-match must not succeed; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        helper.is_object_exist(&bucket, src_key, None).await,
        "source must remain after precondition failure"
    );
    assert!(
        !helper.is_object_exist(&bucket, dst_key, None).await,
        "destination must not be created after precondition failure"
    );

    helper.delete_directory_bucket_with_cascade(&bucket).await;
}

// ---------------------------------------------------------------
// Conditional checks — source-if-none-match (1 test)
// ---------------------------------------------------------------

/// `--source-if-none-match *` means "rename only if the source has no ETag".
/// Because the source object exists it always has an ETag, so the condition
/// is false and the API returns a 412 precondition failure.
/// The source must remain intact.
#[tokio::test]
async fn rename_source_if_none_match_existing_object_fails() {
    let helper = TestHelper::new().await;
    let src_key = "sinm_src.txt";
    let dst_key = "sinm_dst.txt";
    let body = b"source-if-none-match test";
    let (bucket, _) = setup_bucket_with_object(&helper, src_key, body).await;

    let src = format!("s3://{}/{}", bucket, src_key);
    let dst = format!("s3://{}/{}", bucket, dst_key);

    let output = run_s7cmd(&[
        "rename",
        "--source-profile",
        PROFILE_NAME,
        "--source-if-none-match",
        "*",
        &src,
        &dst,
    ]);

    assert_ne!(
        output.status.code(),
        Some(EXIT_CODE_SUCCESS),
        "rename with --source-if-none-match on existing object must not succeed; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        helper.is_object_exist(&bucket, src_key, None).await,
        "source must remain after precondition failure"
    );

    helper.delete_directory_bucket_with_cascade(&bucket).await;
}

// ---------------------------------------------------------------
// Conditional checks — target-if-none-match (2 tests)
// ---------------------------------------------------------------

/// `--target-if-none-match *` means "rename only if the destination has no
/// ETag". When the destination does not exist, no ETag is present, so the
/// condition is true and the rename succeeds.
#[tokio::test]
async fn rename_target_if_none_match_destination_absent_succeeds() {
    let helper = TestHelper::new().await;
    let src_key = "tinm_src.txt";
    let dst_key = "tinm_dst_absent.txt";
    let body = b"target-if-none-match absent test";
    let (bucket, _) = setup_bucket_with_object(&helper, src_key, body).await;

    let src = format!("s3://{}/{}", bucket, src_key);
    let dst = format!("s3://{}/{}", bucket, dst_key);

    let output = run_s7cmd(&[
        "rename",
        "--source-profile",
        PROFILE_NAME,
        "--target-if-none-match",
        "*",
        &src,
        &dst,
    ]);

    assert_eq!(
        output.status.code(),
        Some(EXIT_CODE_SUCCESS),
        "rename with --target-if-none-match on absent destination must succeed; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(helper.is_object_exist(&bucket, dst_key, None).await);
    assert!(!helper.is_object_exist(&bucket, src_key, None).await);

    helper.delete_directory_bucket_with_cascade(&bucket).await;
}

/// `--target-if-none-match *` means "rename only if the destination has no
/// ETag". When the destination already exists it has an ETag, so the
/// condition is false and the API returns 412.
/// Source and destination must remain unchanged.
#[tokio::test]
async fn rename_target_if_none_match_destination_present_fails() {
    let helper = TestHelper::new().await;
    let src_key = "tinm_src2.txt";
    let dst_key = "tinm_dst_present.txt";
    let src_body = b"source body";
    let dst_body = b"pre-existing destination body";
    let (bucket, _) = setup_bucket_with_object(&helper, src_key, src_body).await;
    helper.put_object(&bucket, dst_key, dst_body.to_vec()).await;

    let src = format!("s3://{}/{}", bucket, src_key);
    let dst = format!("s3://{}/{}", bucket, dst_key);

    let output = run_s7cmd(&[
        "rename",
        "--source-profile",
        PROFILE_NAME,
        "--target-if-none-match",
        "*",
        &src,
        &dst,
    ]);

    assert_ne!(
        output.status.code(),
        Some(EXIT_CODE_SUCCESS),
        "rename with --target-if-none-match on present destination must not succeed; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        helper.is_object_exist(&bucket, src_key, None).await,
        "source must remain"
    );
    let dst_bytes = helper.get_object_bytes(&bucket, dst_key, None).await;
    assert_eq!(dst_bytes, dst_body, "destination content must be unchanged");

    helper.delete_directory_bucket_with_cascade(&bucket).await;
}

// ---------------------------------------------------------------
// Conditional checks — target-if-match (2 tests)
// ---------------------------------------------------------------

/// `--target-if-match <correct ETag>` with a pre-existing destination
/// whose ETag matches must overwrite the destination and succeed.
#[tokio::test]
async fn rename_target_if_match_correct_etag_succeeds() {
    let helper = TestHelper::new().await;
    let src_key = "tim_src.txt";
    let dst_key = "tim_dst.txt";
    let src_body = b"source for target-if-match";
    let dst_body = b"existing destination";
    let (bucket, _) = setup_bucket_with_object(&helper, src_key, src_body).await;
    helper.put_object(&bucket, dst_key, dst_body.to_vec()).await;
    let dst_head = helper.head_object(&bucket, dst_key, None).await;
    let dst_etag = dst_head.e_tag().unwrap().to_string();

    let src = format!("s3://{}/{}", bucket, src_key);
    let dst = format!("s3://{}/{}", bucket, dst_key);

    let output = run_s7cmd(&[
        "rename",
        "--source-profile",
        PROFILE_NAME,
        "--target-if-match",
        &dst_etag,
        &src,
        &dst,
    ]);

    assert_eq!(
        output.status.code(),
        Some(EXIT_CODE_SUCCESS),
        "rename with correct --target-if-match must exit 0; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(!helper.is_object_exist(&bucket, src_key, None).await);
    let new_dst_bytes = helper.get_object_bytes(&bucket, dst_key, None).await;
    assert_eq!(
        new_dst_bytes, src_body,
        "destination must now contain source content"
    );

    helper.delete_directory_bucket_with_cascade(&bucket).await;
}

/// `--target-if-match <wrong ETag>` must fail (412) and leave both
/// source and destination unchanged.
#[tokio::test]
async fn rename_target_if_match_wrong_etag_fails() {
    let helper = TestHelper::new().await;
    let src_key = "tim_fail_src.txt";
    let dst_key = "tim_fail_dst.txt";
    let src_body = b"source for target-if-match failure";
    let dst_body = b"existing destination unchanged";
    let (bucket, _) = setup_bucket_with_object(&helper, src_key, src_body).await;
    helper.put_object(&bucket, dst_key, dst_body.to_vec()).await;

    let src = format!("s3://{}/{}", bucket, src_key);
    let dst = format!("s3://{}/{}", bucket, dst_key);

    let output = run_s7cmd(&[
        "rename",
        "--source-profile",
        PROFILE_NAME,
        "--target-if-match",
        "\"aaaabbbbccccdddd0000111122223333\"",
        &src,
        &dst,
    ]);

    assert_ne!(
        output.status.code(),
        Some(EXIT_CODE_SUCCESS),
        "rename with wrong --target-if-match must not succeed; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        helper.is_object_exist(&bucket, src_key, None).await,
        "source must remain"
    );
    let dst_bytes = helper.get_object_bytes(&bucket, dst_key, None).await;
    assert_eq!(dst_bytes, dst_body, "destination content must be unchanged");

    helper.delete_directory_bucket_with_cascade(&bucket).await;
}

// ---------------------------------------------------------------
// Special characters in key (1 test)
// ---------------------------------------------------------------

/// Rename with spaces, slashes, and Unicode characters in the key name.
/// The CLI must percent-encode them correctly when building the
/// `rename_source` header.
#[tokio::test]
async fn rename_special_characters_in_key() {
    let helper = TestHelper::new().await;
    let src_key = "dir/file with spaces & unicode \u{00e9}.txt";
    let dst_key = "dir/renamed \u{00e9} file.txt";
    let body = b"special chars rename body";
    let (bucket, _) = setup_bucket_with_object(&helper, src_key, body).await;

    let src = format!("s3://{}/{}", bucket, src_key);
    let dst = format!("s3://{}/{}", bucket, dst_key);

    let output = run_s7cmd(&["rename", "--source-profile", PROFILE_NAME, &src, &dst]);

    assert_eq!(
        output.status.code(),
        Some(EXIT_CODE_SUCCESS),
        "rename with special-character keys must exit 0; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(!helper.is_object_exist(&bucket, src_key, None).await);
    assert!(helper.is_object_exist(&bucket, dst_key, None).await);

    helper.delete_directory_bucket_with_cascade(&bucket).await;
}

// ---------------------------------------------------------------
// CLI validation — no AWS contact (2 tests)
// ---------------------------------------------------------------

/// Non-Express-One-Zone bucket name (does not end with `--<az>--x-s3`)
/// must be rejected by `validate()` before any AWS call, exiting 2.
#[tokio::test]
async fn rename_non_express_onezone_bucket_exits_2() {
    let output = run_s7cmd(&[
        "rename",
        "--source-profile",
        PROFILE_NAME,
        "s3://my-regular-bucket/source.txt",
        "s3://my-regular-bucket/destination.txt",
    ]);

    assert_eq!(
        output.status.code(),
        Some(EXIT_CODE_CLAP_ARG_ERROR),
        "rename to non-Express-One-Zone bucket must exit 2; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Source and destination in different buckets must be rejected by
/// `validate()` before any AWS call, exiting 2.
#[tokio::test]
async fn rename_different_buckets_exits_2() {
    let output = run_s7cmd(&[
        "rename",
        "--source-profile",
        PROFILE_NAME,
        "s3://bucket-a--apne1-az4--x-s3/source.txt",
        "s3://bucket-b--apne1-az4--x-s3/destination.txt",
    ]);

    assert_eq!(
        output.status.code(),
        Some(EXIT_CODE_CLAP_ARG_ERROR),
        "rename across different buckets must exit 2; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
