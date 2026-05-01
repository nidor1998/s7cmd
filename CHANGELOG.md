# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Preview.** s7cmd is in an early/preview phase (0.x). The CLI surface, flag names, output formats, and exit codes may change between minor versions until 1.0.0.

## [0.2.0] - 2026-05-01

### Added
- `batch-run <FILE>` subcommand: read s7cmd commands from a script file
  (or stdin via `-`, mirroring `put-bucket-policy`) and execute them
  in-process. Supports sequential or parallel (`--parallel N`, `0` =
  num_cpus), fail-fast or `--continue-on-error`, read-all (default with
  progress bar) or `--streaming`. Tracing flags must be passed to
  `batch-run` itself; per-line tracing flags are rejected. Final exit
  code is the worst (highest) seen across all executed commands.
- `--check-format` flag on `batch-run`: validates the script without
  running any command. Stops at the first problematic line, reports it
  as a single error-level log entry, and exits 1. A clean pass logs an
  info-level "format OK" message; verbosity is forced to at least info
  while this flag is set so the message is visible at the default warn
  level.
- `--max-errors <N>` flag on `batch-run`: stop spawning new commands
  once `N` (≥ 1) failures have been recorded (graceful: in-flight
  commands complete). Mutually exclusive with `--continue-on-error`.
  When neither flag is set, the run stops on the first failure (the
  historical default).
- `batch-run` per-line tracing: each dispatched line emits an
  info-level `start` event and a matching outcome event
  (`success`, `warning`, or `failure (exit N)`) prefixed with the
  line number and raw input text, so the active subcommand is
  identifiable in the log. Silent at the default warn level; pass
  `-v` to see them.
- `--dry-run` flag on every state-mutating subcommand (`cp`, `mv`, `rm`,
  `create-bucket`, all `put-*`, all `delete-*`). Argument validation,
  JSON parsing, and SDK setup run as normal; an info-level `[dry-run]`
  log line describes what would have happened, and the binary exits 0
  without making any AWS-side change. Read-only commands (`get-*`,
  `head-*`) deliberately do not accept this flag. Verbosity is forced to
  at least info while `--dry-run` is set so the message is visible at
  default verbosity.

### Changed
- `sync_bin::cli::run`, `clean_bin::run`, `ls_bin::run` now return
  `Result<i32>` instead of calling `std::process::exit` internally, so
  they can be invoked from `batch-run` without killing the process
  mid-batch. Single-subcommand invocations are behaviorally unchanged.
- Bumped `s3util-rs` from 1.0 to 1.1 and re-synced the vendored bin
  modules (`util_bin/cli/*.rs`) accordingly.

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
