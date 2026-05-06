//! Process-level e2e tests for bucket-policy-status subcommand.

#![cfg(e2e_test)]

mod common;

use common::{
    REGION, TestHelper, create_temp_dir, create_test_file, generate_bucket_name, run, s7cmd_cmd,
};

const PROFILE: &str = "s7cmd-e2e-test";

/// Mirrors the e2e_bucket_policy.rs sample — non-public Deny policy that
/// AWS Block Public Access does not reject.
fn sample_policy(bucket: &str) -> String {
    format!(
        r#"{{"Version":"2012-10-17","Statement":[{{"Sid":"DenyInsecureTransport","Effect":"Deny","Principal":"*","Action":"s3:*","Resource":["arn:aws:s3:::{bucket}","arn:aws:s3:::{bucket}/*"],"Condition":{{"Bool":{{"aws:SecureTransport":"false"}}}}}}]}}"#
    )
}

/// Round-trip: put policy → get-bucket-policy-status → expect IsPublic=false.
#[tokio::test]
async fn get_policy_status_after_put_returns_is_public_false() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let bucket_arg = format!("s3://{bucket}");
    let policy_json = sample_policy(&bucket);
    let tmp_dir = create_temp_dir();
    let policy_file = create_test_file(&tmp_dir, "policy.json", policy_json.as_bytes());

    // Put policy
    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-policy",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
        policy_file.to_str().unwrap(),
    ]));
    assert_eq!(
        code,
        Some(0),
        "put-bucket-policy should succeed; stderr: {stderr}"
    );

    // Get policy status
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-bucket-policy-status",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&tmp_dir);

    assert_eq!(
        code,
        Some(0),
        "get-bucket-policy-status should succeed; stderr: {stderr}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("get-bucket-policy-status stdout must be JSON");
    assert_eq!(
        json.get("PolicyStatus")
            .and_then(|p| p.get("IsPublic"))
            .and_then(|v| v.as_bool()),
        Some(false),
        "expected IsPublic=false; got: {json}"
    );
}

/// get-bucket-policy-status on a bucket with no policy attached should
/// exit 4 (NoSuchBucketPolicy → NotFound).
#[tokio::test]
async fn get_policy_status_on_bucket_without_policy_exits_4() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let bucket_arg = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-policy-status",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));

    helper.delete_bucket_with_cascade(&bucket).await;

    assert_eq!(code, Some(4), "must exit 4 (NoSuchBucketPolicy)");
}

/// get-bucket-policy-status on a non-existent bucket should exit 4.
#[tokio::test]
async fn get_policy_status_on_missing_bucket_exits_4() {
    let bucket = generate_bucket_name();
    let bucket_arg = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-policy-status",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));

    assert_eq!(code, Some(4), "missing bucket must exit 4 (NoSuchBucket)");
}
