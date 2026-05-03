# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Preview.** s7cmd is in an early/preview phase (0.x). The CLI surface, flag names, output formats, and exit codes may change between minor versions until 1.0.0.

## [0.3.0] - 2026-05-03

### Added

- **`cp --skip-existing`**: when the target object or file already
  exists, skip the copy instead of overwriting. Lets you resume
  partial bulk transfers (uploads, downloads, S3-to-S3) without
  re-sending objects that already landed at the destination.
  Combine with `--dry-run` to preview which objects would be
  skipped before running for real.
- **`create-bucket --if-not-exists`**: if the bucket already exists
  (and you own it), exit 0 without re-creating. Makes provisioning
  scripts idempotent — re-running the same
  `create-bucket --if-not-exists s3://my-bucket` is safe whether
  or not the bucket is already there. When combined with
  `--tagging`, the tagging step is also skipped on the
  existing-bucket path.

### Changed

- Bumped `s3util-rs` from 1.1 to 1.2 (vendored CLI sources synced
  to match) — this is what brings the two new flags above.
- Expanded the README's *Intended Audience and Issue Tracker Scope*
  section: questions about concurrency-induced performance or
  resource exhaustion, and questions that belong with AWS, with
  the operator of an S3-compatible storage service, or with the
  operating system vendor, are now explicitly out of scope for
  the issue tracker.
- Added a note in the `batch-run` section of the README clarifying
  that while batch-run avoids per-command process startup, it
  still constructs a fresh AWS client (credential, region, HTTP
  client setup) per command — so it is not intended for
  high-throughput parallel processing of large workloads.

### Underlying libraries

This release pins the following exact versions of the underlying
libraries:

```toml
s3sync     = "=1.58.6"
s3util-rs  = "=1.2.0"
s3rm-rs    = "=1.3.6"
s3ls-rs    = "=1.0.0"
```

## [0.2.0] - 2026-05-02

### Added

- **`batch-run` subcommand** for running many s7cmd commands from a
  script file (or `-` for stdin). Choose sequential or parallel
  execution (`--parallel N`, `0` picks the CPU count) and pick how
  failures are handled:
  - default: stop on the first non-zero exit (sequential) or stop
    spawning new commands (parallel; in-flight lines finish);
  - `--continue-on-error`: run every line regardless of outcome;
  - `--continue-on-warning`: keep running past warnings (exit codes
    `3` and `4`) but still stop on true failures;
  - `--max-errors N`: stop once `N` failures have been recorded.
    Parse/validation errors (typos, unknown subcommands, bad
    arguments) count the same as runtime failures, so
    `--max-errors 5` tolerates up to 5 broken lines anywhere.

  The final exit code is the worst seen across the whole batch,
  ranked by severity (`1` > `2` > `3` > `4` > any other non-zero >
  `0`) rather than by numeric value, so an actionable error always
  wins over a SIGINT skip or a "not found". A trailing summary
  `N succeeded, N failed, N warnings, N skipped, elapsed Ts` is
  written to stderr; suppress with `--no-summary`, or pass
  `--json-tracing` to emit it as a single-line JSON object instead.
- **Live progress bar in `batch-run`** drawn on TTY stderr in
  read-all mode while the run is in progress. Suppress with
  `--no-progress` (useful inside terminal multiplexers, `script(1)`,
  or CI runners). Streaming mode, non-TTY stderr, and
  `--json-tracing` already suppress the bar.
- **`--check-format`** validates a `batch-run` script without
  executing anything. It stops at the first problematic line —
  prefixed with the script source (file path, or `stdin` for `-`)
  and the line number — and exits 1; a clean script logs `format OK`.
- **Per-line tracing in `batch-run`**: each line logs a `start` event
  and a matching outcome — `success`, `warning (exit N)`,
  `skipped (exit 130)` (Ctrl-C / SIGINT), or `failure (exit N)` —
  prefixed with the line number and the original command. `start`
  and `success` are info level (silent at the default warn level —
  pass `-v`); `warning`, `skipped`, and `failure` are visible
  without `-v`. Per-line outcome buckets: `0` → succeeded;
  `3` / `4` → warnings; `130` → skipped (never counts toward
  `--max-errors`); other non-zero → failed.
- **`--dry-run` on every state-mutating subcommand** (`cp`, `mv`,
  `rm`, `create-bucket`, `delete-bucket`, all `put-*` and
  `delete-*`). Arguments and inputs are still validated, an
  `[dry-run]` log line describes what would have happened, and the
  command exits 0 without touching AWS. Read-only commands (`get-*`,
  `head-*`, `ls`) intentionally do not accept this flag.

### Changed

- `sync`, `clean`, and `ls` no longer terminate the process on their
  own — required so a single failing line inside `batch-run` does not
  kill the rest of the batch. Behavior of standalone invocations is
  unchanged.
- Bumped `s3util-rs` from 1.0 to 1.1 (vendored CLI sources synced to
  match).

### Fixed

- Dispatching `cp` / `mv` could overflow the thread stack on
  platforms with small default stacks; the inner futures are now
  boxed so dispatch is safe regardless of stack size.

## [0.1.3] - 2026-04-29

### Changed

- Documentation updates.

## [0.1.2] - 2026-04-29

### Added

- `CHANGELOG.md` documenting changes per Keep a Changelog format.

### Changed

- Bumped `s3util-rs` to `1.0.0` and synced vendored CLI sources to upstream `4edffac`. Under `--show-progress`, the destination line (`-> <path>`) is now printed unconditionally on success.
- Expanded README: scope, non-goals, requirements, installation, and AI-development disclosure.

### Removed

- Dropped Windows UAC manifest infrastructure (`s7cmd.manifest`, `s7cmd.rc`, `embed-resource` build dependency).

## [0.1.1] - 2026-04-29

### Changed

- Bumped `nix` from `0.30.1` to `0.31.2`.
- Dropped `windows-11-arm` runner from CI/CD; pre-built `aarch64-pc-windows-msvc` binaries are not available while the `LNK1322` (Cortex-A53 erratum #843419) build failure is unresolved.

## [0.1.0] - 2026-04-29

Initial preview release.

### Added

- Object operations: `ls`, `cp`, `mv`, `rm`, `sync`, `clean`.
- Object metadata: `head-object`, `get-object-tagging`, `put-object-tagging`, `delete-object-tagging`.
- Bucket operations: `create-bucket`, `delete-bucket`, `head-bucket`.
- Bucket-level configuration subcommands: tagging, policy, versioning, lifecycle, encryption, CORS, public-access-block, website, logging, notification.
- Shell completion generation (`--auto-complete-shell`) for bash, elvish, fish, powershell, and zsh.
- E2E test suite covering object/bucket operations against live AWS S3.
- Pre-built binaries for Linux (x86_64, aarch64), macOS (aarch64), and Windows (x86_64).
