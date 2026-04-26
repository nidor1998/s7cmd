# s7cmd — Unified S3 CLI design

**Date:** 2026-04-26
**Status:** Approved (brainstorming)
**Repo:** `nidor1998/s7cmd`

## 1. Goal

Build a single binary, `s7cmd`, that exposes the union of capabilities from two existing crates — `s3sync` (sync) and `s3util-rs` (cp, mv, rm, bucket lifecycle, tagging, policy, versioning, head) — under one subcommand-style CLI:

```
s7cmd sync          [args]
s7cmd cp            [args]
s7cmd mv            [args]
s7cmd rm            [args]
s7cmd create-bucket [args]
s7cmd head-object   [args]
…
```

## 2. Constraints

- **Do not modify `s3sync` or `s3util-rs`.** Both are consumed as published crates. No upstream PRs; no path-overrides in shipped builds.
- **No global options** in v1. Every flag stays per-subcommand, identical to what each lib's standalone binary accepts. (Pre-positioning à la `s7cmd --target-region us-east-1 sync ...` is explicitly out of scope.)
- **Single binary** distribution (`cargo install s7cmd`).
- Subcommand inventory in v1 = whatever `s3sync` + `s3util-rs` already offer; no new commands.
- Lua scripting (s3sync's `lua_support` feature) is supported via passthrough.

## 3. Architecture

### 3.1 Crate topology

Only `s7cmd` changes. `s3sync` and `s3util-rs` are untouched.

```
┌─────────────────────────────────────────────────┐
│ s7cmd  (this repo, binary crate)                │
│                                                 │
│   Cargo.toml depends on:                        │
│     s3sync   = "1.57"                           │
│     s3util-rs = "0.2"                           │
│                                                 │
│   Code:                                         │
│     CLI: top-level Subcommand enum that         │
│          flattens each lib's existing args      │
│     Runners: thin wrappers per subcommand       │
│       - sync:        s3sync::Pipeline (lib API) │
│       - cp/mv/rm:    s3util_rs::transfer +      │
│                      storage (lib API),         │
│                      orchestration written here │
│       - create/del/  s3util_rs::storage::s3::api│
│         head bucket: (lib API)                  │
│       - tagging/     s3util_rs::storage::s3::api│
│         policy/      (lib API)                  │
│         versioning                              │
└─────────────────────────────────────────────────┘
```

Standalone `s3sync` and `s3util` binaries continue to exist in their own crates — unchanged.

### 3.2 Why this works without lib modifications

- `s3sync` officially supports library use (`s3sync::Pipeline::new(config, token).run().await`).
- `s3util-rs` declares "no API stability" but exposes the necessary public types: `Config`, `storage::s3::api::*`, `transfer::*`, and per-subcommand args structs (`CpArgs`, `MvArgs`, `CreateBucketArgs`, …). The runners (`run_cp`, `run_mv`, …) live in `bin/s3util/cli/` and are *not* in the lib — s7cmd writes its own equivalents on top of the public lib types.

## 4. CLI structure

`src/cli.rs`:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser, Clone, Debug)]
#[command(
    name = "s7cmd",
    about = "Unified S3 CLI: s3sync + s3util",
    version,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Cmd,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Cmd {
    /// Synchronize files between local and S3 (or S3 to S3)
    Sync(Box<s3sync::CLIArgs>),

    // --- s3util-rs object operations ---
    /// Copy objects from/to S3
    Cp(s3util_rs::config::args::CpArgs),
    /// Move objects from/to S3 (copy then delete source)
    Mv(s3util_rs::config::args::MvArgs),
    /// Delete a single S3 object
    Rm(s3util_rs::config::args::RmArgs),

    // --- bucket lifecycle ---
    CreateBucket(s3util_rs::config::args::CreateBucketArgs),
    DeleteBucket(s3util_rs::config::args::DeleteBucketArgs),
    HeadBucket(s3util_rs::config::args::HeadBucketArgs),

    // --- object metadata ---
    HeadObject(s3util_rs::config::args::HeadObjectArgs),

    // --- tagging (object) ---
    GetObjectTagging(s3util_rs::config::args::GetObjectTaggingArgs),
    PutObjectTagging(s3util_rs::config::args::PutObjectTaggingArgs),
    DeleteObjectTagging(s3util_rs::config::args::DeleteObjectTaggingArgs),

    // --- tagging (bucket) ---
    GetBucketTagging(s3util_rs::config::args::GetBucketTaggingArgs),
    PutBucketTagging(s3util_rs::config::args::PutBucketTaggingArgs),
    DeleteBucketTagging(s3util_rs::config::args::DeleteBucketTaggingArgs),

    // --- bucket policy ---
    GetBucketPolicy(s3util_rs::config::args::GetBucketPolicyArgs),
    PutBucketPolicy(s3util_rs::config::args::PutBucketPolicyArgs),
    DeleteBucketPolicy(s3util_rs::config::args::DeleteBucketPolicyArgs),

    // --- bucket versioning ---
    GetBucketVersioning(s3util_rs::config::args::GetBucketVersioningArgs),
    PutBucketVersioning(s3util_rs::config::args::PutBucketVersioningArgs),
}
```

Notes:

- `Sync(Box<s3sync::CLIArgs>)` — `s3sync::CLIArgs` is large (~100 fields); boxing keeps the enum size bounded so the rest of the variants don't pay for it.
- All flag parsing is delegated to each lib's existing `#[derive(Parser)]` definitions. s7cmd contributes zero flag declarations.
- Per-subcommand help is whatever the lib already produces. `s7cmd cp --help` ≈ `s3util cp --help`. `s7cmd sync --help` ≈ `s3sync --help`.
- Subcommand naming uses Clap's default kebab-case derivation (`CreateBucket` → `create-bucket`, `GetObjectTagging` → `get-object-tagging`).
- Each lib's subcommand args struct is listed individually rather than embedding `s3util_rs::config::args::Commands` whole — embedding the enum would force `s7cmd s3util cp ...`, which is one nesting level deeper than wanted.

