# `batch-run` end-to-end tests — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 13 process-level, AWS-backed tests for `batch-run` covering every documented use case (mixed workflow, file/stdin sources, streaming, parallel, fail-fast, `--continue-on-error`, `--max-errors`, `--continue-on-warning`, exit-code aggregation, malformed-input error class, `--json-tracing` summary, `--no-summary` suppression, SIGINT propagation).

**Architecture:** A single new test file `tests/e2e_batch_run.rs` gated by `#![cfg(e2e_test)]`, following the established `e2e_*.rs` pattern (one `#[tokio::test]` per case, `TestHelper` for SDK-side verification, per-test bucket lifecycle). The SIGINT test additionally gates on `unix`. No production-code changes; no helper changes.

**Tech Stack:** Rust + tokio + assert_cmd + nix (SIGINT) + serde_json (JSON summary parse) + AWS SDK helpers in `tests/common/mod.rs`.

**Spec reference:** `docs/superpowers/specs/2026-05-02-batch-run-e2e-tests-design.md`.

---

## Operating rules for the executing agent

- **Do NOT run e2e tests.** Project policy in `CLAUDE.md`: "Never run e2e tests (`RUSTFLAGS="--cfg e2e_test" cargo test`)." Use `cargo check` and `cargo clippy` under the same `RUSTFLAGS` to verify compilation only. The user runs the e2e tests against live AWS themselves.
- **Do NOT run `git commit`.** The user's standing rule (recorded in user-memory) is to commit manually. Each task's final step is "stop and ask the user to commit", with a recommended commit message. Do not invoke `git commit`.
- **One file only.** Every task in this plan modifies `tests/e2e_batch_run.rs`. No other file is touched.
- **Run `cargo fmt` and `cargo clippy --all-features` before handing off each task** (project rule from `CLAUDE.md`). For e2e-gated code, the cfg is required: `RUSTFLAGS="--cfg e2e_test" cargo clippy --all-features --tests`.

---

## Task 1: Create the e2e test file skeleton

**Files:**
- Create: `tests/e2e_batch_run.rs`

- [ ] **Step 1: Create the file with imports and gate**

Create `tests/e2e_batch_run.rs` with the following exact content:

```rust
//! Process-level e2e tests for batch-run.
//!
//! Gated by cfg(e2e_test) — hits real AWS via the s7cmd-e2e-test profile.
//!
//! Per-test pattern:
//! 1. Create a fresh bucket via `helper.create_bucket(&bucket, REGION)`.
//! 2. Build a script that bakes `--target-profile s7cmd-e2e-test
//!    --target-region {REGION}` into every dispatched line (batch-run does
//!    not propagate top-level flags to subcommands).
//! 3. Run `s7cmd batch-run -` (or with a script file) via `assert_cmd`.
//! 4. Assert exit code, stderr summary, and SDK-visible side effects.
//! 5. Tear down the bucket with `helper.delete_bucket_with_cascade(&bucket)`.

#![cfg(e2e_test)]

mod common;

use assert_cmd::Command as AssertCommand;
use common::{
    REGION, TestHelper, create_temp_dir, create_test_file, generate_bucket_name, run, s7cmd_cmd,
};
use predicates::prelude::*;
```

- [ ] **Step 2: Verify compilation under the e2e cfg**

Run: `RUSTFLAGS="--cfg e2e_test" cargo check --tests`
Expected: compiles cleanly. `tests/e2e_batch_run.rs` produces no warnings (the unused-import warning will appear if `assert_cmd::Command as AssertCommand` is imported but never used; that import is only needed by tests that use stdin via `write_stdin` — Task 2 introduces the first such test, at which point the warning resolves. If the warning is emitted now, leave it — it disappears as soon as Task 2 adds the first call site).

If you prefer to defer the import warning: replace the imports with the comment `// imports added per task` and add them as each task introduces a new helper. The plan keeps them up-front because every task references them.

