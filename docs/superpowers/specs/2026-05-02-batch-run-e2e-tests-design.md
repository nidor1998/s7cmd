# `batch-run` end-to-end tests — design

Status: draft (awaiting human review)
Date: 2026-05-02
Author: AI-generated (Claude Code), human review pending

## 1. Goal

Add a process-level, AWS-backed test suite for `batch-run` so every
public use case is exercised against real S3 — not just the parser and
dispatch layer that today's `tests/batch_run.rs` covers. The new suite
runs only when the maintainer opts in via `RUSTFLAGS="--cfg e2e_test"`
and authenticates through the existing `s7cmd-e2e-test` AWS profile.

The goal is **functional correctness on a live service** for the
following: successful multi-line dispatch, file-vs-stdin script
sources, streaming mode, parallel dispatch, fail-fast, `--continue-on-
error`, `--max-errors N`, `--continue-on-warning`, exit-code
aggregation, mixed-outcome summaries, `--json-tracing` summary shape,
`--no-summary` suppression, and SIGINT propagation through in-flight
subcommands.

## 2. Non-goals

- Re-testing parsing, rejection, line-cap, `--check-format`, or any
  other behavior that already has full coverage in
  `tests/batch_run.rs`. That file's tests do not hit AWS and should
  stay as they are; this design adds e2e coverage on top of it, it
  does not replace anything.
- Performance or throughput benchmarks. The e2e tests assert
  correctness only.
- S3-compatible storage (MinIO, R2, etc.). The suite targets Amazon
  S3 via the maintainer's `s7cmd-e2e-test` profile, in line with
  existing `e2e_*.rs` files.
- Fuzz coverage of malformed inputs. One representative malformed-
  input test is included for error-class diversity; broader coverage
  belongs at the unit-test layer.

## 3. File layout

A single new file: `tests/e2e_batch_run.rs`.

```rust
//! Process-level e2e tests for batch-run.
//!
//! Gated by cfg(e2e_test) — hits real AWS via the s7cmd-e2e-test profile.

#![cfg(e2e_test)]

mod common;

use common::{
    REGION, TestHelper, create_sized_file, create_temp_dir, create_test_file,
    generate_bucket_name, run, s7cmd_cmd,
};
```