## 5. Runner architecture

Three categories by how much code they need.

### 5.1 Category A — sync (s3sync as a proper library)

Wraps the supported `s3sync::Pipeline` lib API. ~40 LOC.

```rust
// src/runners/sync.rs
pub async fn run(args: s3sync::CLIArgs) -> Result<ExitCode> {
    let mut config = s3sync::Config::try_from(args).map_err(|e| anyhow!("{}", e))?;
    if config.report_sync_status { config.dry_run = true; }

    if let Some(shell) = config.auto_complete_shell {
        clap_complete::generate(shell, &mut Cli::command(), "s7cmd", &mut io::stdout());
        return Ok(ExitCode::SUCCESS);
    }

    if let Some(tc) = &config.tracing_config { crate::tracing_init::init_tracing(tc); }

    let token = create_pipeline_cancellation_token();
    crate::ctrl_c::install(token.clone());

    let mut pipeline = Pipeline::new(config, token).await;
    pipeline.run().await;

    if pipeline.has_error() {
        let errs = pipeline.get_errors_and_consume().unwrap();
        eprintln!("{:?}", errs[0]);
        return Ok(ExitCode::from(EXIT_ERROR));
    }
    if pipeline.has_warning() { return Ok(ExitCode::from(EXIT_WARNING)); }
    Ok(ExitCode::SUCCESS)
}
```

### 5.2 Category B — bucket / object metadata (direct lib API calls)

`create-bucket`, `delete-bucket`, `head-bucket`, `head-object`, `*-bucket-policy`, `*-bucket-tagging`, `*-bucket-versioning`, `*-object-tagging` — 17 commands, each ~30–60 LOC, all direct calls into `s3util_rs::storage::s3::api::*`.

Example:

```rust
// src/runners/create_bucket.rs
pub async fn run(args: CreateBucketArgs) -> Result<ExitCode> {
    let bucket = args.bucket_name().map_err(|e| anyhow!("{}", e.trim_end()))?;
    let client_config = args.common.build_client_config();
    crate::tracing_init::from_common(&args.common);
    let client = client_config.create_client().await;

    api::create_bucket(&client, &bucket).await?;
    info!(bucket = %bucket, "Bucket created.");

    if let Some(raw) = args.tagging.as_deref() {
        let tags = parse_tagging_to_tags(raw)?;
        let tagging = Tagging::builder().set_tag_set(Some(tags)).build()?;
        if let Err(e) = api::put_bucket_tagging(&client, &bucket, tagging).await {
            warn!(error = format!("{e:#}"), "bucket created but tagging failed");
            return Ok(ExitCode::from(EXIT_WARNING));
        }
    }
    Ok(ExitCode::SUCCESS)
}
```

These are mostly mechanical ports of the equivalent files in `s3util-rs/src/bin/s3util/cli/` — they call the same lib functions, with the same exit-status semantics.

### 5.3 Category C — cp / mv / rm (transfer pipeline)

