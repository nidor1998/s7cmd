//! Process-level e2e tests for stdin/stdout transfer paths.
//!
//! Exercises the `StdioToS3` and `S3ToStdio` arms of `run_copy_phase` —
//! `cp - s3://bucket/key` (upload from stdin) and `cp s3://bucket/key -`
//! (download to stdout). These paths are not reachable from any other
//! test file because they require process-level stdin/stdout redirection.

#![cfg(e2e_test)]

mod common;

use std::io::Write;
use std::process::Stdio;

use common::{REGION, TestHelper, generate_bucket_name, run, s7cmd_cmd};

// ---- cp stdin -> S3 ----

#[tokio::test]
async fn cp_stdin_to_s3_uploads_object() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let body = b"data piped in from stdin";
    let target = format!("s3://{bucket}/from-stdin.txt");

    let mut child = s7cmd_cmd()
        .args([
            "cp",
            "--target-profile",
            "s7cmd-e2e-test",
            "--target-region",
            REGION,
            "-",
            &target,
        ])
        .stdin(Stdio::piped())
        .spawn()
        .expect("failed to spawn s7cmd cp");

    child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(body)
        .expect("write stdin");
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("wait for cp");
    assert_eq!(
        output.status.code(),
        Some(0),
        "cp stdin->s3 must exit 0; stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        helper
            .is_object_exist(&bucket, "from-stdin.txt", None)
            .await
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

// ---- cp S3 -> stdout ----

#[tokio::test]
async fn cp_s3_to_stdout_streams_object() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    let body = b"streamed back to stdout".to_vec();
    helper
        .put_object(&bucket, "to-stdout.txt", body.clone())
        .await;

    let source = format!("s3://{bucket}/to-stdout.txt");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "cp",
        "--source-profile",
        "s7cmd-e2e-test",
        "--source-region",
        REGION,
        &source,
        "-",
    ]));

    assert_eq!(code, Some(0), "cp s3->stdout must exit 0; stderr={stderr}");
    assert_eq!(
        stdout.as_bytes(),
        body.as_slice(),
        "stdout must contain streamed object body"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}

// ---- put-bucket-policy reading from stdin ----

#[tokio::test]
async fn put_bucket_policy_reads_from_stdin() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    // Same Deny+Principal:"*"+SecureTransport policy as e2e_bucket_policy.rs
    // — does not grant public access so it bypasses Block-Public-Policy.
    let policy = format!(
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
    );

    let target = format!("s3://{bucket}");
    let mut child = s7cmd_cmd()
        .args([
            "put-bucket-policy",
            "--target-profile",
            "s7cmd-e2e-test",
            "--target-region",
            REGION,
            &target,
            "-",
        ])
        .stdin(Stdio::piped())
        .spawn()
        .expect("failed to spawn s7cmd put-bucket-policy");

    child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(policy.as_bytes())
        .expect("write stdin");
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .expect("wait for put-bucket-policy");
    assert_eq!(
        output.status.code(),
        Some(0),
        "put-bucket-policy from stdin must exit 0; stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}
