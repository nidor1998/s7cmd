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
│     s3sync    = "1.57"                          │
│     s3util-rs = "0.2"                           │
│                                                 │
│   Code:                                         │
│     src/cli.rs          : Cli + Cmd enum        │
│     src/main.rs         : per-subcommand match  │
│                           arms vendored from    │
│                           each lib's main.rs    │
│     src/sync_bin/       : vendored from         │
│                           s3sync/src/bin/s3sync │
│                           (cli::run, tracing,   │
│                            ctrl_c, indicator,   │
│                            ui_config)           │
│     src/util_bin/       : vendored from         │
│                           s3util-rs/src/bin/    │
│                           s3util  (~25 cli/*    │
│                           files, tracing_init)  │
│                                                 │
│   The vendored code calls into each lib's       │
│   public API: s3sync::Pipeline,                 │
│   s3util_rs::{Config, storage::*, transfer::*}. │
└─────────────────────────────────────────────────┘
```

Standalone `s3sync` and `s3util` binaries continue to exist in their own crates — unchanged.

### 3.2 Why this works without lib modifications

- `s3sync` officially supports library use (`s3sync::Pipeline::new(config, token).run().await`).
- `s3util-rs` declares "no API stability" but exposes the necessary public types: `Config`, `storage::s3::api::*`, `transfer::*`, and per-subcommand args structs (`CpArgs`, `MvArgs`, `CreateBucketArgs`, …).
- The runners (`run_cp`, `run_mv`, …), the ctrl-c handler, the tracing init, and the per-subcommand exit-code mapping all live in each lib's `src/bin/` tree, *not* in the lib. **s7cmd vendors that bin code verbatim** — translated to s7cmd's module paths and with the program name swapped (`"s3sync"` / `"s3util"` → `"s7cmd"`) — so behavior is identical to what each standalone binary does. We do not redesign these orchestration concerns; we copy them.

### 3.3 Behavior preserved verbatim per command

For each subcommand, s7cmd reproduces the standalone binary's behavior:

| Concern | Source | Approach |
|---|---|---|
| Ctrl-C handling | `s3sync/src/bin/s3sync/cli/ctrl_c_handler/mod.rs` (sync) / `s3util-rs/src/bin/s3util/cli/ctrl_c_handler.rs` (all others) | Vendor verbatim (the two files are byte-identical apart from one import path). |
| Tracing init | `s3sync/src/bin/s3sync/tracing/mod.rs` (sync) / `s3util-rs/src/bin/s3util/tracing_init/mod.rs` (all others) | Vendor verbatim — note s3sync writes to stdout, s3util writes to stderr; we keep both. |
| Per-subcommand exit code | `s3util-rs/src/bin/s3util/main.rs` match arms / `s3sync/src/bin/s3sync/cli/mod.rs::run()` | Vendor each match arm verbatim into the dispatcher in s7cmd's `main.rs`; same `EXIT_CODE_*` constants from each lib's `cli/mod.rs`. |
| `run_xxx` orchestration | `s3sync/src/bin/s3sync/cli/mod.rs::run()` (sync) / `s3util-rs/src/bin/s3util/cli/<cmd>.rs` (all others) | Vendor verbatim into s7cmd, calling into each lib's public types (`Pipeline`, `storage::s3::api`, `transfer::*`, `Config`). |
| Indicator / progress UI | `s3sync/src/bin/s3sync/cli/indicator.rs` + `ui_config.rs` (sync) / `s3util-rs/src/bin/s3util/cli/indicator.rs` + `ui_config.rs` (cp/mv/rm) | Vendor both, separately — sync uses s3sync's, cp/mv/rm use s3util's. |
| Auto-complete shell | both bins' main.rs | Same short-circuit pattern; only swap `Cli::command()` and `"s7cmd"` as the program name. |

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

**Guiding rule: vendor the bin code verbatim from each lib.** s7cmd does not redesign orchestration; it transcribes it. Two parallel module trees mirror each lib's `src/bin/` layout.

### 5.1 Sync — port of `s3sync/src/bin/s3sync/`

`src/sync_bin/cli/mod.rs::run(config)` is the vendored copy of s3sync's `cli::run`, including:

- callback registration (`UserDefinedEventCallback`, `UserDefinedFilterCallback`, `UserDefinedPreprocessCallback` — needed for `test_user_defined_callback` parity)
- `ctrl_c_handler::spawn_ctrl_c_handler(token)` from the vendored `cli/ctrl_c_handler/mod.rs`
- `indicator::show_indicator(...)` and `ui_config::is_progress_indicator_needed(...)` / `is_show_result_needed(...)` from vendored `cli/indicator.rs` + `cli/ui_config.rs`
- `Pipeline::new(config, token).await`, `pipeline.run().await`, then the `has_error` / `has_warning` branches with the same `EXIT_CODE_*` constants (`SUCCESS=0`, `ERROR=1`, `INVALID_ARGS=2`, `WARNING=3`)
- `show_sync_report_summary(...)` helper — verbatim

The dispatch arm in `src/main.rs` for `Cmd::Sync` mirrors s3sync's `main()` exactly:

```rust
Cmd::Sync(args) => {
    // ports s3sync/src/bin/s3sync/main.rs verbatim
    let mut config = s3sync::Config::try_from(*args)
        .unwrap_or_else(|msg| {
            clap::Error::raw(clap::error::ErrorKind::ValueValidation, msg).exit()
        });
    if config.report_sync_status { config.dry_run = true; }

    if let Some(shell) = config.auto_complete_shell {
        clap_complete::generate(shell, &mut Cli::command(), "s7cmd",
            &mut std::io::stdout());
        return Ok(());                         // (only program-name swap)
    }
    if let Some(tc) = &config.tracing_config {
        sync_bin::tracing::init_tracing(tc);  // vendored from s3sync
    }
    sync_bin::cli::run(config).await           // anyhow::Result<()>
}
```

### 5.2 s3util commands — port of `s3util-rs/src/bin/s3util/`

`src/util_bin/cli/<cmd>.rs` is the vendored `cli/<cmd>.rs` from s3util-rs (e.g. `cp.rs`, `create_bucket.rs`, `head_object.rs`, ..., 20 files). They call back into `s3util_rs::storage::*`, `s3util_rs::transfer::*`, and the s3util-rs lib's public functions. The shared infrastructure (`cli/mod.rs` with `ExitStatus`, `run_copy_phase`, `build_rate_limiter`; `cli/indicator.rs`; `cli/ui_config.rs`; `cli/ctrl_c_handler.rs`; `cli/tagging.rs`) is also vendored.

The dispatch arm for each `Cmd::<UtilCmd>` in `src/main.rs` is a verbatim port of the corresponding match arm in `s3util-rs/src/bin/s3util/main.rs`. Two patterns appear, depending on the runner's return type:

```rust
// Pattern A — runners that return Result<ExitStatus>
//   (cp, mv, head_object, head_bucket, get_object_tagging,
//    get_bucket_tagging, get_bucket_versioning,
//    get_bucket_policy, create_bucket)
Cmd::Cp(args) => {
    if let Some(shell) = args.auto_complete_shell() {
        clap_complete::generate(shell, &mut Cli::command(), "s7cmd",
            &mut std::io::stdout());
        return Ok(());
    }
    let config = match s3util_rs::Config::try_from(args) {
        Ok(c) => c,
        Err(msg) => clap::Error::raw(
            clap::error::ErrorKind::ValueValidation, msg).exit(),
    };
    start_tracing_if_necessary(&config);   // vendored helper
    trace_config_summary(&config);         // vendored helper
    let exit_code = match util_bin::cli::run_cp(config).await {
        Ok(status) => status.code(),       // s3util_rs::cli::ExitStatus::code
        Err(e) => {
            tracing::error!(error = format!("{e:#}"));
            util_bin::cli::EXIT_CODE_ERROR
        }
    };
    std::process::exit(exit_code);
}

// Pattern B — runners that return Result<()>
//   (rm, delete_bucket, put_object_tagging, delete_object_tagging,
//    put_bucket_tagging, delete_bucket_tagging,
//    put_bucket_policy, delete_bucket_policy,
//    put_bucket_versioning)
Cmd::Rm(args) => {
    if let Some(shell) = args.auto_complete_shell() { /* ...same... */ }
    let tc = args.common.build_tracing_config();
    if let Some(tc) = &tc { util_bin::tracing_init::init_tracing(tc); }
    let client_config = args.common.build_client_config();
    let exit_code = match util_bin::cli::run_rm(args, client_config).await {
        Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
        Err(e) => {
            tracing::error!(error = format!("{e:#}"));
            util_bin::cli::EXIT_CODE_ERROR
        }
    };
    std::process::exit(exit_code);
}
```

The only changes from the upstream code are: program-name string (`"s3util"` → `"s7cmd"`), and the `Cli` type referenced in `clap_complete::generate` (s7cmd's `Cli`, not s3util's). Everything else — including `EXIT_CODE_*` constants, `ExitStatus::code()` mapping, `start_tracing_if_necessary` / `trace_config_summary` helpers — is copied unchanged into `src/main.rs` (or `src/util_bin/`).

### 5.3 Estimated vendored LOC

Counted by `wc -l` on the source files:

| Source | Files | LOC |
|---|---|---|
| `s3sync/src/bin/s3sync/cli/` (mod, ctrl_c_handler, indicator, ui_config) | 4 | ~520 |
| `s3sync/src/bin/s3sync/tracing/` | 1 | ~140 |
| `s3sync/src/bin/s3sync/main.rs` (match arm only) | (partial) | ~30 |
| `s3util-rs/src/bin/s3util/cli/` (all 25 files) | 25 | ~3055 |
| `s3util-rs/src/bin/s3util/tracing_init/` | 1 | ~210 |
| `s3util-rs/src/bin/s3util/main.rs` (all 20 match arms + 2 helpers) | (partial) | ~400 |
| s7cmd `src/cli.rs` + `src/main.rs` integration | new | ~150 |
| **Total** | | **~4500 LOC** |

Higher than the previous "~1260 written from scratch" estimate, because vendoring includes test code, long help strings, and the per-subcommand main.rs arms in full. The trade-off is what the user asked for: zero risk of behavioral drift from the libs, no novel orchestration logic to reason about.

## 6. Project layout

Two parallel `*_bin/` trees mirror each lib's `src/bin/` structure. Each file in `sync_bin/` and `util_bin/` corresponds 1:1 to a file in the upstream lib's `src/bin/`, vendored verbatim (with the program-name and module-path adjustments from §5).

```
s7cmd/
├── Cargo.toml
├── README.md
├── CHANGELOG.md
├── LICENSE
├── build.rs                       # shadow-rs (optional `version` feature)
├── docs/superpowers/specs/        # this design + future specs
├── src/
│   ├── main.rs                    # parse Cli, dispatch; per-subcommand
│   │                              # match arms vendored from each lib's main.rs
│   ├── cli.rs                     # s7cmd's Cli + Cmd enum (§4)
│   │
│   ├── sync_bin/                  # mirrors s3sync/src/bin/s3sync/
│   │   ├── mod.rs
│   │   ├── tracing.rs             # vendored from s3sync bin tracing/mod.rs
│   │   └── cli/
│   │       ├── mod.rs             # vendored: run(config), EXIT_CODE_*, helpers
│   │       ├── ctrl_c_handler.rs  # vendored verbatim
│   │       ├── indicator.rs       # vendored verbatim
│   │       └── ui_config.rs       # vendored verbatim
│   │
│   └── util_bin/                  # mirrors s3util-rs/src/bin/s3util/
│       ├── mod.rs
│       ├── tracing_init.rs        # vendored from s3util-rs bin
│       └── cli/
│           ├── mod.rs             # vendored: ExitStatus, run_copy_phase,
│           │                      # build_rate_limiter, EXIT_CODE_*
│           ├── ctrl_c_handler.rs  # vendored verbatim
│           ├── indicator.rs       # vendored verbatim
│           ├── ui_config.rs       # vendored verbatim
│           ├── tagging.rs         # vendored: parse_tagging_to_tags etc.
│           ├── cp.rs              # vendored
│           ├── mv.rs              # vendored
│           ├── rm.rs              # vendored
│           ├── create_bucket.rs   # vendored
│           ├── delete_bucket.rs   # vendored
│           ├── head_bucket.rs     # vendored
│           ├── head_object.rs     # vendored
│           ├── get_object_tagging.rs       # vendored
│           ├── put_object_tagging.rs       # vendored
│           ├── delete_object_tagging.rs    # vendored
│           ├── get_bucket_tagging.rs       # vendored
│           ├── put_bucket_tagging.rs       # vendored
│           ├── delete_bucket_tagging.rs    # vendored
│           ├── get_bucket_policy.rs        # vendored
│           ├── put_bucket_policy.rs        # vendored
│           ├── delete_bucket_policy.rs     # vendored
│           ├── get_bucket_versioning.rs    # vendored
│           └── put_bucket_versioning.rs    # vendored
└── tests/
    ├── cli_help.rs                # smoke: --help output for each subcommand
    ├── cli_dispatch.rs            # parse-only tests for each Cmd variant
    └── e2e/                       # opt-in (gated by S7CMD_E2E=1)
        ├── sync_e2e.rs
        ├── transfer_e2e.rs
        └── bucket_ops_e2e.rs