- [ ] **Step 3: Verify formatting and clippy**

Run: `cargo fmt`
Then: `RUSTFLAGS="--cfg e2e_test" cargo clippy --all-features --tests`
Expected: clippy emits no warnings about `tests/e2e_batch_run.rs` (other than the import-warning above, which Task 2 resolves).

- [ ] **Step 4: Ask the user to commit**

Stop and tell the user the skeleton is ready. Suggested commit message:

```
test(batch-run): scaffold e2e test file
```

Do **not** run `git commit` yourself. Wait for the user's go-ahead before continuing.

---

## Task 2: Test #1 — mixed workflow happy path

**Files:**
- Modify: `tests/e2e_batch_run.rs` (append a new `#[tokio::test]` function)

- [ ] **Step 1: Append the test**

Append the following function to the end of `tests/e2e_batch_run.rs`:

```rust
/// Test #1 — mixed-subcommand happy path.
///
/// Script: create-bucket → put-bucket-tagging → cp → head-object → clean →
/// delete-bucket. Verifies every step landed on S3 and the final bucket is
/// gone.
#[tokio::test]
async fn batch_run_e2e_mixed_workflow_succeeds() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    let local_dir = create_temp_dir();
    let local_file = create_test_file(&local_dir, "payload.txt", b"hello batch-run");

    let bucket_url = format!("s3://{bucket}/");
    let key_url = format!("s3://{bucket}/key");

    let script = format!(
        "create-bucket --target-profile s7cmd-e2e-test --target-region {REGION} {bucket_url}\n\
         put-bucket-tagging --target-profile s7cmd-e2e-test --target-region {REGION} \
         --tagging \"team=data\" {bucket_url}\n\
         cp --target-profile s7cmd-e2e-test --target-region {REGION} {local} {key_url}\n\
         head-object --target-profile s7cmd-e2e-test --target-region {REGION} {key_url}\n\
         clean --target-profile s7cmd-e2e-test --target-region {REGION} {bucket_url}\n\
         delete-bucket --target-profile s7cmd-e2e-test --target-region {REGION} {bucket_url}\n",
        local = local_file.to_str().unwrap(),
    );

    let assert = AssertCommand::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "-"])
        .write_stdin(script)
        .assert();

    assert
        .success()
        .stderr(predicate::str::contains(
            "6 succeeded, 0 failed, 0 warnings, 0 skipped",
        ));

    // SDK-side verification: bucket should be gone.
    assert!(
        !helper.is_bucket_exist(&bucket).await,
        "delete-bucket step should have removed the bucket"
    );

    // Belt-and-suspenders cleanup (idempotent on already-gone buckets).
    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}
```

- [ ] **Step 2: Verify compilation**

Run: `RUSTFLAGS="--cfg e2e_test" cargo check --tests`
Expected: compiles cleanly, no warnings.

- [ ] **Step 3: Verify fmt and clippy**

Run: `cargo fmt && RUSTFLAGS="--cfg e2e_test" cargo clippy --all-features --tests`
Expected: clippy clean.

- [ ] **Step 4: Ask the user to run the test and commit**

Tell the user: "Test #1 added. Please run `RUSTFLAGS=\"--cfg e2e_test\" cargo test --test e2e_batch_run batch_run_e2e_mixed_workflow_succeeds -- --nocapture` against your AWS profile, and let me know the result. If it passes, suggested commit message:"

```
test(batch-run): add e2e mixed-workflow happy path
```

Do **not** run `git commit`. Wait for the user.

---

## Task 3: Test #2 — read script from a file

**Files:**
- Modify: `tests/e2e_batch_run.rs`

- [ ] **Step 1: Append the test**

