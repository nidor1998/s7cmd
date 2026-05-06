//! Process-level e2e tests for bucket-replication subcommands.

#![cfg(e2e_test)]

mod common;

use common::{
    REGION, TestHelper, create_temp_dir, create_test_file, generate_bucket_name, run, s7cmd_cmd,
};
use std::io::Write;
use std::process::{Command, Stdio};

const PROFILE: &str = "s7cmd-e2e-test";

/// Build a minimal-but-valid replication-configuration JSON. S3 requires:
/// - Versioning enabled on both source and destination buckets
/// - An IAM role ARN that S3 can assume (a dummy ARN is sufficient for
///   put — S3 only validates the role at rule application time)
/// - At least one rule with Status, Destination, and (when filter is
///   present) DeleteMarkerReplication
///
/// To keep tests self-contained without IAM setup, we exercise only the
/// failure-path responses (NoSuchBucket / ReplicationConfigurationNotFound).
/// The happy round-trip would require IAM provisioning which is out of scope.
fn sample_replication(dest_bucket: &str) -> String {
    format!(
        r#"{{
            "Role": "arn:aws:iam::000000000000:role/s3-replication-test",
            "Rules": [
                {{
                    "ID": "rule-1",
                    "Priority": 1,
                    "Filter": {{}},
                    "Status": "Enabled",
                    "DeleteMarkerReplication": {{ "Status": "Disabled" }},
                    "Destination": {{ "Bucket": "arn:aws:s3:::{dest_bucket}" }}
                }}
            ]
        }}"#
    )
}

/// get-bucket-replication on a bucket without replication should exit 4
/// (ReplicationConfigurationNotFoundError → NotFound).
#[tokio::test]
async fn get_replication_on_bucket_without_replication_exits_4() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let bucket_arg = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-replication",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));

    helper.delete_bucket_with_cascade(&bucket).await;

    assert_eq!(
        code,
        Some(4),
        "must exit 4 (ReplicationConfigurationNotFoundError)"
    );
}

/// get-bucket-replication on a non-existent bucket should exit 4 (NoSuchBucket).
#[tokio::test]
async fn get_replication_on_missing_bucket_exits_4() {
    let bucket = generate_bucket_name();
    let bucket_arg = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-replication",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));

    assert_eq!(code, Some(4), "missing bucket must exit 4 (NoSuchBucket)");
}

/// delete-bucket-replication on a bucket without replication is idempotent
/// in S3 — succeeds silently. This confirms our wrapper exits 0.
#[tokio::test]
async fn delete_replication_on_bucket_without_replication_succeeds() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let bucket_arg = format!("s3://{bucket}");
    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "delete-bucket-replication",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));

    helper.delete_bucket_with_cascade(&bucket).await;

    assert_eq!(
        code,
        Some(0),
        "delete-bucket-replication on bucket without replication should succeed; stderr: {stderr}"
    );
}

/// delete-bucket-replication on a non-existent bucket should fail with exit 1.
#[tokio::test]
async fn delete_replication_on_missing_bucket_exits_1() {
    let bucket = generate_bucket_name();
    let bucket_arg = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "delete-bucket-replication",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
    ]));

    assert_eq!(code, Some(1));
}

/// put-bucket-replication on a non-existent bucket should fail with exit 1.
/// The body is sent and S3 rejects with NoSuchBucket.
#[tokio::test]
async fn put_replication_on_missing_bucket_exits_1() {
    let bucket = generate_bucket_name();
    let dest = generate_bucket_name();
    let bucket_arg = format!("s3://{bucket}");

    let tmp_dir = create_temp_dir();
    let cfg_file = create_test_file(
        &tmp_dir,
        "replication.json",
        sample_replication(&dest).as_bytes(),
    );

    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "put-bucket-replication",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
        cfg_file.to_str().unwrap(),
    ]));

    let _ = std::fs::remove_dir_all(&tmp_dir);

    assert_eq!(code, Some(1));
}

/// put-bucket-replication with malformed JSON via file exits 1 at parse stage.
#[tokio::test]
async fn put_replication_malformed_json_via_file_exits_1() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let bucket_arg = format!("s3://{bucket}");

    let tmp_dir = create_temp_dir();
    let cfg_file = create_test_file(&tmp_dir, "replication.json", b"not valid json {");

    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "put-bucket-replication",
        "--target-profile",
        PROFILE,
        "--target-region",
        REGION,
        &bucket_arg,
        cfg_file.to_str().unwrap(),
    ]));

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&tmp_dir);

    assert_eq!(code, Some(1));
}

/// put-bucket-replication with malformed JSON via stdin (`-`) exits 1.
#[tokio::test]
async fn put_replication_malformed_json_via_stdin_exits_1() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let bucket_arg = format!("s3://{bucket}");

    let mut child = Command::new(env!("CARGO_BIN_EXE_s7cmd"))
        .args([
            "put-bucket-replication",
            "--target-profile",
            PROFILE,
            "--target-region",
            REGION,
            &bucket_arg,
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn s7cmd");
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"not valid json {").ok();
    }
    let out = child.wait_with_output().expect("wait s7cmd");

    helper.delete_bucket_with_cascade(&bucket).await;

    assert_eq!(out.status.code(), Some(1));
}