These share an orchestration layer that today lives in `s3util-rs/src/bin/s3util/cli/mod.rs` (`run_copy_phase`, `build_rate_limiter`, `ExitStatus`, `detect_direction` plumbing). s7cmd writes its own equivalent on top of the lib's `transfer::*` modules:

```rust
// src/runners/transfer_pipeline.rs    (≈250 LOC; shared by cp/mv/rm)
pub struct TransferPhase { /* cancelled, transfer_result, has_warning */ }

pub async fn run_copy_phase(config: Config) -> Result<TransferPhase> {
    let token = create_pipeline_cancellation_token();
    crate::ctrl_c::install(token.clone());

    let direction = detect_direction(&config.source, &config.target);
    let factory = match direction {
        TransferDirection::LocalToS3 => /* build local source + s3 target */,
        TransferDirection::S3ToLocal => /* build s3 source + local target */,
        TransferDirection::S3ToS3    => /* build s3 source + s3 target */,
        TransferDirection::Stdio(_)  => /* stdio variant */,
    };
    let rate_limiter = build_rate_limiter(&config);
    let indicator = Indicator::new(&config);

    let outcome = factory.run(token.clone(), rate_limiter, indicator).await;
    Ok(TransferPhase::from(outcome, token))
}

// src/runners/cp.rs, mv.rs, rm.rs    (≈30 LOC each)
pub async fn run_cp(args: CpArgs) -> Result<ExitCode> {
    let config = Config::try_from(args).map_err(|e| anyhow!("{}", e))?;
    let phase = transfer_pipeline::run_copy_phase(config).await?;
    /* same exit-code mapping as s3util's cp.rs */
}
```

### 5.4 Estimated runner LOC

| Category | Commands | Per-command | Total |
|---|---|---|---|
| A (sync) | 1 | ~40 | ~40 |
| B (api wrappers) | 17 | ~40 | ~680 |
| C (cp/mv/rm + shared) | 3 + shared | ~30 each + ~250 shared | ~340 |
| Shared infra (`ExitStatus`, ctrl-c, indicator scaffold, tracing init, tagging parser, exit-code constants) | — | — | ~200 |
| **Total** | | | **~1260 LOC** |

## 6. Project layout

```
s7cmd/
├── Cargo.toml
├── README.md
├── CHANGELOG.md
├── LICENSE
├── build.rs                       # shadow-rs (optional `version` feature)
├── src/
│   ├── main.rs                    # ~50 LOC: parse Cli, dispatch, set exit code
│   ├── cli.rs                     # Cli + Cmd enum (Section 4)
│   ├── exit.rs                    # ExitCode constants & ExitStatus → ExitCode
│   ├── tracing_init.rs            # init_tracing(&TracingConfig)
│   ├── ctrl_c.rs                  # install_ctrl_c(token)
│   └── runners/
│       ├── mod.rs                 # pub use re-exports
│       ├── sync.rs                # Category A
│       ├── transfer_pipeline.rs   # Category C shared
│       ├── cp.rs                  # Category C
│       ├── mv.rs                  # Category C
│       ├── rm.rs                  # Category C
│       ├── create_bucket.rs       # Category B
│       ├── delete_bucket.rs       # Category B
│       ├── head_bucket.rs         # Category B
│       ├── head_object.rs         # Category B
│       ├── tagging_common.rs      # parse_tagging_to_tags() etc.
│       ├── object_tagging.rs      # get/put/delete object tagging
│       ├── bucket_tagging.rs      # get/put/delete bucket tagging
│       ├── bucket_policy.rs       # get/put/delete bucket policy
│       └── bucket_versioning.rs   # get/put bucket versioning
└── tests/
    ├── cli_help.rs                # smoke: --help output for each subcommand
    ├── cli_dispatch.rs            # parse-only tests for each Cmd variant
    └── e2e/                       # opt-in (gated by S7CMD_E2E=1)
        ├── sync_e2e.rs
        ├── transfer_e2e.rs
        └── bucket_ops_e2e.rs
```

### 6.1 Cargo.toml sketch