The SIGINT test (#13) is gated additionally on `unix` to match
`tests/e2e_ctrl_c.rs`. Implementation note: use a local
`#[cfg(all(e2e_test, unix))]` attribute on that single test plus its
helper, rather than gating the whole file.

Helpers reused from `tests/common/mod.rs`:
- `s7cmd_cmd()` — `Command` pointing at the freshly-built binary, stdin
  closed by default.
- `run(&mut Command)` — execute and return `(exit_code, stdout, stderr)`.
- `generate_bucket_name()` — UUID-suffixed bucket name.
- `create_temp_dir()`, `create_test_file(...)`, `create_sized_file(...)`
  — local file fixtures under `./playground/`.
- `TestHelper::new().await` — SDK client built from the e2e profile.
- `helper.create_bucket(&bucket, REGION).await`,
  `helper.is_object_exist(&bucket, key, version).await`,
  `helper.delete_bucket_with_cascade(&bucket).await`,
  plus tagging accessors as needed.

`#[tokio::test]` is used throughout (the helper APIs are async). Every
dispatched line in every script bakes in
`--target-profile s7cmd-e2e-test --target-region <REGION>` —
`batch-run` does not propagate top-level flags down to dispatched
subcommands.

## 4. Test list (13 cases)

Each row gets one `#[tokio::test]` function in `tests/e2e_batch_run.rs`.

| # | Test name | Script shape (per-line target-profile/region elided) | Asserts |
|---|-----------|------------------------------------------------------|---------|
| 1 | `batch_run_e2e_mixed_workflow_succeeds` | `create-bucket B` → `put-bucket-tagging B "k=v"` → `cp local.txt s3://B/key` → `head-object s3://B/key` → `clean s3://B/` → `delete-bucket B` | exit 0; SDK confirms tagging set, object present after `cp`, object absent after `clean`, bucket gone after `delete-bucket`; stderr summary `6 succeeded, 0 failed, 0 warnings, 0 skipped`. |
| 2 | `batch_run_e2e_reads_from_file` | Same shape as #1, written to `script.txt` under a temp dir, passed positionally to `batch-run` | Same assertions as #1; confirms `<FILE>` path is exercised. |
| 3 | `batch_run_e2e_streaming_dispatch_succeeds` | `create-bucket B` then `delete-bucket B`, fed via stdin with `--streaming` | exit 0; bucket exists between the two lines (verifiable only after the run; we only assert end state — bucket absent — plus summary `2 succeeded`). |
| 4 | `batch_run_e2e_parallel_two_workers_succeeds` | Setup: `helper.create_bucket(B, REGION)`, then `helper.put_object(B, "k{N}", b"x")` for N=1..4. Script: 4× `put-object-tagging --tagging "team=data" s3://B/kN` lines, run with `--parallel 2`. | exit 0; `helper.get_object_tagging(B, "kN", None)` for each N returns the expected tag; summary `4 succeeded, 0 failed`. Completion order is not asserted. |
| 5 | `batch_run_e2e_default_fails_fast_on_aws_error` | `head-bucket` on non-existent name `MISSING` → `create-bucket REAL` (real, never-created name) | exit 1; `REAL` does **not** exist on S3 (the second line never dispatched); summary `0 succeeded, 1 failed, 0 warnings, 1 skipped`. |
| 6 | `batch_run_e2e_continue_on_error_runs_all_lines` | `head-bucket MISSING_A` → `head-bucket MISSING_B` → `create-bucket REAL` → `delete-bucket REAL`, with `--continue-on-error` | exit 1 (worst-of); summary `2 succeeded, 2 failed, 0 warnings, 0 skipped`; `REAL` does not exist on S3 at end of run. |
| 7 | `batch_run_e2e_max_errors_two_stops_after_second_failure` | `head-bucket MISSING_A` → `head-bucket MISSING_B` → `head-bucket MISSING_C` → `create-bucket REAL`, with `--max-errors 2` | exit 1; summary `0 succeeded, 2 failed, 0 warnings, 2 skipped`; `REAL` does **not** exist on S3 (third + fourth lines never dispatched). |
| 8 | `batch_run_e2e_continue_on_warning_past_not_found` | Setup: create bucket B. Script: `head-object s3://B/missing-key` → `cp local.txt s3://B/real-key` → `head-object s3://B/real-key`, with `--continue-on-warning` | exit 4 (worst-of among 4, 0, 0); summary `2 succeeded, 0 failed, 1 warnings, 0 skipped`; object `real-key` exists on S3 at end. |
| 9 | `batch_run_e2e_exit_code_is_worst_of` | Setup: create bucket B, upload `real-key`. Script: `head-object s3://B/real-key` (exit 0) → `head-object s3://B/missing-key` (exit 4) → `head-bucket MISSING` (exit 1), with `--continue-on-error --continue-on-warning` | exit 4 (numeric max of 0, 4, 1); summary `1 succeeded, 1 failed, 1 warnings, 0 skipped`. |
| 10 | `batch_run_e2e_malformed_policy_failure_propagates` | Setup: create bucket B. Script: `put-bucket-policy --policy '{not valid json' s3://B` → `delete-bucket B`, with `--continue-on-error` | exit 1; summary `1 succeeded, 1 failed, 0 warnings, 0 skipped`; bucket B is gone at end (the second line ran). Distinct from #5/#6/#7 because the AWS error is `MalformedPolicy` (400-class) rather than `NoSuchBucket` (404-class). |
| 11 | `batch_run_e2e_json_tracing_summary_object` | Setup: create bucket B. Script: `head-bucket B` → `head-bucket MISSING`, with `--json-tracing --continue-on-error` | exit 1; stderr contains a single-line JSON object whose fields parse as `{"summary":"batch-run","succeeded":1,"failed":1,"warnings":0,"skipped":0,"elapsed_seconds":<f64>}`. The test parses the JSON via `serde_json` (already a dev-dependency) and checks each numeric field. |
| 12 | `batch_run_e2e_no_summary_suppresses_summary_real_run` | Same setup as #11 but a 2-line all-success script (`head-bucket B` × 2) with `--no-summary` | exit 0; stderr does **not** contain `succeeded,` (matches the existing non-e2e test's negative assertion, but on a live run). |
| 13 | `batch_run_e2e_sigint_marks_in_flight_skipped` | Setup: create bucket B, create a 200 MB sized local file. Spawn `s7cmd batch-run -` with stdin pipe and write a 2-line script (`cp big.bin s3://B/key1` + `cp big.bin s3://B/key2`), wait `STARTUP_DELAY_MS = 1500`, deliver SIGINT, `child.wait()` capped at `WAIT_TIMEOUT_SECS = 30`. | exit code 130; stderr summary line includes `skipped` ≥ 1 (the not-yet-started or in-flight `cp`); bucket cleaned via `delete_bucket_with_cascade`. Unix-only (`#[cfg(all(e2e_test, unix))]`). |

### 4.1 Notes on selected tests

- **#1 / #2 — `clean` step.** `clean s3://B/` deletes all objects under
  the prefix (per the s3rm-rs README). For a one-object bucket this
  yields exit 0 and removes the object. We use it to round out the
  workflow with a bulk-delete step.
- **#9 — exit-code arithmetic.** Existing batch-run logic uses
  numeric `max` over per-line exit codes. With (0, 4, 1) the max is
  4, so the assertion is `exit_code == 4`, not 1. This is the test
  that pins the worst-of rule to its actual implementation.
- **#11 — JSON parsing.** `serde_json::from_str(line)` against the
  one stderr line that starts with `{"summary"`. No regex; structural
  parse so a future field addition only fails the test if a checked
  field is removed or renamed.
- **#13 — file size.** 200 MB is enough to still be in-flight after
  1.5 s on a typical maintainer connection. If a future maintainer
  finds the test flaky on faster links, the size can be bumped; this
  is consistent with `e2e_ctrl_c.rs`'s own sized-file approach.

## 5. Failure-injection cookbook

| Failure class | Mechanism | Used in |
|---|---|---|
| `NoSuchBucket` (404) | Use `generate_bucket_name()` and never call `helper.create_bucket(...)` for that name. | #5, #6, #7, #9 |
| `NoSuchKey` / NOT_FOUND (exit 4) | Real bucket, never-uploaded synthetic key (`does-not-exist.txt`). | #8, #9 |
| `MalformedPolicy` (400) | `put-bucket-policy --policy '{not valid json'`. | #10 |

Successful side effects are always confirmed via SDK calls
(`is_object_exist`, `get_object_tagging`, bucket existence checks),
not by parsing s7cmd stdout — keeps the assertions decoupled from
log/format changes.

## 6. SIGINT specifics (#13)

Adapted from `tests/e2e_ctrl_c.rs`:

- `STARTUP_DELAY_MS = 1500`, `WAIT_TIMEOUT_SECS = 30` — keep aligned
  with the existing file so a future tuning change can be applied
  uniformly.
- Spawn the binary directly (`Command::new(env!("CARGO_BIN_EXE_s7cmd"))`)
  rather than via cargo, so the test process — not cargo —
  receives our own signal handling. Stdout/stderr are piped so we
  can scan stderr for the summary line; the existing ctrl_c tests
  use `Stdio::null()` because they only check exit code, but we
  need stderr content here.
- Stdin is `Stdio::piped()` so we can write the script after spawn.
- After `kill(pid, SIGINT)`, `child.wait_with_output()` is wrapped in
  `tokio::time::timeout(Duration::from_secs(WAIT_TIMEOUT_SECS), ...)`
  via `tokio::task::spawn_blocking`, mirroring the existing helper's
  shape.

A small private helper in this file (not in `common/`) builds the
spawned-and-signaled child and returns
`(exit_code, stdout, stderr)`. Pulling it into `common/` would couple
two tests that don't otherwise share helpers; keep it local.

## 7. Cleanup discipline

For every test:

- All buckets created (whether by the test setup directly via
  `helper.create_bucket` or by the script under test via
  `create-bucket`) are torn down at the end with
  `helper.delete_bucket_with_cascade(&bucket).await`. If a test's
  script ends with `delete-bucket`, the cascade call is still
  invoked as a no-op-safe belt-and-suspenders cleanup.
- Local temp dirs are removed via
  `let _ = std::fs::remove_dir_all(&local_dir);`.
- Failure-injection tests that operate on never-created bucket names
  do nothing for those names — there's nothing to delete.
- Cleanup runs unconditionally at the end of the test body, after
  assertions. We do not use a `Drop` guard; existing tests don't,
  and a panic mid-test will leave a UUID-suffixed bucket that the
  maintainer can sweep manually (consistent with current practice).

## 8. Risks and mitigations

- **AWS spend.** 13 tests × ~1 bucket each = ~13 buckets per run.
  All are deleted via cascade. Object volume is small except for
  test #13's 200 MB file (a single PUT, then SIGINT mid-stream). In
  line with existing e2e files; no special concern.
- **SIGINT flakiness.** Sized-file approach used here is the same
  one the maintainer already accepted for the cp/mv ctrl_c tests.
  If the upload finishes before the signal lands, the test's
  `skipped >= 1` assertion will fail; that's the same
  failure mode as the existing tests and is acceptable.
- **Streaming-mode timing.** The `--streaming` test (#3) does a
  short two-line script. There's no risk of the reader racing the
  dispatch in a way that affects correctness; the test asserts the
  final state on S3, not order or timing.
- **`clean` semantics.** `clean s3://B/` on a single-object bucket
  is expected to exit 0 and remove the object. If the underlying
  s3rm-rs returns a different exit code (e.g. for an empty bucket
  variant), the assertion in #1 needs adjusting — flagged here so a
  reviewer can check.

## 9. Out-of-scope follow-ups

- Adding a Windows-friendly cancellation test for batch-run
  (currently #13 is Unix-only). The existing ctrl_c suite is also
  Unix-only; aligning with it is intentional.
- Running these tests in CI. They require the maintainer's AWS
  profile and are not run by `cargo test`. They remain
  maintainer-invoked.
- Negative tests for `batch-run --json-tracing` schema regressions
  beyond the field-presence check in #11. A formal JSON schema
  would belong with the rest of the json-tracing test stack, not
  here.

## 10. Acceptance criteria

- `cargo check --all-features --tests` passes (no e2e flag) — the
  new file compiles when gated out, no unused-import warnings.
- `RUSTFLAGS="--cfg e2e_test" cargo check --tests` passes — the
  e2e-gated file compiles with the cfg.
- All 13 tests pass when the maintainer runs them against AWS with
  the `s7cmd-e2e-test` profile.
- `cargo fmt` and `cargo clippy --all-features` are clean.
- No changes to `tests/batch_run.rs` or any other existing file
  except `tests/common/mod.rs` *only if* a missing helper turns up
  during implementation; any such addition is mentioned in the
  implementation plan, not invented in this design.