```

**Vendoring contract:** every file under `sync_bin/` and `util_bin/` carries a header comment naming the source file and the upstream version it was copied from, e.g.:

```rust
// Vendored from s3util-rs v0.2.0
//   src/bin/s3util/cli/cp.rs
// Adjustments: program name "s3util" → "s7cmd"; uses crate-local ExitStatus
// from util_bin::cli::mod.
```

This makes upstream re-syncs auditable.

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

Because the bin code is vendored verbatim, the existing test suites of each lib already cover the orchestration logic. s7cmd's own tests focus on the integration seam — that the dispatcher routes correctly and that the lib's behavior reaches the user via the s7cmd binary.

| Layer | Scope | Cost | What it catches |
|---|---|---|---|
| Compile-time | `cargo build` | seconds | API drift in either lib; arg-struct refactors; new pub fields needed by vendored bin code |
| `tests/cli_help.rs` | `s7cmd --help`, `s7cmd <subcmd> --help` exit 0 and contain known headings; `s7cmd sync --help` includes `--filter-callback-lua-script` (Lua passthrough smoke) | ms | Subcommand wiring; flatten works; flag conflicts; Lua feature reaches the user |
| `tests/cli_dispatch.rs` | `Cli::try_parse_from(&[...])` per variant; assert correct `Cmd::*` matched with expected fields | ms | Subcommand routing; positional/flag parsing |
| Vendored unit tests | `cargo test` runs the test modules vendored alongside each `cli/*.rs` | seconds | Detect when an upstream change to a vendored file would break a vendored test |
| `tests/e2e/` (gated by `S7CMD_E2E=1` + `AWS_*` env) | Real S3 / MinIO against a test bucket | seconds–minutes | End-to-end smoke: each subcommand produces the same observable outcome as the standalone tool |

E2E coverage is intentionally narrow (one fixture per category — small object, multipart-threshold object, bucket-create+head+delete) since the deeper coverage already lives in each lib's own test suites. Two fixtures per category at most.

## 8. Known risks

1. **Vendored bin code drifts from upstream.** The vendoring strategy (§3.2, §6) freezes the bin code at the pinned lib version. When s3sync or s3util-rs releases a new version with bug fixes / new flags / new subcommands, those don't reach s7cmd until someone manually re-runs the vendoring. Mitigation: every vendored file has a header naming its source path and upstream version; a maintenance script (`scripts/vendor-sync.sh`) diffs each vendored file against the upstream pin and emits a delta report. Run on every lib version bump.
2. **`s3util-rs`'s "no API stability" disclaimer.** s7cmd uses `Config`, `storage::s3::api::*`, `transfer::*`, and per-subcommand args structs. Mitigation: pin to `s3util-rs = "0.2"`; add a CI job that bumps to the latest s3util-rs and runs the test suite. If still passing, ratchet the pin; if broken, file an issue or patch the vendored code.
3. **Dep-tree duplication.** s3sync, s3util-rs, and s7cmd transitively pull in `aws-sdk-s3`, `aws-config`, `tokio`, etc. Diverging *minor* versions cause Cargo to compile two copies, bloating the binary and risking type-incompatibility at API boundaries. Mitigation: keep s7cmd's pinned minors aligned with both libs at release time; document the alignment in a `MAINTAINERS.md` note.
4. **Features-flag mismatch.** If either lib gates types s7cmd uses behind a feature flag, that feature must be enabled on our dep declaration. Today: only s3sync's `lua_support` (default-on, see risk #6). Re-check at implementation time.
5. **Tracing writers differ.** s3sync's tracing writes to stdout (default); s3util-rs's writes to stderr (`with_writer(std::io::stderr)`). After `s7cmd sync ...`, log lines land on stdout; after `s7cmd cp ...`, on stderr. This matches each lib's standalone behavior — explicit non-goal to unify in v1.
6. **Lua passthrough means `mlua` is in the dep tree by default.** s3sync vendors Lua 5.4. Adds ~500 KB to release binary and a C build step. To opt out, depend on s3sync with `default-features = false` (must re-enable s3sync's `version` feature explicitly if wanted).
7. **Global tracing subscriber.** `tracing_subscriber::*::init()` panics if called twice. Each subcommand only invokes one tracing init (sync uses s3sync's, others use s3util's), and each process invocation runs exactly one subcommand, so the constraint holds. Worth a comment in `main.rs` so a future "warm up tracing in `main`" refactor doesn't break it.
8. **License compatibility.** Both libs are Apache-2.0; vendored files must keep the same notice. s7cmd's `LICENSE` already matches. Add an `ATTRIBUTIONS.md` listing the vendored sources.

## 9. Phasing (input to the implementation plan)

- **Phase 1** — scaffold + sync subcommand. Vendor `sync_bin/` from s3sync; wire `Cmd::Sync` dispatch arm. Validates the vendoring contract end-to-end with one lib.
- **Phase 2** — cp / mv / rm. Vendor `util_bin/cli/{mod,ctrl_c_handler,indicator,ui_config,cp,mv,rm}.rs` plus `tracing_init/`; wire three dispatch arms. Validates the larger surface and the shared transfer infra.
- **Phase 3** — bucket / object metadata commands. Vendor remaining `util_bin/cli/*.rs` files (~17); wire dispatch arms. Mostly mechanical and parallelizable; can land in 2–3 batches.

Each phase ends with: passing `cargo build` + `cargo test` + manual smoke `s7cmd <new-subcmd> --help` against a real bucket (MinIO local container suffices).

## 10. Out of scope for v1

- Global options (`s7cmd --target-region us-east-1 sync ...`) — every flag is per-subcommand.
- Unified `--profile` (vs `--source-profile` / `--target-profile`).
- New subcommands not in either lib (`ls`, `du`, `cat`).
- Behavioral changes to either lib (e.g. shared progress bar, unified output format).
- Workspace / monorepo refactor; cross-crate `[patch.crates-io]` for releases.
