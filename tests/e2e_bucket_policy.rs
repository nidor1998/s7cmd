//! Process-level e2e tests for bucket-policy subcommands.

#![cfg(e2e_test)]

mod common;

use common::{REGION, TestHelper, generate_bucket_name, run, s7cmd_cmd};

/// Minimal bucket policy that grants the test profile's principal a no-op
/// permission. We don't care what the policy *does* — only that put / get /
/// delete dispatch against it. The key inside is filled in at runtime
/// because the bucket name is unique per test.
fn sample_policy(bucket: &str) -> String {
    format!(
        r#"{{
            "Version": "2012-10-17",
            "Statement": [
                {{
                    "Sid": "AllowGetObject",
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:GetObject",
                    "Resource": "arn:aws:s3:::{bucket}/*"
                }}
            ]
        }}"#
    )
}

#[tokio::test]
async fn put_bucket_policy_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let policy = sample_policy(&bucket);
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-policy",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--policy",
        &policy,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "put-bucket-policy must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_bucket_policy_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper.put_bucket_policy(&bucket, &sample_policy(&bucket)).await;

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-bucket-policy",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "get-bucket-policy must exit 0; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("AllowGetObject"),
        "stdout must contain seeded policy SID; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn get_bucket_policy_dispatch_not_found() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let target = format!("s3://{bucket}");
    let (code, _stdout, _stderr) = run(s7cmd_cmd().args([
        "get-bucket-policy",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(4),
        "get-bucket-policy on bucket without policy must exit 4"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn delete_bucket_policy_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper.put_bucket_policy(&bucket, &sample_policy(&bucket)).await;

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "delete-bucket-policy",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "delete-bucket-policy must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}
