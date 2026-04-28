//! Process-level e2e tests for bucket-policy subcommands.

#![cfg(e2e_test)]

mod common;

use common::{
    REGION, TestHelper, create_temp_dir, create_test_file, generate_bucket_name, run, s7cmd_cmd,
};

/// Minimal bucket policy that does NOT grant public access — required because
/// many AWS accounts have S3 Block Public Access enabled, which rejects any
/// `Allow + Principal:"*"` policy with `BlockPublicPolicy`. A pure Deny
/// statement with `Principal:"*"` is exempt: it restricts access rather than
/// granting it. We don't care what the policy *does* — only that put / get /
/// delete dispatch against it. The bucket name is interpolated at runtime
/// because each test uses a unique bucket.
fn sample_policy(bucket: &str) -> String {
    format!(
        r#"{{
            "Version": "2012-10-17",
            "Statement": [
                {{
                    "Sid": "DenyInsecureConnections",
                    "Effect": "Deny",
                    "Principal": "*",
                    "Action": "s3:*",
                    "Resource": [
                        "arn:aws:s3:::{bucket}",
                        "arn:aws:s3:::{bucket}/*"
                    ],
                    "Condition": {{
                        "Bool": {{"aws:SecureTransport": "false"}}
                    }}
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

    // put-bucket-policy reads POLICY from a file path (or "-" for stdin).
    let local_dir = create_temp_dir();
    let policy_path =
        create_test_file(&local_dir, "policy.json", sample_policy(&bucket).as_bytes());

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "put-bucket-policy",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        &target,
        policy_path.to_str().unwrap(),
    ]));

    assert_eq!(
        code,
        Some(0),
        "put-bucket-policy must exit 0; stdout={stdout}\nstderr={stderr}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn get_bucket_policy_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper
        .put_bucket_policy(&bucket, &sample_policy(&bucket))
        .await;

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
        stdout.contains("DenyInsecureConnections"),
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
async fn get_bucket_policy_dispatch_bucket_not_found() {
    // Hits the `BucketNotFound` arm logged as `bucket … not found`.
    let bucket = generate_bucket_name();

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
        "get-bucket-policy on missing bucket must exit 4"
    );
}

#[tokio::test]
async fn get_bucket_policy_policy_only_outputs_inner_policy() {
    // Hits the `--policy-only` branch in get_bucket_policy that calls
    // `render_policy_only`. Output should be the inner JSON, not the
    // double-encoded `{"Policy": "<...>"}` wrapper.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper
        .put_bucket_policy(&bucket, &sample_policy(&bucket))
        .await;

    let target = format!("s3://{bucket}");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "get-bucket-policy",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--policy-only",
        &target,
    ]));

    assert_eq!(
        code,
        Some(0),
        "get-bucket-policy --policy-only must exit 0; stderr={stderr}"
    );
    assert!(
        stdout.contains("DenyInsecureConnections"),
        "stdout must contain seeded SID; stdout={stdout}"
    );
    // The wrapper field name must NOT appear when --policy-only is used.
    assert!(
        !stdout.contains("\"Policy\":"),
        "stdout must not contain the `Policy` wrapper field; stdout={stdout}"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

#[tokio::test]
async fn delete_bucket_policy_dispatch_success() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper
        .put_bucket_policy(&bucket, &sample_policy(&bucket))
        .await;

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