```rust
/// Test #2 — same script as #1, but written to a file and passed
/// positionally instead of piped via stdin. Confirms the `<FILE>` source
/// path is exercised against real AWS.
#[tokio::test]
async fn batch_run_e2e_reads_from_file() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    let local_dir = create_temp_dir();
    let local_file = create_test_file(&local_dir, "payload.txt", b"hello batch-run");

    let bucket_url = format!("s3://{bucket}/");
    let key_url = format!("s3://{bucket}/key");

    let script_body = format!(
        "create-bucket --target-profile s7cmd-e2e-test --target-region {REGION} {bucket_url}\n\
         put-bucket-tagging --target-profile s7cmd-e2e-test --target-region {REGION} \
         --tagging \"team=data\" {bucket_url}\n\
         cp --target-profile s7cmd-e2e-test --target-region {REGION} {local} {key_url}\n\
         head-object --target-profile s7cmd-e2e-test --target-region {REGION} {key_url}\n\
         clean --target-profile s7cmd-e2e-test --target-region {REGION} {bucket_url}\n\
         delete-bucket --target-profile s7cmd-e2e-test --target-region {REGION} {bucket_url}\n",
        local = local_file.to_str().unwrap(),
    );
    let script_path = create_test_file(&local_dir, "script.txt", script_body.as_bytes());

    let (code, _stdout, stderr) = run(s7cmd_cmd().args([
        "batch-run",
        script_path.to_str().unwrap(),
    ]));

    assert_eq!(code, Some(0), "expected exit 0; stderr={stderr}");
    assert!(
        stderr.contains("6 succeeded, 0 failed, 0 warnings, 0 skipped"),
        "summary mismatch; stderr={stderr}"
    );
    assert!(
        !helper.is_bucket_exist(&bucket).await,
        "delete-bucket step should have removed the bucket"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}
```

- [ ] **Step 2: cargo check + clippy**

Run: `RUSTFLAGS="--cfg e2e_test" cargo check --tests && cargo fmt && RUSTFLAGS="--cfg e2e_test" cargo clippy --all-features --tests`
Expected: clean.

- [ ] **Step 3: Ask the user to run + commit**

Suggested commit message:
```
test(batch-run): add e2e file-source variant of mixed workflow
```

---

## Task 4: Test #3 — streaming mode

**Files:**
- Modify: `tests/e2e_batch_run.rs`

- [ ] **Step 1: Append the test**

```rust
/// Test #3 — streaming mode. Two-line script (create + delete) executed
/// as it is read. Verifies the streaming reader dispatches real ops.
#[tokio::test]
async fn batch_run_e2e_streaming_dispatch_succeeds() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();

    let bucket_url = format!("s3://{bucket}/");
    let script = format!(
        "create-bucket --target-profile s7cmd-e2e-test --target-region {REGION} {bucket_url}\n\
         delete-bucket --target-profile s7cmd-e2e-test --target-region {REGION} {bucket_url}\n",
    );

    AssertCommand::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--streaming", "-"])
        .write_stdin(script)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "2 succeeded, 0 failed, 0 warnings, 0 skipped",
        ));

    assert!(
        !helper.is_bucket_exist(&bucket).await,
        "bucket should be gone after streaming run"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}
```

- [ ] **Step 2: cargo check + clippy** (same commands as Task 3 Step 2).

- [ ] **Step 3: Ask the user to run + commit**

Suggested commit message:
```
test(batch-run): add e2e streaming-mode dispatch
```

---

## Task 5: Test #4 — parallel two workers

**Files:**
- Modify: `tests/e2e_batch_run.rs`

- [ ] **Step 1: Append the test**