```toml
[package]
name = "s7cmd"
version = "0.1.0"
edition = "2024"
license = "Apache-2.0"
description = "Unified S3 CLI bundling s3sync and s3util-rs"

[dependencies]
# s3sync's `lua_support` is in its default features, so Lua flags
# (--preprocess-callback-lua-script, --filter-callback-lua-script, etc.)
# automatically appear under `s7cmd sync --help`. Use default-features = false
# to opt out (would also disable shadow-rs `version`; re-enable explicitly).
s3sync     = "1.57"
s3util-rs  = "0.2"

clap            = { version = "4.6", features = ["derive", "env", "cargo", "string"] }
clap_complete   = "4.6"
tokio           = { version = "1", features = ["full"] }
anyhow          = "1"
tracing         = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json", "local-time"] }

# aws-sdk-s3 / aws-config / aws-types are pulled in transitively; pin to the
# same minor versions as s3sync and s3util-rs so Cargo unifies them in the
# dep tree (see risk #3 in §8).

[features]
default = []
version = ["dep:shadow-rs"]

[build-dependencies]
shadow-rs = { version = "2", optional = true }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

Key call-outs:

- **Pin to a *minor* version of each lib** (e.g. `s3sync = "1.57"`, not `s3sync = "1"`). We depend on internal-ish details of `s3util-rs` (storage/transfer modules), and minor bumps must be a deliberate update rather than a silent `cargo update`.
- **No `[lib]` section** — pure binary crate.
- **No workspace** — s7cmd, s3sync, s3util-rs each live in their own repo. Local development can use `[patch.crates-io]` to point at sibling clones temporarily.

## 7. Testing

| Layer | Scope | Cost | What it catches |
|---|---|---|---|
| Compile-time | `cargo build` | seconds | API drift in either lib; arg-struct refactors; new fields |
| `tests/cli_help.rs` | `s7cmd --help`, `s7cmd <subcmd> --help` exit 0 and contain known headings; assert `s7cmd sync --help` includes `--filter-callback-lua-script` (Lua passthrough smoke) | ms | Subcommand wiring; flatten works; flag conflicts; Lua feature reaches the user |
| `tests/cli_dispatch.rs` | `Cli::try_parse_from(&[...])` per variant; assert correct `Cmd::*` matched with expected fields | ms | Subcommand routing; positional/flag parsing |
| `tests/e2e/` (gated by `S7CMD_E2E=1` + `AWS_*` env) | Real S3 / MinIO against a test bucket | seconds–minutes | Behavioral parity with `s3sync`/`s3util` standalone |

E2E tests are *behavioral parity* tests against the standalone binaries: for the same input and config, `s7cmd cp` should leave the bucket in the same state as `s3util cp`. Two test fixtures: a small file set and a multipart-threshold-crossing file. Anything beyond that is duplicating each lib's own test suite.

## 8. Known risks

1. **`s3util-rs`'s "no API stability" disclaimer.** s7cmd uses `Config`, `storage::s3::api::*`, `transfer::*`, and per-subcommand args structs. Mitigation: pin to `s3util-rs = "0.2"`; add a CI job that bumps to the latest s3util-rs and runs the test suite. If still passing, ratchet the pin; if broken, file an issue or patch s7cmd.
2. **Behavioral parity drift.** `s3util cp` vs `s7cmd cp` could diverge on output, exit codes, or ctrl-c handling. Mitigation: runners are deliberate ports of `bin/s3util/cli/`, copying the same `ExitStatus` semantics and indicator usage. E2E parity tests catch divergence.
3. **Dep-tree duplication.** s3sync, s3util-rs, and s7cmd transitively pull in `aws-sdk-s3`, `aws-config`, `tokio`, etc. Diverging *minor* versions cause Cargo to compile two copies, bloating the binary and risking type-incompatibility at API boundaries. Mitigation: keep s7cmd's pinned minors aligned with both libs at release time; document the alignment in a `MAINTAINERS.md` note.
4. **Features-flag mismatch.** If either lib gates types s7cmd uses behind a feature flag, that feature must be enabled on our dep declaration. Today: nothing of ours is gated. Re-check at implementation time.
5. **Lua passthrough means `mlua` is in the dep tree by default.** s3sync vendors Lua 5.4. Adds ~500 KB to release binary and a C build step. To opt out, depend on s3sync with `default-features = false` (must re-enable s3sync's `version` feature explicitly if wanted).

## 9. Phasing (input to the implementation plan)

- **Phase 1** — scaffold + sync subcommand only (Category A). Validates the architecture end-to-end with one lib.
- **Phase 2** — cp / mv / rm + shared transfer pipeline (Category C). Validates the harder pattern.
- **Phase 3** — bucket / object metadata commands (Category B). Mostly mechanical, can land in batches.

## 10. Out of scope for v1

- Global options (`s7cmd --target-region us-east-1 sync ...`) — every flag is per-subcommand.
- Unified `--profile` (vs `--source-profile` / `--target-profile`).
- New subcommands not in either lib (`ls`, `du`, `cat`).
- Behavioral changes to either lib (e.g. shared progress bar, unified output format).
- Workspace / monorepo refactor; cross-crate `[patch.crates-io]` for releases.
