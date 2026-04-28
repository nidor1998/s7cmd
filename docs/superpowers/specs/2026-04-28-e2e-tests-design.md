# s7cmd — End-to-end tests design

**Date:** 2026-04-28
**Status:** Approved (brainstorming)
**Repo:** `nidor1998/s7cmd`

## 1. Goal

Add end-to-end (E2E) tests that launch the real `s7cmd` binary as a subprocess and verify, at the **process level**, that:

1. Every subcommand dispatches to the right handler and produces the right exit code on success and on each documented failure mode.
2. Tracing output lands on the documented stream (stdout for `sync`, stderr for everything else; flippable for `sync` via `--tracing-stderr`; JSON via `--json-tracing`).
3. Subcommands with cancellation handlers (`sync`, `ls`, `clean`, `cp`, `mv`) exit with code 130 on SIGINT and do not hang.

Detailed behavior of each transfer / list / delete pipeline is the responsibility of the upstream crates (`s3sync`, `s3util-rs`, `s3ls-rs`, `s3rm-rs`) and is **not** retested here.

## 2. Constraints

- **Real AWS, gated behind `cfg(e2e_test)`** — mirrors s3util-rs. The user prepares an AWS profile named `s7cmd-e2e-test`. Default `cargo test` stays offline; E2E runs with `RUSTFLAGS="--cfg e2e_test" cargo test`.
- **Process-level only** — every test invokes `env!("CARGO_BIN_EXE_s7cmd")` as a subprocess. Tests do not import the binary's modules.
- **One success + targeted failure(s) per subcommand** — depth, not breadth, is owned by each crate's own E2E suite.
- **Self-contained tests** — every test creates and tears down its own bucket (UUID-suffixed name) and does not depend on previous test state.
- **Unix-only Ctrl+C tests** — `#[cfg(unix)]` gate; no Windows SIGINT equivalent is in scope.

## 3. File layout

```
tests/
├── cli_dispatch.rs              (existing, keep)
├── cli_help.rs                  (existing, keep)
├── cli_arg_validation.rs        NEW — non-AWS, runs by default
├── common/
│   └── mod.rs                   NEW — TestHelper, profile, bucket naming
├── e2e_object_ops.rs            NEW — sync, ls, clean, cp, mv, rm
├── e2e_object_metadata.rs       NEW — head-object + 3 object-tagging
├── e2e_bucket_ops.rs            NEW — create-bucket, delete-bucket, head-bucket
├── e2e_bucket_tagging.rs        NEW — get/put/delete-bucket-tagging
├── e2e_bucket_policy.rs         NEW — get/put/delete-bucket-policy
├── e2e_bucket_versioning.rs     NEW — get/put-bucket-versioning
├── e2e_tracing.rs               NEW — cross-cutting: stdout vs stderr per command class
└── e2e_ctrl_c.rs                NEW — SIGINT exit-130 for sync/ls/clean/cp/mv
```

Every `e2e_*.rs` starts with `#![cfg(e2e_test)]` and `mod common;`. `cli_arg_validation.rs` is **not** gated and runs in default `cargo test`.

`Cargo.toml` updates:

- `[lints.rust] unexpected_cfgs = { level = "warn", check-cfg = ['cfg(e2e_test)'] }`
- New dev-deps: `uuid` (bucket-name suffix), `nix` (Unix signals), `tokio` already present.
  `aws-sdk-s3` is already a regular dep — reuse for SDK-driven setup/teardown.

## 4. `tests/common/mod.rs`

Trimmed `TestHelper` — only what process-level tests need (no SSE-C / checksum tables / programmatic transfer helpers).

### 4.1 Constants

- `PROFILE_NAME = "s7cmd-e2e-test"`
- `REGION = "ap-northeast-1"` — overridable via `S7CMD_E2E_REGION` env var (read at `TestHelper::new()` time).

### 4.2 API surface

