# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Preview.** s7cmd is in an early/preview phase (0.x). The CLI surface, flag names, output formats, and exit codes may change between minor versions until 1.0.0.

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