```rust
/// Test #4 — `--parallel 2` against four concurrent put-object-tagging
/// lines. Setup pre-creates the bucket and four objects via SDK helpers;
/// the script only does the tagging. Verifies all four tags landed.
#[tokio::test]
async fn batch_run_e2e_parallel_two_workers_succeeds() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    for n in 1..=4 {
        let key = format!("k{n}");
        helper.put_object(&bucket, &key, b"x".to_vec()).await;
    }

    let script = (1..=4)
        .map(|n| {
            format!(
                "put-object-tagging --target-profile s7cmd-e2e-test --target-region {REGION} \
                 --tagging \"team=data\" s3://{bucket}/k{n}"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    AssertCommand::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--parallel", "2", "-"])
        .write_stdin(script)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "4 succeeded, 0 failed, 0 warnings, 0 skipped",
        ));

    for n in 1..=4 {
        let key = format!("k{n}");
        let tagging = helper.get_object_tagging(&bucket, &key, None).await;
        let tags = tagging.tag_set();
        let found = tags
            .iter()
            .any(|t| t.key() == "team" && t.value() == "data");
        assert!(found, "expected tag team=data on key {key}; got {tags:?}");
    }

    helper.delete_bucket_with_cascade(&bucket).await;
}
```

- [ ] **Step 2: cargo check + clippy.**

- [ ] **Step 3: Ask user to run + commit.**

Suggested commit message:
```
test(batch-run): add e2e --parallel 2 dispatch
```

---

## Task 6: Test #5 — default fail-fast on AWS error

**Files:**
- Modify: `tests/e2e_batch_run.rs`

- [ ] **Step 1: Append the test**

```rust
/// Test #5 — default fail-fast: a head-bucket on a never-created bucket
/// fails (NoSuchBucket → exit 1), the second line never dispatches, the
/// real bucket is never created.
#[tokio::test]
async fn batch_run_e2e_default_fails_fast_on_aws_error() {
    let helper = TestHelper::new().await;
    let missing = generate_bucket_name();
    let real = generate_bucket_name();

    let script = format!(
        "head-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{missing}\n\
         create-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{real}\n",
    );

    let assert = AssertCommand::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "-"])
        .write_stdin(script)
        .assert();

    assert
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "0 succeeded, 1 failed, 0 warnings, 1 skipped",
        ));

    assert!(
        !helper.is_bucket_exist(&real).await,
        "second line should not have dispatched; real bucket must not exist"
    );

    // Cleanup is no-op for both names (neither was created).
    helper.delete_bucket_with_cascade(&real).await;
}
```

- [ ] **Step 2: cargo check + clippy.**

- [ ] **Step 3: Ask user to run + commit.**

Suggested commit message:
```
test(batch-run): add e2e default fail-fast on AWS error
```

---

## Task 7: Test #6 — `--continue-on-error` runs all lines

**Files:**
- Modify: `tests/e2e_batch_run.rs`

- [ ] **Step 1: Append the test**

```rust
/// Test #6 — `--continue-on-error` keeps running past failures. Two
/// missing-bucket head-bucket lines fail; create-bucket then delete-bucket
/// succeed. Worst-of exit code is 1.
#[tokio::test]
async fn batch_run_e2e_continue_on_error_runs_all_lines() {
    let helper = TestHelper::new().await;
    let missing_a = generate_bucket_name();
    let missing_b = generate_bucket_name();
    let real = generate_bucket_name();

    let script = format!(
        "head-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{missing_a}\n\
         head-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{missing_b}\n\
         create-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{real}\n\
         delete-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{real}\n",
    );

    let assert = AssertCommand::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--continue-on-error", "-"])
        .write_stdin(script)
        .assert();

    assert
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "2 succeeded, 2 failed, 0 warnings, 0 skipped",
        ));

    assert!(
        !helper.is_bucket_exist(&real).await,
        "real bucket should be gone (delete-bucket ran)"
    );

    helper.delete_bucket_with_cascade(&real).await;
}
```

- [ ] **Step 2: cargo check + clippy.**

- [ ] **Step 3: Ask user to run + commit.**

Suggested commit message:
```
test(batch-run): add e2e --continue-on-error runs all lines
```

---

## Task 8: Test #7 — `--max-errors 2` stops after second failure

**Files:**
- Modify: `tests/e2e_batch_run.rs`

- [ ] **Step 1: Append the test**

