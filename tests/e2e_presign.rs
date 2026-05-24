//! Process-level e2e tests for the presign subcommand.
//!
//! presign signs URLs locally — no S3 API call is made — so creation
//! succeeds even for non-existent buckets/keys. These tests cover both the
//! local signing path and the resulting URL's behavior when fetched
//! against real AWS.

#![cfg(e2e_test)]

mod common;

use std::io::Read;

use common::{REGION, TestHelper, generate_bucket_name, run, s7cmd_cmd};

const PROFILE: &str = "s7cmd-e2e-test";

/// HTTP GET via `ureq` and return `(http_status, body_bytes)`. Used to
/// fetch a presigned URL and assert both the status code (success vs.
/// expired/AccessDenied) and the body content. Going through ureq (with
/// rustls + bundled webpki-roots) keeps these tests portable across
/// Linux / macOS / Windows e2e runs without depending on a system
/// `curl` binary.
fn http_get(url: &str) -> (u16, Vec<u8>) {
    match ureq::get(url).call() {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let mut body = Vec::new();
            resp.into_body()
                .into_reader()
                .read_to_end(&mut body)
                .expect("read presigned-URL body");
            (status, body)
        }
        // ureq v3 returns Err(StatusCode(code)) for any 4xx/5xx; we still
        // want the status code to distinguish `expired` (403) from `missing` (404).
        Err(ureq::Error::StatusCode(code)) => (code, Vec::new()),
        Err(e) => panic!("HTTP transport error fetching presigned URL: {e}"),
    }
}

/// presign an object that exists, then GET the URL and assert the body.
/// Also asserts the URL has the SigV4 query shape and the bucket/key
/// embedded — pinning the structure so a future SDK change that produced
/// a malformed URL would fail loudly, not just at the HTTP-fetch step.
#[tokio::test]
async fn presign_get_returns_url_that_downloads_object_body() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let key = "presign-test.txt";
    let body = b"hello presigned world".to_vec();
    helper.put_object(&bucket, key, body.clone()).await;

    let object_arg = format!("s3://{bucket}/{key}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "presign",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &object_arg,
    ]));

    // Capture the URL before the cascade-delete so the URL itself
    // remains stable regardless of teardown order.
    let url = stdout.trim().to_string();
    let success = code == Some(0);

    // Fetch first, then teardown — fetching after delete would race
    // S3's eventually-consistent path.
    let (http_status, fetched_body) = if success {
        http_get(&url)
    } else {
        (0, Vec::new())
    };

    helper.delete_bucket_with_cascade(&bucket).await;

    assert!(success, "presign should succeed; stderr: {stderr}");
    assert_eq!(code, Some(0));
    assert!(
        url.starts_with("https://"),
        "presigned URL must be HTTPS; got: {url}"
    );
    assert!(
        url.contains(&bucket),
        "presigned URL must contain the bucket name; got: {url}"
    );
    assert!(
        url.contains(key),
        "presigned URL must contain the key; got: {url}"
    );
    assert!(
        url.contains("X-Amz-Signature="),
        "presigned URL must carry a SigV4 signature; got: {url}"
    );
    assert!(
        url.contains("X-Amz-Expires=3600"),
        "default --expires-in must be 3600; got: {url}"
    );
    assert_eq!(http_status, 200, "GET on presigned URL must return 200");
    assert_eq!(
        fetched_body, body,
        "presigned URL must return the original object body"
    );
}

/// `--expires-in N` must be reflected in the `X-Amz-Expires` query param.
#[tokio::test]
async fn presign_expires_in_propagates_to_url_query() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let key = "expires-test.txt";
    helper.put_object(&bucket, key, b"x".to_vec()).await;

    let object_arg = format!("s3://{bucket}/{key}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "presign",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        "--expires-in",
        "120",
        &object_arg,
    ]));

    helper.delete_bucket_with_cascade(&bucket).await;

    assert_eq!(code, Some(0), "presign should succeed; stderr: {stderr}");
    let url = stdout.trim().to_string();
    assert!(
        url.contains("X-Amz-Expires=120"),
        "presigned URL must carry the requested --expires-in; got: {url}"
    );
}

/// Presigning is a local-only operation — no S3 call is made — so a URL
/// for a non-existent key should still be generated successfully. Fetching
/// it should then return 404 NoSuchKey.
#[tokio::test]
async fn presign_for_missing_key_succeeds_then_get_404s() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let object_arg = format!("s3://{bucket}/nonexistent-key");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "presign",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &object_arg,
    ]));

    let url = stdout.trim().to_string();
    let success = code == Some(0);

    let http_status = if success { http_get(&url).0 } else { 0 };

    helper.delete_bucket_with_cascade(&bucket).await;

    assert!(
        success,
        "presign for missing key must succeed (signing is local); stderr: {stderr}"
    );
    assert_eq!(
        http_status, 404,
        "fetching presigned URL for a missing key must return 404"
    );
}

/// presign on a non-existent bucket must also succeed at signing time —
/// the SDK never calls S3 for presign — and 404 at fetch time.
#[tokio::test]
async fn presign_for_missing_bucket_succeeds_then_get_404s() {
    let nonexistent = format!("s7cmd-nonexistent-{}", uuid::Uuid::new_v4());
    let object_arg = format!("s3://{nonexistent}/key");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "presign",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &object_arg,
    ]));

    assert_eq!(
        code,
        Some(0),
        "presign for missing bucket must succeed (signing is local); stderr: {stderr}"
    );
    let url = stdout.trim().to_string();
    let (http_status, _) = http_get(&url);
    assert_eq!(
        http_status, 404,
        "fetching presigned URL for missing bucket must return 404"
    );
}

/// A 1-second URL must reject (HTTP 403 AccessDenied) once the validity
/// window has elapsed. Confirms the X-Amz-Expires value is enforced
/// server-side, not just included as cosmetic metadata.
#[tokio::test]
async fn presign_short_expiry_url_rejected_after_window() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let key = "short-expiry.txt";
    helper.put_object(&bucket, key, b"transient".to_vec()).await;

    let object_arg = format!("s3://{bucket}/{key}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "presign",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        "--expires-in",
        "1",
        &object_arg,
    ]));
    let success = code == Some(0);
    let url = stdout.trim().to_string();

    // Sleep past the 1s window. Two seconds of slack covers clock skew
    // between the local machine and S3's signing-time check.
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let http_status = if success { http_get(&url).0 } else { 0 };

    helper.delete_bucket_with_cascade(&bucket).await;

    assert!(success, "presign should succeed; stderr: {stderr}");
    assert_eq!(
        http_status, 403,
        "expired presigned URL must return 403 AccessDenied; got: {http_status}"
    );
}

/// presign with the `Cmd::Presign` dispatch path — bucket-only (no key)
/// target. `bucket_key()` rejects this post-parse, so the command exits 1
/// (validation error) rather than producing a URL.
#[tokio::test]
async fn presign_bucket_only_target_exits_1() {
    let bucket = format!("s7cmd-nonexistent-{}", uuid::Uuid::new_v4());
    let object_arg = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "presign",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &object_arg,
    ]));
    assert_eq!(code, Some(1), "bucket-only path should exit 1 (validation)");
}