```rust
pub struct TestHelper { client: aws_sdk_s3::Client }

impl TestHelper {
    // Construction
    pub async fn new() -> Self;
    pub fn generate_bucket_name() -> String;          // "s7cmd-e2e-{uuid}"
    pub fn create_temp_dir() -> PathBuf;              // ./playground/tmp_{uuid}/
    pub fn create_test_file(dir: &Path, name: &str, body: &[u8]) -> PathBuf;

    // Bucket lifecycle (SDK)
    pub async fn create_bucket(&self, bucket: &str, region: &str);
    pub async fn is_bucket_exist(&self, bucket: &str) -> bool;
    pub async fn delete_bucket_with_cascade(&self, bucket: &str);   // idempotent

    // Object lifecycle (SDK)
    pub async fn put_object(&self, bucket: &str, key: &str, body: Vec<u8>);
    pub async fn is_object_exist(&self, bucket: &str, key: &str, version_id: Option<String>) -> bool;
    pub async fn delete_object(&self, bucket: &str, key: &str, version_id: Option<String>);
    pub async fn delete_all_objects(&self, bucket: &str);
    pub async fn delete_all_object_versions(&self, bucket: &str);

    // Seeding helpers (only used to set state that a `get-*` / `delete-*` test reads)
    pub async fn put_object_tagging(&self, bucket: &str, key: &str, tags: &[(&str, &str)]);
    pub async fn put_bucket_tagging(&self, bucket: &str, tags: &[(&str, &str)]);
    pub async fn put_bucket_policy(&self, bucket: &str, policy_json: &str);
    pub async fn enable_bucket_versioning(&self, bucket: &str);

    // Multipart cleanup (Ctrl+C tests for cp/mv)
    pub async fn abort_all_multipart_uploads(&self, bucket: &str);
}

// Process helpers (free functions)
pub fn s7cmd_cmd() -> std::process::Command;          // pre-built, stdin null, stdout/stderr piped
pub fn run(cmd: &mut Command) -> (Option<i32>, String, String);  // (exit_code, stdout, stderr)
```

`delete_bucket_with_cascade` is best-effort + idempotent (no-op on already-deleted). A panic mid-test will skip teardown; we accept this cost — orphan buckets are easy to clean up manually with the same profile.

## 5. Exit-code coverage matrix

| Exit | Meaning | Where covered | How |
|---|---|---|---|
| **0** | Success | each `e2e_*.rs` | one happy-path per subcommand against real AWS |
| **1** | Generic error | `e2e_*.rs` (selected) | runtime failures: `delete-bucket` on bucket-not-empty, `put-bucket-policy` with malformed JSON, etc. |
| **2** | Clap arg error | `cli_arg_validation.rs` (no AWS gate) | per-subcommand: missing required args, invalid value, unknown flag. Plus top-level no-subcommand and unrecognized subcommand. |
| **3** | Warning | `e2e_object_ops.rs` | `sync` produces a warning path (e.g. `--check-etag` detecting an ETag drift). |
| **4** | Not found | `e2e_*.rs` (head/get groups) | `head-bucket` on missing bucket, `head-object` on missing key, `get-bucket-tagging`/`get-bucket-policy`/`get-bucket-versioning` on bucket without that config. |
| **130** | SIGINT | `e2e_ctrl_c.rs` | sync, ls, clean, cp, mv each spawn a long-running invocation, send SIGINT, assert 130. |

Every test asserts on `output.status.code()` against the specific expected value. The shared `run()` helper returns `(exit_code, stdout, stderr)` so failing assertions can include both streams.

Per subcommand (22 total), at minimum:

- 1 success test (exit 0) in its `e2e_*.rs`
- 1 arg-error test (exit 2) in `cli_arg_validation.rs`
- Plus exit 1 / 3 / 4 / 130 where the subcommand can produce them.

## 6. Tracing tests (`e2e_tracing.rs`)

Three pieces, all `cfg(e2e_test)` (need to actually run a command past `init_tracing`).

### 6.1 Default streams

One representative test per source crate (sync, ls, clean, util_bin) — verifying the dispatch wires up the right `init_tracing()` per group, not exhaustive per-subcommand:

| Command class | Expected stream |
|---|---|
| `sync` | **stdout** |
| `ls`, `clean`, `cp`, `mv`, `rm`, `head-*`, `get/put/delete-*` | **stderr** |

Each test runs the chosen command with `-vvv --disable-color-tracing`, asserts tracing markers (`config =`, level markers `TRACE`/`DEBUG`/`INFO`) on the expected stream, and asserts they do **not** leak to the other.

### 6.2 `--tracing-stderr` flips sync

One test that runs `sync --tracing-stderr -vvv …` and asserts tracing now appears on stderr, not stdout.

### 6.3 `--json-tracing` produces JSON

One test per command class: assert the chosen stream contains a JSON object — match substrings like `{"timestamp":` and `"level":"`.

### 6.4 Environment scrub

All tracing tests scrub `RUST_LOG`, `NO_COLOR`, `CLICOLOR`, `JSON_TRACING`, `TRACING_STDERR`, `AWS_SDK_TRACING` from the child env (these are `env =` clap args that user shell config could otherwise inject).

## 7. Ctrl+C tests (`e2e_ctrl_c.rs`)

`#![cfg(unix)]` for the entire file; on Windows the file becomes a no-op. Pattern mirrors s3util-rs's `cancel_s3_to_stdout_sigint_exits_130`:

1. Spawn the binary directly via `CARGO_BIN_EXE_s7cmd` (not `cargo run` — cargo intercepts SIGINT).
2. Sleep ~1.5s to let the binary start, authenticate, and enter its work loop.
3. Send SIGINT via `nix::sys::signal::kill(Pid::from_raw(child.id() as i32), Signal::SIGINT)`.
4. `child.wait()` (wrapped in `tokio::time::timeout(30s)` so a hang fails fast).
5. Assert `status.code() == Some(130)`.

| Test | Setup | Long-running invocation |
|---|---|---|
| `cancel_sync_sigint_exits_130` | seed bucket with a 30 MiB object | `s7cmd sync --source-profile … --rate-limit-bandwidth 2MiB s3://b/ ./local/` |
| `cancel_ls_sigint_exits_130` | seed bucket with many small objects | `s7cmd ls --recursive s3://b/` (slow listing) |
| `cancel_clean_sigint_exits_130` | seed bucket with many small objects | `s7cmd clean --force --recursive --rate-limit 2 s3://b/` |
| `cancel_cp_sigint_exits_130` | local 30 MiB file | `s7cmd cp --target-profile … --rate-limit-bandwidth 2MiB ./big.bin s3://b/key` |
| `cancel_mv_sigint_exits_130` | seed bucket with 30 MiB object | `s7cmd mv --source-profile … --target-profile … --rate-limit-bandwidth 2MiB s3://b1/k s3://b2/k` |

Tear-down: `delete_bucket_with_cascade` plus `abort_all_multipart_uploads` for cp/mv (clears any orphan MPUs SIGINT left behind).

If a subcommand's flag set lacks a usable throttle (verified via `--help` during implementation), fall back to s3util-rs's softer pattern: assert "process exits, not hangs" without a strict exit-code check, with a comment explaining why. Default plan assumes throttles are available.

## 8. Per-subcommand E2E test list

### 8.1 `e2e_object_ops.rs` — sync, ls, clean, cp, mv, rm

- `sync_dispatch_success` — `sync ./localdir s3://b/`; exit 0
- `sync_dispatch_warning` — `sync --check-etag` against object whose ETag differs; exit 3
- `ls_dispatch_success_buckets` — `ls` (no target) lists buckets; exit 0; stdout contains test bucket
- `ls_dispatch_success_objects` — `ls s3://b/` after seeding two keys; exit 0; stdout contains both
- `clean_dispatch_success` — `clean --force s3://b/` after seeding; exit 0
- `cp_dispatch_success_local_to_s3` — exit 0; SDK-verify object exists
- `cp_dispatch_success_s3_to_local` — exit 0; assert file content matches
- `mv_dispatch_success` — exit 0; source absent, target present
- `rm_dispatch_success` — exit 0; object absent after