```rust
/// Test #7 — `--max-errors 2`: three missing-bucket lines + one
/// create-bucket. After the second failure, dispatch stops; lines 3 and 4
/// never run. The real bucket must not exist.
#[tokio::test]
async fn batch_run_e2e_max_errors_two_stops_after_second_failure() {
    let helper = TestHelper::new().await;
    let missing_a = generate_bucket_name();
    let missing_b = generate_bucket_name();
    let missing_c = generate_bucket_name();
    let real = generate_bucket_name();

    let script = format!(
        "head-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{missing_a}\n\
         head-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{missing_b}\n\
         head-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{missing_c}\n\
         create-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{real}\n",
    );

    let assert = AssertCommand::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--max-errors", "2", "-"])
        .write_stdin(script)
        .assert();

    assert
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "0 succeeded, 2 failed, 0 warnings, 2 skipped",
        ));

    assert!(
        !helper.is_bucket_exist(&real).await,
        "real bucket must not exist (lines 3 and 4 should have been skipped)"
    );

    helper.delete_bucket_with_cascade(&real).await;
}
```

- [ ] **Step 2: cargo check + clippy.**

- [ ] **Step 3: Ask user to run + commit.**

Suggested commit message:
```
test(batch-run): add e2e --max-errors stops after Nth failure
```

---

## Task 9: Test #8 — `--continue-on-warning` past NOT_FOUND

**Files:**
- Modify: `tests/e2e_batch_run.rs`

- [ ] **Step 1: Append the test**

```rust
/// Test #8 — `--continue-on-warning`: head-object on a missing key
/// returns exit 4 (NOT_FOUND warning). With the flag, the run continues
/// past it; subsequent cp + head-object on a real key both succeed.
/// Worst-of exit code is 4.
#[tokio::test]
async fn batch_run_e2e_continue_on_warning_past_not_found() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let local_file = create_test_file(&local_dir, "payload.txt", b"hello");

    let script = format!(
        "head-object --target-profile s7cmd-e2e-test --target-region {REGION} \
         s3://{bucket}/missing-key\n\
         cp --target-profile s7cmd-e2e-test --target-region {REGION} {local} \
         s3://{bucket}/real-key\n\
         head-object --target-profile s7cmd-e2e-test --target-region {REGION} \
         s3://{bucket}/real-key\n",
        local = local_file.to_str().unwrap(),
    );

    let assert = AssertCommand::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--continue-on-warning", "-"])
        .write_stdin(script)
        .assert();

    assert
        .failure()
        .code(4)
        .stderr(predicate::str::contains(
            "2 succeeded, 0 failed, 1 warnings, 0 skipped",
        ));

    assert!(
        helper.is_object_exist(&bucket, "real-key", None).await,
        "real-key should have been uploaded"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}
```

- [ ] **Step 2: cargo check + clippy.**

- [ ] **Step 3: Ask user to run + commit.**

Suggested commit message:
```
test(batch-run): add e2e --continue-on-warning past NOT_FOUND
```

---

## Task 10: Test #9 — exit code is worst-of (0, 4, 1) → 4

**Files:**
- Modify: `tests/e2e_batch_run.rs`

- [ ] **Step 1: Append the test**

```rust
/// Test #9 — pin the worst-of rule: exit codes (0, 4, 1) yield process
/// exit 4 (numeric max, not "1 wins because it's an error"). Requires
/// both `--continue-on-error` and `--continue-on-warning` so all three
/// lines dispatch.
#[tokio::test]
async fn batch_run_e2e_exit_code_is_worst_of() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;
    helper
        .put_object(&bucket, "real-key", b"hello".to_vec())
        .await;

    let missing_bucket = generate_bucket_name();

    let script = format!(
        "head-object --target-profile s7cmd-e2e-test --target-region {REGION} \
         s3://{bucket}/real-key\n\
         head-object --target-profile s7cmd-e2e-test --target-region {REGION} \
         s3://{bucket}/missing-key\n\
         head-bucket --target-profile s7cmd-e2e-test --target-region {REGION} \
         s3://{missing_bucket}\n",
    );

    let assert = AssertCommand::cargo_bin("s7cmd")
        .unwrap()
        .args([
            "batch-run",
            "--continue-on-error",
            "--continue-on-warning",
            "-",
        ])
        .write_stdin(script)
        .assert();

    assert
        .failure()
        .code(4)
        .stderr(predicate::str::contains(
            "1 succeeded, 1 failed, 1 warnings, 0 skipped",
        ));

    helper.delete_bucket_with_cascade(&bucket).await;
}
```

