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

use common::{
    REGION, TestHelper, create_temp_dir, create_test_file, generate_bucket_name, run, s7cmd_cmd,
};

/// Build a deterministic ASCII body of `len` bytes (cycling lowercase
/// letters). All bytes are in 0x61..=0x7A so `String::from_utf8_lossy`
/// is identity — `stdout.as_bytes()` round-trips losslessly.
fn ascii_body(len: usize) -> Vec<u8> {
    (0..len).map(|i| b'a' + (i % 26) as u8).collect()
}

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

// ---- cp S3 -> stdout: parallel ranged-GET path (s3util-rs 1.2.0) ----
//
// These three tests confirm that s7cmd's CLI surface routes correctly
// into s3util-rs 1.2.0's new s3-to-stdio dispatcher:
//
// - `transfer_parallel` for size >= multipart_threshold under -p > 1
// - `transfer_serial` for size <  multipart_threshold under -p > 1
// - `transfer_parallel` always for --auto-chunksize, with per-part chunk
//   plan derived from the source's actual parts list (composite ETag
//   verifies exactly).
//
// Body integrity is the assertion: a regression in dispatcher routing,
// chunk planning, ordering, or write-buffer handling would corrupt the
// stream. Bodies are deterministic ASCII so stdout round-trips through
// String::from_utf8_lossy without loss.

#[tokio::test]
async fn cp_s3_to_stdout_parallel_path_round_trips_large_body() {
    // Stage a multipart-uploaded source via `s7cmd cp` with chunksize
    // equal to the download default (8 MiB). 16 MiB body / 8 MiB
    // chunksize → 2 parts → composite source ETag of the form
    // "<md5>-2".
    //
    // Then download with -p4: dispatcher sees size ≥ default 8 MiB
    // threshold and routes to transfer_parallel. The parallel path
    // detects the source's multipart-shaped ETag and recomputes
    // composite over the same 8 MiB boundaries (default chunksize is
    // unchanged on the download side), so the recomputed ETag matches
    // the source ETag exactly. Exit 0 implies ETag verify passed;
    // byte equality covers ordering + assembly through the parallel
    // pipeline.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let body = ascii_body(16 * 1024 * 1024);
    let src_local = create_test_file(&local_dir, "parallel-src.bin", &body);
    let s3_target = format!("s3://{bucket}/parallel.bin");

    let (up_code, up_stdout, up_stderr) = run(s7cmd_cmd().args([
        "cp",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--multipart-threshold",
        "5MiB",
        "--multipart-chunksize",
        "8MiB",
        src_local.to_str().unwrap(),
        &s3_target,
    ]));
    assert_eq!(
        up_code,
        Some(0),
        "staging multipart upload must exit 0; stdout={up_stdout}\nstderr={up_stderr}"
    );

    let source = format!("s3://{bucket}/parallel.bin");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "cp",
        "--source-profile",
        "s7cmd-e2e-test",
        "--source-region",
        REGION,
        "--max-parallel-uploads",
        "4",
        &source,
        "-",
    ]));

    assert_eq!(
        code,
        Some(0),
        "cp s3->stdout parallel must exit 0 (composite ETag must verify); stderr={stderr}"
    );
    assert_eq!(
        stdout.len(),
        body.len(),
        "stdout length must match source size (parallel pipeline must not truncate or duplicate)"
    );
    assert_eq!(
        stdout.as_bytes(),
        body.as_slice(),
        "parallel ranged-GET must reassemble bytes in source order"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn cp_s3_to_stdout_auto_chunksize_with_multipart_source_round_trips() {
    // Stage a multipart-uploaded source via `s7cmd cp` (forced by
    // --multipart-threshold 5MiB --multipart-chunksize 5MiB on a 12 MiB
    // body → 3 parts: 5+5+2 MiB → composite ETag of the form "<md5>-3").
    // 5MiB is clap's enforced minimum for both flags.
    // Then download with --auto-chunksize -p4: the dispatcher must
    // always take transfer_parallel for auto_chunksize, fetch the parts
    // list (GetObjectAttributes / per-part HeadObject fallback), and
    // align ranged GETs with the source's actual part boundaries so the
    // recomputed composite ETag matches. Exit 0 implies ETag verify
    // passed; byte equality covers ordering + assembly.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let body = ascii_body(12 * 1024 * 1024);
    let src_local = create_test_file(&local_dir, "src.bin", &body);
    let s3_target = format!("s3://{bucket}/multipart-source.bin");

    let (up_code, up_stdout, up_stderr) = run(s7cmd_cmd().args([
        "cp",
        "--target-profile",
        "s7cmd-e2e-test",
        "--target-region",
        REGION,
        "--multipart-threshold",
        "5MiB",
        "--multipart-chunksize",
        "5MiB",
        src_local.to_str().unwrap(),
        &s3_target,
    ]));
    assert_eq!(
        up_code,
        Some(0),
        "staging multipart upload must exit 0; stdout={up_stdout}\nstderr={up_stderr}"
    );

    let source = format!("s3://{bucket}/multipart-source.bin");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "cp",
        "--source-profile",
        "s7cmd-e2e-test",
        "--source-region",
        REGION,
        "--auto-chunksize",
        "--max-parallel-uploads",
        "4",
        &source,
        "-",
    ]));
    assert_eq!(
        code,
        Some(0),
        "cp s3->stdout --auto-chunksize must exit 0 (composite ETag must verify); stderr={stderr}"
    );
    assert_eq!(
        stdout.len(),
        body.len(),
        "stdout length must match source size"
    );
    assert_eq!(
        stdout.as_bytes(),
        body.as_slice(),
        "auto-chunksize parallel pipeline must reassemble bytes in source order"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}

#[tokio::test]
async fn cp_s3_to_stdout_dispatcher_serial_below_threshold_round_trips() {
    // 1 MiB body well below the default 8 MiB multipart_threshold.
    // With -p4 the dispatcher HEADs first, sees size < threshold, and
    // forwards to transfer_serial (the old code path, but now reached
    // through the new dispatcher rather than directly). Confirms the
    // dispatcher routing decision doesn't break small-object downloads.
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    let body = ascii_body(1024 * 1024);
    helper.put_object(&bucket, "small.bin", body.clone()).await;

    let source = format!("s3://{bucket}/small.bin");
    let (code, stdout, stderr) = run(s7cmd_cmd().args([
        "cp",
        "--source-profile",
        "s7cmd-e2e-test",
        "--source-region",
        REGION,
        "--max-parallel-uploads",
        "4",
        &source,
        "-",
    ]));

    assert_eq!(
        code,
        Some(0),
        "cp s3->stdout dispatcher→serial must exit 0; stderr={stderr}"
    );
    assert_eq!(
        stdout.as_bytes(),
        body.as_slice(),
        "below-threshold body must round-trip through dispatcher→serial"
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