### 8.2 `e2e_object_metadata.rs` — head-object, 3 object-tagging

- `head_object_dispatch_success` — exit 0; stdout shows metadata
- `head_object_dispatch_not_found` — exit 4
- `put_object_tagging_dispatch_success` — exit 0; SDK-verify
- `get_object_tagging_dispatch_success` — exit 0; stdout contains seeded tag
- `delete_object_tagging_dispatch_success` — exit 0; SDK-verify empty

### 8.3 `e2e_bucket_ops.rs` — create-bucket, delete-bucket, head-bucket

- `create_bucket_dispatch_success` — exit 0; SDK-verify
- `head_bucket_dispatch_success` — exit 0
- `head_bucket_dispatch_not_found` — exit 4
- `delete_bucket_dispatch_success` — exit 0
- `delete_bucket_dispatch_error_not_empty` — exit 1 (seed object first, then attempt delete)

### 8.4 `e2e_bucket_tagging.rs` — get/put/delete-bucket-tagging

- `put_bucket_tagging_dispatch_success` — exit 0; SDK-verify
- `get_bucket_tagging_dispatch_success` — exit 0; stdout contains seeded tag
- `get_bucket_tagging_dispatch_not_found` — exit 4 (bucket has no tagging)
- `delete_bucket_tagging_dispatch_success` — exit 0

### 8.5 `e2e_bucket_policy.rs` — get/put/delete-bucket-policy

- `put_bucket_policy_dispatch_success` — exit 0
- `get_bucket_policy_dispatch_success` — exit 0; stdout contains policy JSON
- `get_bucket_policy_dispatch_not_found` — exit 4 (bucket has no policy)
- `delete_bucket_policy_dispatch_success` — exit 0

### 8.6 `e2e_bucket_versioning.rs` — get/put-bucket-versioning

- `put_bucket_versioning_dispatch_success` — exit 0
- `get_bucket_versioning_dispatch_success` — exit 0; stdout contains `Enabled`

All 22 subcommands covered with at least one success test; `head-*` and `get-*` additionally get a NotFound (exit 4) test.

## 9. `cli_arg_validation.rs` (non-AWS, default-run)

One arg-error test per subcommand — invokes `s7cmd <sub>` with a malformed/missing/conflicting argument and asserts `exit == 2` plus a non-empty stderr. Examples:

- `sync_no_args_exits_2`
- `ls_invalid_target_exits_2` — `ls notavalidpath` → "target must be an S3 path"
- `cp_missing_target_exits_2` — `cp s3://b/k`
- `head_bucket_missing_target_exits_2`
- `put_bucket_tagging_missing_tags_exits_2`
- … (one per subcommand)

Plus existing top-level coverage:
- `unrecognized_subcommand_exits_2`
- `no_subcommand_prints_usage_exits_2`

## 10. Running the suite

```bash
# Default cargo test — fast, no AWS
cargo test

# Full E2E suite — requires the s7cmd-e2e-test profile
RUSTFLAGS="--cfg e2e_test" cargo test -- --test-threads=1
```

`--test-threads=1` is recommended for E2E to avoid parallel bucket-create churn against the same account. With ~30 e2e tests at a few seconds each, a serial run is on the order of a couple of minutes.

## 11. Out of scope

- Replicating the upstream crates' deep functional tests (etag/checksum verification, multipart edge cases, versioning semantics, tagging conflict semantics, etc.). Those live in s3sync / s3util-rs / s3ls-rs / s3rm-rs.
- Mock-server tests (no minio / moto). The user explicitly chose real AWS.
- Windows SIGINT testing.
- Performance / throughput assertions.
- Resource-tagging / cost-tracking on test buckets.

## 12. Open items handed to implementation

- Exact rate-limit / slowdown flag names per subcommand for Ctrl+C tests — verify against `--help` during implementation; fall back to soft "process exits, not hangs" assertions if a usable throttle is missing.
- The exact warning-producing flag combination for `sync` (Section 8.1) — verify via `s3sync` docs or `s7cmd sync --help` during implementation.