- [ ] **Step 2: cargo check + clippy.**

- [ ] **Step 3: Ask user to run + commit.**

Suggested commit message:
```
test(batch-run): add e2e worst-of exit-code aggregation
```

---

## Task 11: Test #10 — malformed-policy failure propagates

**Files:**
- Modify: `tests/e2e_batch_run.rs`

- [ ] **Step 1: Append the test**

```rust
/// Test #10 — non-NoSuchBucket failure (`MalformedPolicy`, 400-class)
/// propagates through dispatch the same way 404 does. Bucket cleanup
/// happens on the next line because of `--continue-on-error`.
#[tokio::test]
async fn batch_run_e2e_malformed_policy_failure_propagates() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let script = format!(
        "put-bucket-policy --target-profile s7cmd-e2e-test --target-region {REGION} \
         --policy '{{not valid json' s3://{bucket}\n\
         delete-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{bucket}\n",
    );

    let assert = AssertCommand::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--continue-on-error", "-"])
        .write_stdin(script)
        .assert();

    assert
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "1 succeeded, 1 failed, 0 warnings, 0 skipped",
        ));

    assert!(
        !helper.is_bucket_exist(&bucket).await,
        "delete-bucket should have removed the bucket on the second line"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}
```

- [ ] **Step 2: cargo check + clippy.**

- [ ] **Step 3: Ask user to run + commit.**

Suggested commit message:
```
test(batch-run): add e2e malformed-policy error class
```

---

## Task 12: Test #11 — `--json-tracing` summary object

**Files:**
- Modify: `tests/e2e_batch_run.rs`

- [ ] **Step 1: Append the test**

```rust
/// Test #11 — `--json-tracing` emits a single-line JSON summary object on
/// stderr. Parse it with serde_json (already a dev-dependency) and check
/// each numeric field against the expected counts.
///
/// Note: `assert_cmd::Command::output()` is used (not the `run()` helper
/// from `common`) because we need to both pipe stdin (`write_stdin`) and
/// capture stderr bytes; `run()` does not accept stdin.
#[tokio::test]
async fn batch_run_e2e_json_tracing_summary_object() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let missing = generate_bucket_name();

    let script = format!(
        "head-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{bucket}\n\
         head-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{missing}\n",
    );

    let output = AssertCommand::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--json-tracing", "--continue-on-error", "-"])
        .write_stdin(script)
        .output()
        .expect("spawn s7cmd");

    assert_eq!(output.status.code(), Some(1), "expected exit 1");

    let stderr_text = String::from_utf8_lossy(&output.stderr);
    let summary_line = stderr_text
        .lines()
        .find(|l| l.trim_start().starts_with(r#"{"summary""#))
        .unwrap_or_else(|| panic!("no JSON summary line in stderr; stderr={stderr_text}"));

    let parsed: serde_json::Value =
        serde_json::from_str(summary_line.trim()).expect("summary line must be JSON");

    assert_eq!(parsed["summary"], "batch-run");
    assert_eq!(parsed["succeeded"], 1);
    assert_eq!(parsed["failed"], 1);
    assert_eq!(parsed["warnings"], 0);
    assert_eq!(parsed["skipped"], 0);
    assert!(
        parsed["elapsed_seconds"].is_number(),
        "elapsed_seconds missing or not numeric"
    );

    helper.delete_bucket_with_cascade(&bucket).await;
}
```

