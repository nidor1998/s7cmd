# s7cmd

[![Crates.io](https://img.shields.io/crates/v/s7cmd.svg)](https://crates.io/crates/s7cmd)
[![GitHub](https://img.shields.io/github/downloads/nidor1998/s7cmd/total?label=downloads%20%28GitHub%29)](https://github.com/nidor1998/s7cmd/releases)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
![MSRV](https://img.shields.io/badge/msrv-1.91.1-red)
![CI](https://github.com/nidor1998/s7cmd/actions/workflows/ci.yml/badge.svg?branch=main)
[![codecov](https://codecov.io/gh/nidor1998/s7cmd/graph/badge.svg?token=XFICDPTMDG)](https://codecov.io/gh/nidor1998/s7cmd)

A reliable, flexible, and fast command-line tool for Amazon S3.

s7cmd combines the speed of a Rust async runtime with the breadth of
the AWS S3 API surface, providing high-throughput object operations
(`ls`, `cp`, `mv`, `rm`, `sync`, `clean`) alongside comprehensive
bucket administration (lifecycle, policy, encryption, CORS, public
access block, website, logging, notification, and more) — all from a
single static binary.

s7cmd is a thin command-line wrapper over four Rust libraries by the
same author: [s3sync](https://github.com/nidor1998/s3sync),
[s3ls-rs](https://github.com/nidor1998/s3ls-rs),
[s3util-rs](https://github.com/nidor1998/s3util-rs), and
[s3rm-rs](https://github.com/nidor1998/s3rm-rs). Originally, only
s3sync and s3rm-rs existed and were not intended to be merged, but
in response to user requests for a unified interface, the
functionality was split into focused libraries and bundled together
into a single binary as s7cmd. Built on `aws-sdk-rust` and `tokio`,
s7cmd targets workloads that demand both performance and operational
completeness: data engineering pipelines, ML training data
preparation, multi-account bucket governance, and integrity-critical
migrations.

## Why s7cmd?

- **Single binary, full coverage.** Object transfer, bulk delete, and
  every common bucket-level configuration in one tool.
- **Strong integrity verification.** Native support for SHA256, SHA1,
  CRC32, CRC32C, and CRC64NVME — aligned with S3's 2025 default
  checksum policy.
- **Predictable performance.** Configurable workers, multipart
  thresholds, and chunk sizes; bounded memory footprint suitable for
  small instances and large CI runners alike.
- **Apache-2.0 licensed.** No copyleft concerns for enterprise
  deployment or container distribution.

### Scope

s7cmd is designed to cover **Amazon S3 object operations and bucket
management** — listing (`ls`), single- and bulk-object transfers
(`cp` / `mv` / `rm`), recursive synchronization (`sync`), bulk delete
(`clean`), and the common bucket-level configurations (tagging,
versioning, policy, lifecycle, encryption, CORS,
public-access-block, website, logging, notification). For any S3 use
case outside that scope, use a more comprehensive tool such as the
[AWS CLI](https://aws.amazon.com/cli/) (`aws s3` / `aws s3api`).

s7cmd targets **Amazon S3** as its primary supported platform.
S3-compatible storage (MinIO, Cloudflare R2, Backblaze B2, Wasabi,
Ceph RGW, DigitalOcean Spaces, IBM COS, and similar) is supported
on a **best-effort basis only** — such services may work via
`--endpoint-url`, but they are not part of the official test matrix
and behavior may change between releases. This is a structural
consequence of building on `aws-sdk-rust`, which is generated from
AWS service models and assumes Amazon S3 semantics (checksum
headers, endpoint resolution, signing variants, response schemas);
features that depend on AWS-specific semantics, such as CRC64NVME
checksums or newer S3 API additions, may not work against
non-AWS endpoints. Bug reports against S3-compatible storage will
be triaged but not prioritized, and fixes are not guaranteed.

s7cmd is **not** intended to be a drop-in replacement for, or
behaviorally compatible with, any other S3 client — including the
AWS CLI (`aws s3`, `aws s3api`) and tools such as `s3cmd`, `s4cmd`,
`s5cmd`, and `s6cmd`. Its command-line flags, transfer semantics,
verification rules, and exit codes are designed around the
underlying libraries' own scope and design principles — not
interoperability with another tool's interface. Output formats and
flag names will not be adjusted to match any external tool, and
scripts written against another S3 client should not be expected to
work with `s7cmd` unmodified. The numeric progression in the name
(`s3cmd` → `s4cmd` → `s5cmd` → `s6cmd` → `s7cmd`) does **not** imply
succession or compatibility.

### Non-Goals

The following are explicitly out of scope and will not be added,
regardless of demand:

- Official support, testing, or guaranteed compatibility for any
  storage service other than Amazon S3. S3-compatible storage may
  work on a best-effort basis as described in the Scope section
  above, but adding dedicated code paths, provider-specific
  workarounds, or backends for services such as MinIO, Cloudflare
  R2, Backblaze B2, Wasabi, Ceph RGW, DigitalOcean Spaces, IBM COS,
  Tencent COS, Alibaba OSS, Azure Blob Storage, or Google Cloud
  Storage is out of scope.
- Feature parity with, or porting features from, other S3 clients.
  Feature requests of the form "tool X has feature Y, please add
  it to s7cmd" — including variants such as "feature Y would also
  be useful in s7cmd," "many users expect Y because tool X has it,"
  or "Y is missing compared to tool X" — will be closed without
  further discussion. The existence of a feature, flag, command,
  output format, or behavior in `aws s3`, `aws s3api`, `s3cmd`,
  `s4cmd`, `s5cmd`, `s6cmd`, or any other S3 tool carries no weight
  in s7cmd's design decisions, regardless of how the request is
  framed. Each feature is evaluated solely against s7cmd's own
  scope and the design principles of its underlying libraries. If
  the feature you need exists in another tool, use that tool.
- Outperforming other S3 tools on raw speed or memory usage.
  Performance and resource consumption are addressed only when they
  compromise practical workflows — not for edge cases or benchmark
  wins. Issues of the form "tool X transfers Y MB/s faster" or
  "tool X uses less RAM than s7cmd in benchmark Z" will be closed.
  If raw throughput is your top criterion, use a tool optimized
  for it.
- FUSE filesystem mounting, daemon mode, or any persistent
  background process. s7cmd is a one-shot CLI; it runs, transfers,
  and exits.
- Workflow orchestration features — scheduling, cross-run state
  databases, retry queues that survive process restart, or DAG
  execution. Use a workflow engine such as Airflow, Argo Workflows,
  or AWS Step Functions for orchestration.
- A graphical user interface, a TUI, or an interactive shell mode.
- A plugin or extension mechanism.
- AWS service coverage beyond S3. s7cmd will not add subcommands for
  IAM, KMS, CloudFront, or any other AWS service, even when they
  interact closely with S3.

Issues and pull requests requesting any of the above will be closed.

### Maintenance Model

s7cmd is maintained as a personal project. Dependency updates and
critical bug fixes are applied on a best-effort basis. New features
are not actively solicited. If you need guaranteed enterprise
support, this is not the tool for you.

### About the name

The name follows the `s3cmd` / `s4cmd` / `s5cmd` / `s6cmd` lineage,
but s7cmd is not affiliated with, derived from, or compatible with
any of them. The number 7 was chosen simply because it was the
next available one. There is no deeper meaning.

## Usage

```
Usage: s7cmd [OPTIONS] [COMMAND]

Object Operations:
  ls                                    List S3 objects
  cp                                    Copy objects from/to S3 (or S3 to S3)
  mv                                    Move objects from/to S3 (copy then delete source)
  rm                                    Delete a single S3 object
  sync                                  Synchronize files between local and S3 (or S3 to S3)
  clean                                 Bulk-delete S3 objects

Object Metadata:
  head-object                           Head an S3 object
  get-object-tagging                    Get an S3 object's tagging
  put-object-tagging                    Put tagging on an S3 object
  delete-object-tagging                 Delete tagging from an S3 object

Bucket Operations:
  create-bucket                         Create an S3 bucket
  delete-bucket                         Delete an S3 bucket
  head-bucket                           Head an S3 bucket

Bucket Tagging:
  get-bucket-tagging                    Get a bucket's tagging
  put-bucket-tagging                    Put tagging on a bucket
  delete-bucket-tagging                 Delete tagging from a bucket

Bucket Policy:
  get-bucket-policy                     Get a bucket's policy
  put-bucket-policy                     Put a bucket policy
  delete-bucket-policy                  Delete a bucket's policy

Bucket Versioning:
  get-bucket-versioning                 Get a bucket's versioning configuration
  put-bucket-versioning                 Put a bucket versioning configuration

Bucket Lifecycle:
  get-bucket-lifecycle-configuration    Get a bucket's lifecycle configuration
  put-bucket-lifecycle-configuration    Put a bucket lifecycle configuration
  delete-bucket-lifecycle-configuration Delete a bucket's lifecycle configuration

Bucket Encryption:
  get-bucket-encryption                 Get a bucket's encryption configuration
  put-bucket-encryption                 Put a bucket encryption configuration
  delete-bucket-encryption              Delete a bucket's encryption configuration

Bucket CORS:
  get-bucket-cors                       Get a bucket's CORS configuration
  put-bucket-cors                       Put a bucket CORS configuration
  delete-bucket-cors                    Delete a bucket's CORS configuration

Bucket Public Access Block:
  get-public-access-block               Get a bucket's public access block configuration
  put-public-access-block               Put a bucket public access block configuration
  delete-public-access-block            Delete a bucket's public access block configuration

Bucket Website:
  get-bucket-website                    Get a bucket's website configuration
  put-bucket-website                    Put a bucket website configuration
  delete-bucket-website                 Delete a bucket's website configuration

Bucket Logging:
  get-bucket-logging                    Get a bucket's logging configuration
  put-bucket-logging                    Put a bucket logging configuration

Bucket Notification:
  get-bucket-notification-configuration Get a bucket's notification configuration
  put-bucket-notification-configuration Put a bucket notification configuration

Batch:
  batch-run                             Run s7cmd commands from a file (or - for stdin)

Other:
  help                                  Print this message or the help of the given subcommand(s)

Options:
      --auto-complete-shell <SHELL>  Generate shell completions for s7cmd (all subcommands) and exit [possible values: bash, elvish, fish, powershell, zsh]
  -h, --help                         Print help (see more with '--help')
  -V, --version                      Print version
```

### `batch-run`

Reads s7cmd commands from a file (or stdin via `-`), one per line,
and executes them in the same process — avoiding the per-command
fork/exec, dynamic linker work, and Rust/tokio runtime startup you
would pay if you invoked `s7cmd` once per line from a shell loop.
(AWS SDK clients and TLS connections are still built per dispatched
command, so network-side overhead is not eliminated; the win is
process startup.) It is the recommended way to drive thousands of
small operations (per-object tagging, mixed bucket-config edits,
etc.) without spawning a process per command.

```text
Usage: s7cmd batch-run [OPTIONS] <FILE>
```

`<FILE>` is a path to a script file, or `-` to read from stdin
(mirrors `put-bucket-policy`).

**Input format.** One command per line. Each line is tokenized with
shell-style quoting (POSIX `shlex`), so quoted arguments with
spaces work as expected. Blank lines are ignored. Lines whose first
non-whitespace character is `#` are treated as comments and skipped.
Each line is parsed as if it were a top-level s7cmd invocation
*without* the leading `s7cmd` — i.e. start with the subcommand name:

```text
# create two buckets, then tag one of them
create-bucket s3://example-bucket-1
create-bucket s3://example-bucket-2
put-bucket-tagging --tagging "team=data&env=prod" s3://example-bucket-1

# upload a file with a key that contains spaces
cp ./report.csv "s3://example-bucket-1/reports/Q1 2026.csv"
```

Pass the script as a file argument, or pipe it in via `-`:

```sh
s7cmd batch-run commands.txt
s7cmd batch-run - < commands.txt
```

**Execution modes.** Two read modes and two parallelism modes,
freely combined:

| Flag | Effect |
|------|--------|
| (default) | Read the whole script first, validate every line, then execute. Catches bad lines before any line runs. Shows a progress bar when stderr is a TTY. |
| `--streaming` | Execute commands as they are read. No progress bar. Use for unbounded or pipelined input where buffering the whole script is undesirable. |
| `--parallel 1` (default) | Sequential execution. |
| `--parallel N` | Run up to *N* commands concurrently. |
| `--parallel 0` | Use all logical CPUs. |

**Failure handling.** By default, the first failing command stops
sequential execution and prevents new spawns in parallel
mode. Pass `--continue-on-error` to run every line regardless, or
`--max-errors N` (`N` ≥ 1) to keep running up to `N` failures and
then stop gracefully (sequential: stops after the N-th failure;
parallel: stops spawning new commands once N failures have been
recorded — in-flight commands complete). `--continue-on-error` and
`--max-errors` are mutually exclusive. The process exit code is
the worst (highest) seen across all executed commands.

**Format check.** Pass `--check-format` to validate the script
without executing anything. The walk stops at the first
tokenize / parse / validate problem (or read I/O error), reports
that line as a single error-level log entry, and exits 1. On a
clean pass an info-level "format OK" message is emitted. Verbosity
is forced to at least info so that message is visible at the
default warn level.

**Per-line tracing.** Each dispatched line emits a `start` event
and a matching outcome event (`ok`, `warning`, or `error (exit N)`)
at info level, prefixed with the line number and the original
input text. They are silent at the default warn level — pass `-v`
to see them.

**Tracing flags belong to `batch-run`, not per-line.** Pass
`--json-tracing`, `--aws-sdk-tracing`, `--span-events-tracing`,
`--disable-color-tracing`, and `-v`/`-q` to `batch-run` itself —
e.g. `s7cmd batch-run --aws-sdk-tracing commands.txt`. Lines that
set `--json-tracing`, `--aws-sdk-tracing`, `--span-events-tracing`,
or `--disable-color-tracing` are rejected at validation time;
per-line `-v`/`-q` is silently ignored (the tracing subscriber is
installed once, at the top of the run).

**Caveats and safety.**

- Even when you increase the parallelism level (`--parallel`), the
  various rate limits apply on a per-command basis (they are not
  divided across or aggregated over the workers).
- Increasing `--parallel` may increase the load on the operating
  system. It consumes CPU, memory, file descriptors, and other
  resources — pick a value the host and the target service can
  absorb.
- `batch-run` is a dangerous command and must be used with caution.
  Whenever possible, perform a dry run by using each subcommand's
  `--dry-run` flag, and pass `-v` to `batch-run` itself to surface
  the per-line info-level logs for preliminary verification.

**Restrictions.**

- Nested `batch-run` is rejected.
- `cp`/`mv` lines may not use `-` (stdin/stdout) as source or target.
- Per-line input is capped at 16 KiB.

**Summary.** When the run completes (or aborts), an
`N ok, N failed, N skipped, elapsed Ts` line is written to stderr.
Suppress it with `--no-summary`.

## Documentation

Each subcommand is documented in the README of its underlying
library. For details on flags, semantics, and exit codes, refer to:

| Subcommand                         | Documentation                                          |
| ---------------------------------- | ------------------------------------------------------ |
| `ls`                               | [s3ls-rs](https://github.com/nidor1998/s3ls-rs)        |
| `sync`                             | [s3sync](https://github.com/nidor1998/s3sync)          |
| `clean`                            | [s3rm-rs](https://github.com/nidor1998/s3rm-rs)        |
| `cp`, `mv`, `rm`, and all others   | [s3util-rs](https://github.com/nidor1998/s3util-rs)    |
| `batch-run`                        | s7cmd-only — see the section above                     |

Each of these projects (except `batch-run`) also ships its own
standalone binary, which can be used independently of s7cmd.

## Requirements

- x86_64 Linux (kernel 3.2 or later)
- ARM64 Linux (kernel 4.1 or later)
- Windows 11 (x86_64, aarch64)
- macOS 11.0 or later (aarch64, x86_64)

All features are tested on the above platforms.

## Installation

Download the latest binary from [GitHub Releases](https://github.com/nidor1998/s7cmd/releases)

You should build Intel Mac and ARM64 Windows binaries yourself.

## Fully AI-generated, always human-verified

No human wrote a single line of source code in this project. Every line of s7cmd's own source code (including the vendored adaptations from upstream), every test, all documentation, CI/CD configuration, and this README were generated by AI using [Claude Code](https://docs.anthropic.com/en/docs/claude-code/overview) (Anthropic). The same applies to three of the four underlying libraries: [s3util-rs](https://github.com/nidor1998/s3util-rs), [s3ls-rs](https://github.com/nidor1998/s3ls-rs), and [s3rm-rs](https://github.com/nidor1998/s3rm-rs). The fourth, [s3sync](https://github.com/nidor1998/s3sync), is human-written and serves as the reference architecture from which the AI-generated siblings were derived.

Human verification is a permanent policy, not a one-time event applied only to the initial build. Human engineers authored the requirements, design specifications, and s3sync reference architecture, and continue to review and verify every change to the design, source code, and tests. Every release is manually tested by humans before it ships, and all E2E test scenarios are verified against live AWS S3. No AI-generated change is released without human review and testing — this applies equally to the initial build and to all future updates, including dependency bumps, bug fixes, and new features. The development follows a spec-driven process: requirements and design documents are written first, and the AI generates code to match those specifications under continuous human oversight.

Every underlying library maintains 96%+ automated test coverage. This serves a dual purpose: it verifies that AI-generated code meets its specifications, and it ensures the project remains maintainable by hand — whether because AI tooling becomes unavailable, or because a future maintainer prefers to work without AI assistance. Combined with the modular library design and Apache-2.0 licensing, this means s7cmd can be safely forked and maintained without AI assistance if the need arises.

## License

Apache-2.0. See `LICENSE`.