- [ ] **Step 2: cargo check + clippy.**

Expected: clippy clean. If you get an `unused_must_use` warning on the deleted block, you missed the deletion — re-check Step 1.

- [ ] **Step 3: Ask user to run + commit.**

Suggested commit message:
```
test(batch-run): add e2e --json-tracing summary object
```

---

## Task 13: Test #12 — `--no-summary` suppresses summary on a real run

**Files:**
- Modify: `tests/e2e_batch_run.rs`

- [ ] **Step 1: Append the test**

```rust
/// Test #12 — `--no-summary` suppresses the summary line even on a real
/// AWS-backed run. Two head-bucket lines on a real bucket; both succeed.
#[tokio::test]
async fn batch_run_e2e_no_summary_suppresses_summary_real_run() {
    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let script = format!(
        "head-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{bucket}\n\
         head-bucket --target-profile s7cmd-e2e-test --target-region {REGION} s3://{bucket}\n",
    );

    AssertCommand::cargo_bin("s7cmd")
        .unwrap()
        .args(["batch-run", "--no-summary", "-"])
        .write_stdin(script)
        .assert()
        .success()
        .stderr(predicate::str::contains("succeeded,").not());

    helper.delete_bucket_with_cascade(&bucket).await;
}
```

- [ ] **Step 2: cargo check + clippy.**

- [ ] **Step 3: Ask user to run + commit.**

Suggested commit message:
```
test(batch-run): add e2e --no-summary suppression on real run
```

---

## Task 14: Test #13 — SIGINT marks in-flight commands skipped

**Files:**
- Modify: `tests/e2e_batch_run.rs`

This test is Unix-only (matches `tests/e2e_ctrl_c.rs`). It needs `nix` (already a dev-dependency) and `create_sized_file` (already in `common`). The `create_sized_file` import is local to the test body so it doesn't trigger an unused-import warning on non-unix builds.

- [ ] **Step 1: Append the SIGINT test**

```rust
#[cfg(all(e2e_test, unix))]
#[tokio::test]
async fn batch_run_e2e_sigint_marks_in_flight_skipped() {
    use common::create_sized_file;
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;
    use std::process::Stdio;
    use std::time::Duration;

    const STARTUP_DELAY_MS: u64 = 1500;
    const WAIT_TIMEOUT_SECS: u64 = 30;
    const PAYLOAD_BYTES: usize = 200 * 1024 * 1024; // 200 MB

    let helper = TestHelper::new().await;
    let bucket = generate_bucket_name();
    helper.create_bucket(&bucket, REGION).await;

    let local_dir = create_temp_dir();
    let payload = create_sized_file(&local_dir, "big.bin", PAYLOAD_BYTES);

    let script = format!(
        "cp --target-profile s7cmd-e2e-test --target-region {REGION} {p} s3://{bucket}/key1\n\
         cp --target-profile s7cmd-e2e-test --target-region {REGION} {p} s3://{bucket}/key2\n",
        p = payload.to_str().unwrap(),
    );

    let mut child = s7cmd_cmd()
        .args(["batch-run", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn s7cmd");

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("stdin piped");
        stdin
            .write_all(script.as_bytes())
            .expect("write script to child stdin");
    }
    // Closing stdin ensures batch-run sees EOF after the two lines.
    drop(child.stdin.take());

    tokio::time::sleep(Duration::from_millis(STARTUP_DELAY_MS)).await;
    let pid = Pid::from_raw(child.id() as i32);
    let _ = kill(pid, Signal::SIGINT);

    let wait_handle = tokio::task::spawn_blocking(move || child.wait_with_output());
    let output = match tokio::time::timeout(
        Duration::from_secs(WAIT_TIMEOUT_SECS),
        wait_handle,
    )
    .await
    {
        Ok(Ok(Ok(output))) => output,
        Ok(Ok(Err(e))) => panic!("child.wait_with_output failed: {e}"),
        Ok(Err(e)) => panic!("spawn_blocking join failed: {e}"),
        Err(_) => panic!("child did not exit within {WAIT_TIMEOUT_SECS}s after SIGINT"),
    };

    assert_eq!(
        output.status.code(),
        Some(130),
        "SIGINT must yield exit 130; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr_text = String::from_utf8_lossy(&output.stderr);
    // Summary line is `N succeeded, N failed, N warnings, N skipped, elapsed Ts`.
    // We assert at least one skipped (an in-flight or never-dispatched cp).
    let skipped_nonzero = stderr_text
        .lines()
        .filter(|l| l.contains("succeeded,") && l.contains("skipped"))
        .any(|l| {
            // crude: pull the integer immediately before `skipped`
            l.split(',')
                .find_map(|chunk| {
                    let trimmed = chunk.trim();
                    trimmed
                        .strip_suffix("skipped")
                        .map(str::trim)
                        .and_then(|n| n.parse::<u64>().ok())
                })
                .map(|n| n > 0)
                .unwrap_or(false)
        });
    assert!(
        skipped_nonzero,
        "expected skipped >= 1 in summary; stderr={stderr_text}"
    );

    // Cleanup: in-flight multipart uploads + bucket.
    helper.abort_all_multipart_uploads(&bucket).await;
    helper.delete_bucket_with_cascade(&bucket).await;
    let _ = std::fs::remove_dir_all(&local_dir);
}
```

- [ ] **Step 2: cargo check + clippy.**

Expected: clippy clean. The `#[cfg(all(e2e_test, unix))]` gate keeps the test out of non-unix builds, and `create_sized_file` is imported inside the test body so there is no unused-import warning on Windows builds either.

- [ ] **Step 3: Ask user to run + commit.**

Suggested commit message:
```
test(batch-run): add e2e SIGINT marks in-flight skipped
```

---

## Task 15: Final cross-check

**Files:** none (verification only).

- [ ] **Step 1: Run the full e2e cargo check + clippy gauntlet**

```bash
cargo fmt --check
RUSTFLAGS="--cfg e2e_test" cargo check --tests
RUSTFLAGS="--cfg e2e_test" cargo clippy --all-features --tests
```

Expected: all three commands pass with no warnings about `tests/e2e_batch_run.rs`.

- [ ] **Step 2: Run the non-e2e default test build**

```bash
cargo check --tests
cargo clippy --all-features --tests
```

Expected: passes. With the `#![cfg(e2e_test)]` gate in place, `tests/e2e_batch_run.rs` should compile to nothing on the default cfg.

- [ ] **Step 3: Tell the user the work is complete**

Inform the user: "All 13 e2e tests are added in `tests/e2e_batch_run.rs`. Compilation under both cfgs is clean. Please run the e2e suite (`RUSTFLAGS=\"--cfg e2e_test\" cargo test --test e2e_batch_run`) against your AWS profile when ready."

---

## Summary of files

| File | Status | Lines added (est.) |
|------|--------|--------------------|
| `tests/e2e_batch_run.rs` | created | ~600 |

No production code changes. No changes to `tests/common/mod.rs`, `tests/batch_run.rs`, or any other test file. No new dependencies.

## Spec-to-task coverage

| Spec test # | Plan task | Notes |
|---|---|---|
| 1 mixed workflow | Task 2 | |
| 2 file source | Task 3 | |
| 3 streaming | Task 4 | |
| 4 parallel 2 | Task 5 | |
| 5 default fail-fast | Task 6 | |
| 6 continue-on-error | Task 7 | |
| 7 max-errors 2 | Task 8 | |
| 8 continue-on-warning | Task 9 | |
| 9 worst-of exit | Task 10 | exit 4, not 1 — verifies numeric max |
| 10 malformed policy | Task 11 | |
| 11 json-tracing | Task 12 | serde_json field check |
| 12 no-summary | Task 13 | |
| 13 SIGINT | Task 14 | unix-only, sized 200 MB payload |
