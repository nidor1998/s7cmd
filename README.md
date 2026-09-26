# s7cmd

[![Crates.io](https://img.shields.io/crates/v/s7cmd.svg)](https://crates.io/crates/s7cmd)
[![GitHub](https://img.shields.io/github/downloads/nidor1998/s7cmd/total?label=downloads%20%28GitHub%29)](https://github.com/nidor1998/s7cmd/releases)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
![CI](https://github.com/nidor1998/s7cmd/actions/workflows/ci.yml/badge.svg?branch=main)
[![codecov](https://codecov.io/gh/nidor1998/s7cmd/graph/badge.svg?token=XFICDPTMDG)](https://codecov.io/gh/nidor1998/s7cmd)

A reliable, flexible, and fast command-line tool for Amazon S3.

s7cmd combines the speed of a Rust async runtime with the breadth of
the AWS S3 API surface, providing high-throughput object operations
(`ls`, `cp`, `mv`, `rename`, `rm`, `sync`, `clean`) alongside comprehensive
bucket administration (lifecycle, policy, encryption, object annotations, CORS, public
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

## Features highlights

s7cmd is designed to pursue reliability and performance as equal goals,
rather than trading one for the other.

- **Single binary, full coverage.** Object transfer, bulk delete, and
  every common bucket-level configuration in one tool.
- **Strong integrity verification.** Native support for SHA256, SHA1,
  CRC32, CRC32C, and CRC64NVME — aligned with S3's 2026 default
  checksum policy.
- **Predictable performance.** Configurable workers, multipart
  thresholds, and chunk sizes; bounded memory footprint suitable for
  small instances and large CI runners alike.
- **Apache-2.0 licensed.** No copyleft concerns for enterprise
  deployment or container distribution.

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
  rename                                Rename an S3 object within an Express One Zone bucket
  rm                                    Delete a single S3 object
  restore-object                        Restore an archived S3 object
  presign                               Generate a pre-signed URL for an S3 object (GET only)
  sync                                  Synchronize files between local and S3 (or S3 to S3)
  clean                                 Bulk-delete S3 objects

Object Metadata:
  head-object                           Head an S3 object
  get-object-tagging                    Get an S3 object's tagging
  put-object-tagging                    Put tagging on an S3 object
  delete-object-tagging                 Delete tagging from an S3 object

Object Annotation:
  get-object-annotation                 Download a named annotation payload from an S3 object
  put-object-annotation                 Attach a named annotation payload to an S3 object
  delete-object-annotation              Delete a named annotation from an S3 object
  list-object-annotations               List the annotations of an S3 object and print them as JSON

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

Bucket Replication:
  get-bucket-replication                Get a bucket's replication configuration
  put-bucket-replication                Put a bucket replication configuration
  delete-bucket-replication             Delete a bucket's replication configuration

Bucket Transfer Acceleration:
  get-bucket-accelerate-configuration   Get a bucket's transfer acceleration configuration
  put-bucket-accelerate-configuration   Put a bucket transfer acceleration configuration

Bucket Request Payment:
  get-bucket-request-payment            Get a bucket's request payment configuration
  put-bucket-request-payment            Put a bucket request payment configuration

Bucket Policy Status:
  get-bucket-policy-status              Get a bucket's policy status (whether it is public)

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

> Note that although batch-run avoids launching a separate process
> for each command, it still initializes a new AWS client per
> command. This incurs per-command overhead such as credential
> resolution, region resolution, and HTTP client setup, so batch-run
> is not intended for high-throughput parallel processing of large
> workloads. 
> In my EC2 environment, even after disabling IMDS, copying small files was limited to about 1,000 copies per second (with --parallel 32).
> Using the `sync` subcommand allows you to copy more than 3,000 files per second under similar conditions (though you'll need to adjust the `--worker-size` parameter), so there is a trade-off between flexibility and speed.

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
| (default) | Read the whole script first, validate every line, then execute. A line that can't be parsed or validated is not rejected up front: it fails with exit code `2` when execution reaches it, so the lines before it still run (see **Failure handling** below). To catch bad lines before any line runs, use `--check-format` first. Shows a progress bar when stderr is a TTY. |
| `--streaming` | Execute commands as they are read. No progress bar. Use for unbounded or pipelined input where buffering the whole script is undesirable. |
| `--parallel 1` (default) | Sequential execution. Lines run in script order. |
| `--parallel N` | Run up to *N* commands concurrently (max 1024; a larger value is rejected at parse time). Completion order is not guaranteed. |
| `--parallel 0` | Use all logical CPUs. Completion order is not guaranteed. |

Script order is preserved only with `--parallel 1`. With
`--parallel N` (or `--parallel 0`), commands may complete in any
order; do not rely on later lines observing the effects of earlier
ones.

Default mode loads the entire script into memory, so very large
scripts will use proportional memory. Use `--streaming` to execute
lines as they are read.

**Failure handling.** By default, the first failing command stops
sequential execution and prevents new spawns in parallel
mode. Pass `--continue-on-error` to run every line regardless, or
`--max-errors N` (`N` ≥ 1) to keep running up to `N` failures and
then stop gracefully (sequential: stops after the N-th failure;
parallel: stops spawning new commands once N failures have been
recorded — in-flight commands complete). Pass
`--continue-on-warning` to keep running past per-line warnings
(exit codes 3 and 4 — `EXIT_CODE_WARNING` and
`EXIT_CODE_NOT_FOUND`) while still stopping on true failures
according to `--max-errors` (or the default first-failure stop).
`--continue-on-error` is mutually exclusive with both
`--max-errors` and `--continue-on-warning`. The process exit code
is the worst seen across all executed commands, ranked by severity
rather than numeric value: `1` (error) > `2` (invalid args) > `3`
(warning) > `4` (not found) > any other non-zero (e.g. `130`
SIGINT) > `0`. So a run mixing exit `1` and exit `130` exits `1`,
not `130`.

Lines that can't be parsed or validated (quoting errors, unknown
subcommands, missing or invalid arguments, empty commands) count as
failures the same way runtime failures do — they synthesize exit
code `2`, log at error level, increment the `failed` bucket, and
count toward `--max-errors` / `--continue-on-error`. So
`--max-errors 5` will let you tolerate up to 5 typo'd lines anywhere
in the script. (True read I/O errors — line over the 16 KiB cap,
non-UTF-8 bytes, file unreadable — still abort the whole run.)

**Format check.** Pass `--check-format` to validate the script
without executing anything. The walk stops at the first
parse or validation problem (or read I/O error), reports
that line as a single error-level log entry — identifying the
script source (file path, or `stdin` for `-`) and the line
number — and exits 1. On a clean pass a `"format OK"` message
is emitted.

**Per-line tracing.** Each dispatched line emits a `start` event
and a matching outcome event (`success`, `warning (exit N)`,
`skipped (exit 130)`, or `failure (exit N)`) prefixed with the
line number and the original input text. `start` and `success`
are info level (silent at the default warn level — pass `-v` to
see them); `warning` and `skipped` are warn level and `failure`
is error level, all three visible without `-v`. If a line carries
an inline credential option (`--*-access-key`,
`--*-secret-access-key`, `--*-session-token`, `--*-sse-c-key`,
`--*-sse-c-key-md5`), its value is masked to `****` in the logged
text.

**Tracing flags belong to `batch-run`, not per-line.** Pass
`--json-tracing`, `--aws-sdk-tracing`, `--span-events-tracing`,
`--disable-color-tracing`, and `-v`/`-q` to `batch-run` itself —
e.g. `s7cmd batch-run --aws-sdk-tracing commands.txt`. Lines that
set `--json-tracing`, `--aws-sdk-tracing`, `--span-events-tracing`,
or `--disable-color-tracing` are rejected at validation time;
per-line `-v`/`-q` is silently ignored (the tracing subscriber is
installed once, at the top of the run).

**Caveats and safety.**

- If you're not concerned with performance, it's best to leave
  `--parallel` at its default setting and run the process in series.
  There are many factors to consider when parallelizing.
- Even when you increase the parallelism level (`--parallel`), the
  various rate limits apply on a per-command basis (they are not
  divided across or aggregated over the workers).
- Increasing `--parallel` may increase the load on the operating
  system. It consumes CPU, memory, file descriptors, and other
  resources — pick a value the host and the target service can
  absorb.
- On EC2 instances using an IAM instance profile, setting
  `--parallel` too high is likely to trigger IMDS-related errors
  (credential resolution hits the instance metadata service per
  command, and IMDS will throttle under heavy concurrent load).
- The failure threshold in parallel mode is "stop spawning new
  commands", not "cancel in-flight commands." When `--parallel N`
  is close to or exceeds the number of script lines, every line may
  already be in flight by the time the threshold trips, so the run
  completes as if no threshold were set. The threshold is most
  effective when the line count is significantly larger than `N`. To
  cancel work that is already in flight, send SIGINT (Ctrl-C); per-
  subcommand cancellation handlers propagate it into in-flight
  transfers.
- `batch-run` is a dangerous command and must be used with caution.
  Whenever possible, perform a dry run by using each subcommand's
  `--dry-run` flag, and pass `-v` to `batch-run` itself to surface
  the per-line info-level logs for preliminary verification.

For example, suppose you want to create two buckets and tag one of
them. First, prepare a dry-run script (`sample_dry_run.txt`) with
each subcommand's `--dry-run` flag baked in:

```text
# sample_dry_run.txt — preview only; nothing is sent to S3 except per-subcommand --dry-run client-side validation.
create-bucket --dry-run s3://example-bucket-1
create-bucket --dry-run s3://example-bucket-2
put-bucket-tagging --dry-run --tagging "team=data&env=prod" s3://example-bucket-1
```

Run it with `-v` on `batch-run` itself so the per-line `start` /
`success` events and the per-subcommand `[dry-run] would …` info
lines are visible. `--no-progress` is added so the live progress
bar (drawn by default on TTY stderr) does not interleave with
the log lines you want to read:

```console
$ s7cmd batch-run -v --no-progress sample_dry_run.txt
2026-04-30T23:34:11.178191Z  INFO line started line=2 event="start" command="create-bucket" raw="create-bucket --dry-run s3://example-bucket-1"
2026-04-30T23:34:11.282653Z  INFO [dry-run] would create bucket. bucket=example-bucket-1
2026-04-30T23:34:11.282756Z  INFO line completed line=2 event="success" exit_code=0 command="create-bucket" raw="create-bucket --dry-run s3://example-bucket-1"
2026-04-30T23:34:11.282762Z  INFO line started line=3 event="start" command="create-bucket" raw="create-bucket --dry-run s3://example-bucket-2"
2026-04-30T23:34:11.283018Z  INFO [dry-run] would create bucket. bucket=example-bucket-2
2026-04-30T23:34:11.283038Z  INFO line completed line=3 event="success" exit_code=0 command="create-bucket" raw="create-bucket --dry-run s3://example-bucket-2"
2026-04-30T23:34:11.283040Z  INFO line started line=4 event="start" command="put-bucket-tagging" raw="put-bucket-tagging --dry-run --tagging \"team=data&env=prod\" s3://example-bucket-1"
2026-04-30T23:34:11.283239Z  INFO [dry-run] would put bucket tagging. bucket=example-bucket-1
2026-04-30T23:34:11.283284Z  INFO line completed line=4 event="success" exit_code=0 command="put-bucket-tagging" raw="put-bucket-tagging --dry-run --tagging \"team=data&env=prod\" s3://example-bucket-1"
batch-run: 3 succeeded, 0 failed, 0 warnings, 0 skipped, elapsed 0.1s
```

Once the dry run looks correct, run the same commands without
`--dry-run` (`sample.txt`):

```text
# sample.txt — the real run; this DOES create buckets and apply tags.
create-bucket s3://example-bucket-1
create-bucket s3://example-bucket-2
put-bucket-tagging --tagging "team=data&env=prod" s3://example-bucket-1
```

```console
$ s7cmd batch-run -v --no-progress sample.txt
2026-04-30T23:35:42.418901Z  INFO line started line=2 event="start" command="create-bucket" raw="create-bucket s3://example-bucket-1"
2026-04-30T23:35:43.512214Z  INFO Bucket created. bucket=example-bucket-1
2026-04-30T23:35:43.512410Z  INFO line completed line=2 event="success" exit_code=0 command="create-bucket" raw="create-bucket s3://example-bucket-1"
2026-04-30T23:35:43.512430Z  INFO line started line=3 event="start" command="create-bucket" raw="create-bucket s3://example-bucket-2"
2026-04-30T23:35:44.601877Z  INFO Bucket created. bucket=example-bucket-2
2026-04-30T23:35:44.602008Z  INFO line completed line=3 event="success" exit_code=0 command="create-bucket" raw="create-bucket s3://example-bucket-2"
2026-04-30T23:35:44.602020Z  INFO line started line=4 event="start" command="put-bucket-tagging" raw="put-bucket-tagging --tagging \"team=data&env=prod\" s3://example-bucket-1"
2026-04-30T23:35:44.881342Z  INFO Bucket tagging set. bucket=example-bucket-1
2026-04-30T23:35:44.881455Z  INFO line completed line=4 event="success" exit_code=0 command="put-bucket-tagging" raw="put-bucket-tagging --tagging \"team=data&env=prod\" s3://example-bucket-1"
batch-run: 3 succeeded, 0 failed, 0 warnings, 0 skipped, elapsed 2.5s
```

The per-subcommand `[dry-run] would …` info lines are replaced by
their concrete counterparts (`Bucket created.`,
`Bucket tagging set.`); everything else — the `start` / `success`
events, the trailing summary — is the same shape.

Without `-v`, both the per-line `start` / `success` events and
the per-subcommand info lines (`[dry-run] would …`,
`Bucket created.`, etc.) are suppressed at the default warn
level — only warnings and errors are logged, plus the trailing
summary line on stderr. That is why the safety guidance pairs
`--dry-run` with `-v`: you need info-level output to see what
*would* happen.

**Restrictions.**

- Nested `batch-run` is rejected.
- `cp`/`mv` lines may not use `-` (stdin/stdout) as source or target.
- Per-line input is capped at 16 KiB.

**Summary.** When the run completes (or aborts), an
`N succeeded, N failed, N warnings, N skipped, elapsed Ts` line is
written to stderr. Per-line outcomes bucket as: exit `0` →
`succeeded`; exit `3` or `4` (`EXIT_CODE_WARNING`,
`EXIT_CODE_NOT_FOUND`) → `warnings`; exit `130` (the conventional
Unix code for SIGINT — returned by per-subcommand cancellation
handlers when the user hits Ctrl-C) → `skipped` (logged at warn
level, not error, and never counted toward `--max-errors`); any
other non-zero exit → `failed`; lines that were never dispatched
(fail-fast or `--max-errors` threshold tripped, or SIGINT) →
`skipped`.
Suppress the line with `--no-summary`. With `--json-tracing` the
same information is emitted as a single-line JSON object instead,
e.g. `{"summary":"batch-run","succeeded":48,"failed":1,"warnings":2,"skipped":1,"elapsed_seconds":3.4}`.

**Progress bar.** In read-all mode, when stderr is a TTY, a live
progress bar is drawn on stderr while the run is in progress.
Suppress it with `--no-progress` — useful when stderr is a TTY
but you want machine-readable log output (terminal multiplexers,
`script(1)`, some CI runners). Streaming mode and non-TTY stderr
already suppress the bar. `--json-tracing` also suppresses it
automatically (the bar would interleave with JSON output).
`--no-progress` and `--no-summary` are independent — each controls
only its own visual element; pass both for fully clean output.

## Proxy support

s7cmd respects the standard proxy environment variables
(`HTTP_PROXY`, `HTTPS_PROXY`) automatically.
No flags are required — set the variables in your shell and every
subcommand routes its S3 traffic through the proxy.

Proxy authentication is supported via the URL form
`http(s)://user:password@proxy:port`.

## Documentation

Each subcommand is documented in the README of its underlying
library. For details on flags, semantics, and exit codes, refer to:

| Subcommand                         | Documentation                                          |
| ---------------------------------- | ------------------------------------------------------ |
| `ls`                               | [s3ls-rs](https://github.com/nidor1998/s3ls-rs)        |
| `sync`                             | [s3sync](https://github.com/nidor1998/s3sync)          |
| `clean`                            | [s3rm-rs](https://github.com/nidor1998/s3rm-rs)        |
| `cp`, `mv`, `rename`, `rm`, and all others | [s3util-rs](https://github.com/nidor1998/s3util-rs)    |
| `batch-run`                        | s7cmd-only — see the section above                     |

Each of these projects (except `batch-run`) also ships its own
standalone binary, which can be used independently of s7cmd.

**Exit code on interruption (v1.8.0+).** Every subcommand that catches
Ctrl-C (SIGINT) for graceful shutdown — `sync`, `ls`, `clean`, `cp`,
and `mv` — exits with code `130` (128 + SIGINT, the conventional shell
encoding for termination by signal) when interrupted, and the
interruption takes precedence over any errors or warnings the aborted
run had recorded. The pinned library releases ship the same behavior
in their standalone binaries (s3sync 1.62.0, s3util-rs 1.10.0,
s3rm-rs 1.6.0, s3ls-rs 1.3.0), so s7cmd and the upstream tools now
agree on interruption exit codes; the vendored frontends differ only
in returning the code instead of calling `process::exit`, so
`batch-run` survives an interrupted line. Subcommands that finish in a
single API call
install no handler, so Ctrl-C terminates them through the default
signal disposition — which shells also report as `130`. At `clean`'s
confirmation prompt, Ctrl-C likewise terminates immediately via the
default handler. Inside `batch-run`, an interrupted line is bucketed
`skipped (exit 130)` and the batch exit code follows the severity
ranking described in the `batch-run` section above.

## Security assumptions

s7cmd is built on a fundamental security assumption: **both the object storage system and the specific bucket you
operate on must be trusted.**

Within this trust model, s7cmd implements the security measures you would reasonably expect of an S3 command-line
tool: encrypted transport (TLS/HTTPS) for data in transit, end-to-end integrity verification for transfers (ETag,
MD5, SHA256, and CRC checksums), support for server-side encryption, secure handling of credentials through the
standard AWS credential providers (with credential environment-variable values hidden from `--help` output), and —
for `ls` — control-character escaping of the object keys, prefixes, and owner names returned by S3.
These measures protect the confidentiality and integrity of your data against transport-level and accidental
threats.

However, s7cmd assumes that the storage endpoint is honest and non-adversarial — that it correctly implements the
S3 API and returns the data, listings, metadata, and checksum values it actually stores, without tampering. Because
subcommands decide what to transfer, list, or delete from the listings and metadata the endpoint reports, the
integrity-verification and filtering features are **not** a defense against a malicious or compromised storage
backend that deliberately returns falsified data, fabricated listings, or forged checksums. Against such an
adversarial endpoint, these guarantees do not hold.

Crucially, trust must extend to the **bucket**, not just the storage provider. Even when the object storage system
itself is fully trustworthy, a bucket can still be adversarial — for example, a bucket you do not control, a shared
bucket writable by others, or one whose objects, metadata, or checksums were crafted by an attacker. If you
synchronize from, copy from, list, or delete from such a bucket, the data and metadata it serves are already
untrusted at the source, and s7cmd's guarantees no longer apply. A trusted storage provider hosting an untrusted
bucket is, for the purposes of this security model, an untrusted source.

Operating on an untrusted, compromised, or non-conformant endpoint or bucket is outside s7cmd's security model.
Selecting a trustworthy storage provider, and ensuring that every bucket you operate on is one you control or
trust — including its credentials, encryption, and access policies — remains your responsibility.

This mirrors the "Security assumptions" sections of the underlying
[s3sync](https://github.com/nidor1998/s3sync) / [s3util-rs](https://github.com/nidor1998/s3util-rs) /
[s3rm-rs](https://github.com/nidor1998/s3rm-rs) / [s3ls-rs](https://github.com/nidor1998/s3ls-rs) projects.

## Requirements

- x86_64 Linux (kernel 3.2 or later)
- ARM64 Linux (kernel 4.1 or later)
- Windows 11 (x86_64, aarch64)
- macOS 11.0 or later (aarch64)

All features are tested on the above platforms.

## Installation

Download the latest binary from [GitHub Releases](https://github.com/nidor1998/s7cmd/releases)

## Fully AI-generated, always human-verified

Every line of s7cmd's own source code (including the vendored adaptations from upstream), every test, all documentation, CI/CD configuration, and this README were generated by AI using [Claude Code](https://docs.anthropic.com/en/docs/claude-code/overview) (Anthropic). The same applies to three of the four underlying libraries: [s3util-rs](https://github.com/nidor1998/s3util-rs), [s3ls-rs](https://github.com/nidor1998/s3ls-rs), and [s3rm-rs](https://github.com/nidor1998/s3rm-rs). The fourth, [s3sync](https://github.com/nidor1998/s3sync), is human-written and serves as the reference architecture from which the AI-generated siblings were derived.

Human verification is a permanent policy, not a one-time event applied only to the initial build. Human engineers authored the requirements, design specifications, and s3sync reference architecture, and continue to review and verify every change to the design, source code, and tests. Every release is manually tested by humans before it ships, and all E2E test scenarios are verified against live AWS S3. No AI-generated change is released without human review and testing — this applies equally to the initial build and to all future updates, including dependency bumps, bug fixes, and new features. The development follows a spec-driven process: requirements and design documents are written first, and the AI generates code to match those specifications under continuous human oversight.

Every underlying library maintains 96%+ automated test coverage. This serves a dual purpose: it verifies that AI-generated code meets its specifications, and it ensures the project remains maintainable by hand — whether because AI tooling becomes unavailable, or because a future maintainer prefers to work without AI assistance. Combined with the modular library design and Apache-2.0 licensing, this means s7cmd can be safely forked and maintained without AI assistance if the need arises.

### Quality verification (by AI self-assessment)

Measurements below are taken at commit `37d493d` on branch `fix/annotation-stdout-flush` (version 1.8.5, not yet tagged — `v1.8.4` plus five commits; measured 2026-09-26). The coverage figures are sourced from `lcov_report.txt` (`cargo llvm-cov`; `lcov.info` is the matching machine-readable LCOV artifact) and reflect a single combined run — `cargo llvm-cov` with `RUSTFLAGS="--cfg e2e_test"` on the maintainer's machine, 2026-09-26 — so the unit tests, the process-level CLI and batch-run tests, and the live-AWS e2e suite are all included in the report.

| Metric                         | Value                                                         |
|--------------------------------|---------------------------------------------------------------|
| Production code                | ~17,900 lines of Rust across 85 source files in `src/`        |
| Unit tests (in `src/`)         | 506 `#[test]` / `#[tokio::test]` annotations                  |
| CLI integration tests          | 732 annotations across 65 files (64 `tests/cli_*.rs` files plus `tests/batch_run.rs`); they spawn the real binary with no AWS credentials — S3 interactions, where exercised, hit an in-process loopback mock server; run in CI |
| E2E integration tests          | 258 annotations across 28 `tests/e2e_*.rs` files (gated behind `--cfg e2e_test`; run only by the maintainer against live AWS) |
| Code coverage (llvm-cov, combined unit + CLI + e2e run) | 97.54% regions (376 / 15,262 missed), 97.09% functions (35 / 1,201 missed), 98.50% lines (163 / 10,876 missed) |
| Static analysis (clippy)       | 0 warnings (`cargo clippy --all-features`)                    |
| Formatting                     | 0 diffs (`cargo fmt --all --check`)                           |
| Supply chain (cargo-deny)      | Clean (`cargo deny -L error check`); runs on every push and PR in `ci.yml` and daily at 01:34 UTC in `cargo-deny.yml`; `advisories.ignore = []` |
| Code adapted from the underlying projects | CLI frontends vendored from the upstream binaries — `src/sync_bin/` ([s3sync](https://github.com/nidor1998/s3sync)), `src/util_bin/` ([s3util-rs](https://github.com/nidor1998/s3util-rs)), `src/clean_bin/` ([s3rm-rs](https://github.com/nidor1998/s3rm-rs)), `src/ls_bin/` ([s3ls-rs](https://github.com/nidor1998/s3ls-rs)) — plus adapted dispatch arms in `src/dispatch.rs`; the engines themselves are consumed as exact-pinned library dependencies (s3sync 1.62.3, s3util-rs 1.10.5, s3rm-rs 1.6.4, s3ls-rs 1.3.4). `src/batch_run/` is s7cmd-original |

What these numbers do and do not show:
- They show what the combined test run exercises — including the live-AWS e2e suite — not how the binary behaves under production load over time. CI asserts only the non-e2e build (unit, CLI, and batch-run tests) on every push and PR, across seven build targets; it neither runs the e2e suite nor produces or gates on coverage.
- Coverage measures s7cmd's own `src/` — the wrapper layer: the clap argument surfaces, subcommand dispatch, the vendored CLI frontends, and batch-run. The transfer, listing, and bulk-delete engines are separate crates measured in their own repositories (each underlying library maintains 96%+ coverage, as noted above); their figures are not part of the numbers in this table.
- Coverage is a structural metric. A covered line can still be incorrect; an uncovered line can still be correct. Use it to size the test surface, not to certify behaviour.
- The e2e suite covers live-AWS paths (object, bucket-configuration, and stdio round-trips, object annotations, batch-run, dry-run, ctrl-c cancellation, exit codes) but runs only on the maintainer's machine, and reproducing the coverage figures above requires AWS credentials.

The codebase is built through spec-driven development with human review at every step. Test counts and coverage will change as dependencies are updated and refinements land.

### AI assessment of safety and correctness (by Claude, Anthropic)

<details>
<summary>Click to expand the full assessment</summary>

> Assessment date: 2026-09-26.
>
> Assessor: Claude Opus 5.5 by Anthropic (model ID `claude-opus-5-5`), effort setting: max.
>
> Assessed version: s7cmd 1.8.5, source code at commit `8e91eeb` (branch `fix/annotation-stdout-flush`). Its Rust code is identical to that of commit `37d493d`; `8e91eeb` changed only this README, and the supplied coverage files were written between the two commits. 1.8.5 had not been tagged; the latest tag was `v1.8.4`. Compared with that tag, the Rust code differs in `src/util_bin/cli/get_object_annotation.rs` (the stdout flush fix described in the 1.8.5 changelog entry), in comments of two `batch-run` files, and in two test files, and the `s3util-rs` pin moved from 1.10.4 to 1.10.5.
>
> Basis: this section was written from scratch for this version. Every statement below comes from the source code, the tests, the supplied coverage files, the published source of the pinned engine crates and of other crates locked in `Cargo.lock`, or an experiment run for this assessment. The two statements about how Amazon S3 itself handles versioned deletes (finding 18) describe documented S3 behavior and were not tested. Before publication, each statement was checked again against its evidence.

#### Scope and method

- **Source code.** All 85 Rust files under `src/` (17,903 lines) were read in full: 8,984 lines of program code (comments included) and 8,919 lines of unit tests in `#[cfg(test)]` modules. They are the entry point and dispatch (`src/main.rs`, `src/cli.rs`, `src/dispatch.rs`, `src/pipe_safe.rs`), the s7cmd-original `batch-run` engine (`src/batch_run/`, 8 files, 4,746 lines), and the four frontends copied ("vendored") from the engine libraries' own command-line programs (`src/util_bin/` 59 files, `src/sync_bin/` 6, `src/ls_bin/` 3, `src/clean_bin/` 5).
- **Comparison with upstream.** Each of the 72 vendored files that has a counterpart in an upstream program (every file in the four frontend directories except the two that only declare modules, plus `src/pipe_safe.rs`) was compared token by token with that program's source in the pinned engine release, ignoring comments, whitespace and test modules. 39 are identical; the differences in the other 33 are described under *Architecture*.
- **Engine crates and AWS SDK.** Where s7cmd relies on behavior implemented elsewhere, the published source of the pinned release was read: s3sync 1.62.3, s3util-rs 1.10.5, s3rm-rs 1.6.4 and s3ls-rs 1.3.4, and, for logging and timeouts, `aws-sigv4` 1.5.1, `aws-smithy-runtime` 1.14.0 and `aws-config` 1.12.0 as locked in `Cargo.lock`.
- **Tests.** All 94 Rust files under `tests/` (25,465 lines) were read in full. Test annotations were counted, every test function was checked for exit-code assertions that accept any runtime result, the bucket names used by the live-AWS tests were traced, and every statement below about a test was checked against its file.
- **Checks.** Re-run with rustc and cargo 1.98.1, cargo-deny 0.20.2 and cargo-llvm-cov 0.9.1 on macOS (arm64). The offline tests were run with the AWS configuration and credentials files redirected to paths that do not exist, so that, as on CI runners, no AWS credentials were available. The live-AWS tests were compiled and linted but not run: they need AWS credentials and are run by the maintainer.
- **Coverage.** The supplied `lcov.info` and `lcov_report.txt` were analyzed record by record, split into program code and test code, and compared with a measurement of the offline tests alone, made for this assessment on the same source code.
- **Experiments.** A debug build of commit `8e91eeb` was run against loopback HTTP servers written for this assessment. They log every request and can answer every request with 200 (optionally after a delay), answer with a 404 that has no body, serve one versioned object (optionally with a wrong ETag), or accept connections and never answer. Signals were sent by a driver that restores the default SIGINT handling in the child process, and a pseudo-terminal was used where a terminal was needed. Only fake credentials were configured and `AWS_ENDPOINT_URL` pointed at a loopback server, so no request could reach AWS.

#### Checks re-run for this assessment

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | No differences |
| `cargo clippy --all-features -- -D warnings` | No warnings |
| `cargo clippy --all-features --all-targets -- -D warnings` | No warnings |
| The same, with `RUSTFLAGS="--cfg e2e_test"` (compiles and lints the live-AWS tests) | No warnings |
| `cargo test --all-features --locked` (offline tests, no AWS credentials available) | 1,249 passed, 0 failed, 0 ignored, in 94 test binaries |
| `cargo deny -L error check`, RustSec advisory database at its commit of 2026-09-25 | advisories ok, bans ok, licenses ok, sources ok |

The 1,249 tests are the 506 unit tests in `src/`, the 732 offline tests under `tests/`, and the 11 unit tests of `src/cli.rs`, which run a second time because `tests/cli_routing.rs` includes that file.

#### Tests and coverage

The tests contain 1,496 test annotations:

- 506 unit tests in `src/`.
- 732 offline tests in 65 files under `tests/` (64 `tests/cli_*.rs` files and `tests/batch_run.rs`). 670 of them start the real binary; the other 62, in `tests/cli_routing.rs`, check argument parsing in-process. Apart from seven tests (finding 6), the offline tests either stop before any S3 request (argument errors, `--dry-run`) or point the binary at a loopback server that replays prepared HTTP responses (the shared mock server, started 37 times in 17 files, or a similar server in `tests/cli_sigint_exit_code.rs`) or at a closed local port.
- 258 live-AWS tests in 28 `tests/e2e_*.rs` files, compiled only with `--cfg e2e_test`. The ones that contact AWS use bucket names that contain a freshly generated random identifier (for example `s7cmd-e2e-<uuid>`); the few fixed bucket names in these files appear only in tests that stop at argument validation.

The 68 process-level `batch-run` tests (`tests/batch_run.rs`) check success or failure (45 `.failure()` checks) but never a specific exit code; the exit-code rules are checked by the executor's unit tests. The two tests named `*_drains_unread_stdin` pass a script file, for which `batch-run` does not drain stdin (`src/batch_run/mod.rs:62-65`), so they cannot detect whether draining works.

The live-AWS tests were read, not run. Of the 34 dry-run tests in `tests/e2e_dry_run.rs`, 32 read the S3 state back after the dry run, and the two `restore-object` tests instead rely on a real call failing; no live-AWS test runs `sync`, `clean`, `put-object-annotation` or `delete-object-annotation` with `--dry-run` (`rename`'s dry run is tested in `tests/e2e_rename.rs`). Transferred bytes are compared with the originals in `tests/e2e_object_ops.rs`, `tests/e2e_object_annotations.rs`, `tests/e2e_rename.rs` and `tests/e2e_presign.rs`.

CI builds and runs the offline tests on seven targets; on the two Windows targets, the Unix-only signal tests are compiled out. CI does not compile the live-AWS test code and does not measure coverage.

The supplied coverage files were written at 13:54 local time on 2026-09-26, after commit `37d493d` (13:47), and their totals agree with each other. The right-hand column is the measurement made for this assessment, running only the offline tests:

| Metric | Supplied files (unit, offline and live-AWS tests) | Offline tests only |
| --- | --- | --- |
| Lines | 98.50% (163 of 10,876 missed) | 94.49% (599 missed) |
| Regions | 97.54% (376 of 15,262 missed) | 93.19% (1,040 missed) |
| Functions | 97.09% (35 of 1,201 missed) | 97.00% (36 missed) |
| Branches | not recorded | not recorded |

- These totals include the unit-test code inside `src/`. `lcov.info` holds 10,192 line records, one per source line, and 5,959 of them (58%) are in `#[cfg(test)]` modules. Of the 4,233 records in program code, 62 were never executed in the supplied data (98.5% executed) and 510 in the offline measurement (88.0% executed). The offline tests, which are the tests CI runs, therefore executed about 88% of the program code in this measurement, not 98.50%. llvm-cov's own totals count 10,876 line entries rather than 10,192 records, and 163 missed entries rather than the 79 never-executed records (62 in program code, 17 in test code).
- By area, the offline tests alone executed 97.8% of `batch-run`'s program code, 90.6% of the `clean` frontend's and 79.6% of the `util_bin` frontend's; in the supplied data these are 98.0%, 93.6% and 99.6%. 448 program-code records were executed only in the supplied data and none only in the offline measurement, which is consistent with the supplied data being the offline tests plus the live-AWS tests. Code reached only by the live-AWS tests includes the S3-to-S3 and stdin/stdout branches of the copy wrapper (`src/util_bin/cli/mod.rs:437-564`), the success paths of most `get-*`, `put-*` and `delete-*` commands, `rename`, and `clean`'s refusal to run without `--force` in a non-interactive session.
- Of the 62 program-code records never executed in the supplied data, 55 handle conditions that the tests cannot produce or that the pinned code does not reach: failure to register a Ctrl-C handler (8), a closed pipe while log output is flushed (4), a panic in a background task (16), a failed write of the shell-completion script (4), and branches that earlier validation or the engines' behavior makes unreachable, such as `unreachable!` arms and error arms of functions that always return `Ok` (23). The other 7 can be reached in normal use: `cp` exiting 3 after a verification warning (`src/util_bin/cli/cp.rs:42`; the live-AWS test that checks this exit code starts the binary through `cargo run`, and the coverage data records no execution of the line), an answer other than `yes` at `clean`'s confirmation prompt (`src/clean_bin/mod.rs:76-78`), a Ctrl-C that arrives while `batch-run --streaming --parallel` waits for the next line or a free worker (`src/batch_run/executor.rs:575-576`), and the validation path used on a machine with one logical CPU (`src/batch_run/mod.rs:631`). The experiments below exercised the first two.
- The figures cover s7cmd's `src/` only; the engine crates and the AWS SDK are not measured. Coverage shows which code ran during the tests, not whether its results were checked.

#### Architecture: what s7cmd's own code does

s7cmd has 56 subcommands. For 55 of them, `src/dispatch.rs` converts the parsed arguments into the configuration of one of four engine libraries, each pinned to an exact version, and calls a frontend vendored from that library's own program: s3sync for `sync`, s3ls-rs for `ls`, s3rm-rs for `clean`, and s3util-rs for the other 52. The 56th, `batch-run`, is s7cmd's own: it tokenizes each script line, parses it with the same parser as the top-level command, validates it, and runs the lines in one process, one at a time or up to 1,024 at once. The exit codes produced are 0, 1, 2, 3, 4, 101 and 130; `src/main.rs:80` converts them with `ExitCode::from(code as u8)`, and all of them fit in a byte.

The vendored frontends return an exit code instead of ending the process, so that `batch-run` can run many subcommands in one process. The only process-ending calls in s7cmd's code are clap's, for top-level argument errors and `--help`/`--version` output (`src/main.rs:57-59`); `batch-run` parses its lines with `try_get_matches_from` and does not reach them. In the engine libraries, the only such calls are s3util-rs's three `validate_state_flag` functions, which s7cmd does not call (it has its own non-exiting `check_state_flag` functions), and one in a documentation example in s3rm-rs. `batch-run` runs each line inside `catch_unwind` (`src/batch_run/executor.rs:130`), so a panic in a subcommand becomes exit code 101 for that line; all build profiles use `panic = "unwind"`, which this requires. There is no `unsafe` code outside test modules. The program code has 21 `unwrap`, `expect` or `unreachable!` sites. Nineteen cannot fail, because of a preceding check, a hard-coded value or an upstream validation. The other two re-raise a panic that has already happened elsewhere: a poisoned lock in `sync`'s status report (`src/sync_bin/cli/mod.rs:95`), and a panicking thread in `batch-run`'s validation of scripts of 256 lines or more (`src/batch_run/mod.rs:650`), which ends the run before any line executes.

The 33 vendored files that differ from the pinned upstream programs differ in the following ways. Adaptations for running many subcommands in one process: exit codes are returned instead of passed to `process::exit`; the upstream `main`, configuration-loading and completion-script functions are removed; the three state-flag checks do not exit; the log subscriber is installed with `try_init`, with filters that add s7cmd's own log targets (in the s3util-rs frontend, whose subscriber `batch-run` uses, also those of the other three engines); and upstream's registration of user-defined Rust callbacks, which upstream enables only through a test flag, is absent. Log messages: program names are changed, and 18 `get-*`, `head-*` and `restore-object` frontends log "not found" at warn level where upstream uses error level (the exit code, 4, is the same). Behavior: `cp` and `mv` dry runs return earlier than upstream, before the storage objects and the progress indicator are created, and log "would copy." rather than "would copy object."; `head-object` reports a missing bucket and a missing key with the same message; and `get-object-annotation` verifies the temporary file before renaming it over the destination, where upstream renames first. The rest are import paths, visibility and module order. No fix present in the upstream programs was found missing from s7cmd.

Build and dependencies: the four engines are pinned with `=` versions, and `Cargo.lock` (402 packages) is committed. TLS in the shipped binary is rustls 0.23.45 with the aws-lc-rs provider; `ring` is in `Cargo.lock` only through development dependencies. `deny.toml` bans `openssl-sys`, which is absent, keeps an empty advisory-ignore list and allows only crates.io as a source; cargo-deny runs in CI on every push, on pull requests to `main`, and on a daily schedule. The release workflow builds with `--locked`, publishes SHA-256 files and GitHub build-provenance attestations with the archives, and publishes to crates.io through trusted publishing rather than a stored token.

#### Behavior observed in experiments

| Scenario | Observed |
| --- | --- |
| All 34 subcommands that accept `--dry-run`, against the logging server | No PUT, POST or DELETE request. Read-only requests were sent by `create-bucket --if-not-exists` (HEAD), `cp --skip-existing` (HEAD), `sync` and `clean` (object listing). After `mv --dry-run`, the local source file was still present. |
| `put-bucket-policy` and the eight other `put-*` commands that read a JSON file, with `--dry-run` and a file that is not JSON | `put-bucket-policy` exited 0 (the policy is not parsed); the other eight exited 1 with a JSON parse error. No request in any case. |
| `cp --dry-run` and `mv --dry-run` from a local file that does not exist | Exit 0, "[dry-run] would copy.". Without `--dry-run`, `cp` exited 1 with "source file not found" and sent no request. |
| `mv` onto itself, written directly and in directory form (`s3://b/d/k` to `s3://b/d/`) | Rejected with "cannot mv an object onto itself", exit 1, no request. With the same endpoint written as `http://127.0.0.1:<port>` for the source and `http://localhost:<port>` for the target, the check did not fire (finding 8). |
| `mv` from a versioned S3 object to a local file | HEAD, then GET with `versionId=V1`, then DELETE with `versionId=V1`. `rm` without `--source-version-id` sent DELETE without a version ID. |
| Download whose ETag does not match the data | `cp` exited 3, logged "e_tag mismatch. file in the local storage may be corrupted.", and kept the downloaded file. `mv` exited 1 and sent no DELETE; with `--no-fail-on-verify-error`, it exited 0 and deleted the source version. |
| `clean` without `--force` | With no terminal: refused ("Cannot run destructive operation without --force (-f) in a non-interactive environment (no TTY)"), exit 2, no request. In a terminal: answering `no` printed "Deletion cancelled." and exited 0; no request was sent before the answer. |
| `--help` of all 56 subcommands with the 10 credential environment variables set | Variable names shown; no value shown. |
| Logs with credentials on the command line (`head-bucket`, `head-object`, `ls`, `clean`, `sync`, `cp`, a `batch-run` line; default level, `-v`, `-vv`, `-vvv`, `-vv` and `-vvv` with `--aws-sdk-tracing`, `RUST_LOG=trace`) | The secret access key and the session token never appeared. The SSE-C key and its MD5 appeared in full for real requests of `head-object`, `sync`, `cp` and a `batch-run` line with `-vvv --aws-sdk-tracing` or `RUST_LOG=trace` (finding 4). The access key ID appeared in full in the SDK's debug and trace output and in `sync`'s configuration trace, and partly masked in the `ls` and `clean` configuration traces. |
| `batch-run` logging lines that carry credential options | Values masked as `****` for correctly spelled options, in `--flag value` and `--flag=value` form, on invalid lines and on lines that fail to tokenize. Values printed in full for misspelled options (finding 3). |
| `batch-run`, default mode: valid line, invalid line, valid line | Line 1 ran (its PUT reached the server), exit 2, line 3 skipped, as the README describes. `--check-format` on the same script reported line 2, exit 1, no request. |
| `batch-run` of three `head-bucket`, `put-bucket-tagging` or `cp` lines, server answering after 3 s, SIGINT at 1 s | `head-bucket` and `put-bucket-tagging`: exit 0 in sequential, `--parallel 2` and `--streaming` mode, with 1 or 2 lines counted as skipped (finding 2). `cp`: exit 130. |
| `batch-run --parallel 2` and `--parallel 4`: invalid line 1, valid lines 2–8, server answering after 1 s | Lines 2–3 (`--parallel 2`) and lines 2–5 (`--parallel 4`) ran in each of three runs; the sequential run ran none (finding 1). |
| Endpoint that accepts connections and never answers | Without a timeout option, `head-bucket`, `cp` and `ls` were still waiting after 60 s. At SIGINT, `head-bucket` and `put-bucket-tagging` ended at once, `sync` and `clean` ended with exit 130, and `ls`, `cp` and `batch-run` (sequential and `--parallel 2`) were still running after two SIGINTs and ended on SIGTERM. With `--operation-timeout-milliseconds 3000`, `head-bucket` failed with exit 1 after about 3 s, also as a `batch-run` line; `--read-timeout-milliseconds 3000` had the same effect (finding 5). |
| Long batches, `--streaming -q` | Maximum resident set size 33, 39 and 59 MiB for 2,000, 8,000 and 32,000 `cp --dry-run` lines; 44, 45 and 45 MiB for the same numbers of `put-bucket-tagging --dry-run` lines (finding 10). |
| `--streaming`: a line that exits 4, then a line over 16 KiB | Exit 4, summary "0 failed". After a successful line instead: exit 1 (finding 11). |
| `batch-run --streaming -` with a writer that keeps stdin open for 12 s | SIGINT at 2 s: summary at 2 s, exit when the writer closed stdin (12 s). Invalid first line, which reaches the failure limit at once: summary and exit 2 only at 12 s (finding 12). |
| `batch-run -qq` or `-qqq` with a `cp -vv` line | The following lines' info-level events, including `batch-run`'s own per-line events, were printed; with `batch-run` at its default level or `-q`, they were not (finding 9). |
| A trailing word after all arguments | `rm s3://b1/k zsh` sent the DELETE and exited 0; `head-bucket s3://b1 notashell` was rejected with "invalid value 'notashell' for '[SHELL]'" (finding 7). |
| Offline tests with a default AWS profile (fake keys) and `AWS_ENDPOINT_URL` pointed at a logging server | 7 tests in 4 files sent signed `PutBucketVersioning` (2), `PutBucketAccelerateConfiguration` (2), `PutBucketRequestPayment` (2) and `RestoreObject` (1) requests for bucket `example`; no other offline test sent a request to that endpoint, and all test binaries passed. Without credentials, the same commands failed before sending (finding 6). |
| stderr redirected to a file | ANSI color codes written by the s3util-rs family (for example `get-bucket-policy`) and by `batch-run`; none by `ls`, `clean` and `sync`, or with `--disable-color-tracing` (finding 14). |
| `--check-format` on a `sync` line whose local source directory does not exist yet | Rejected ("source file/directory not found"). A `cp` line with a missing local source was accepted (finding 13). |
| Ctrl-C at `clean`'s confirmation prompt, in a terminal | Standalone: the process ended at once. As a `batch-run` line: the prompt kept waiting; answering `no` gave "Deletion cancelled." and a summary of 1 succeeded, exit 0 (finding 15). |
| `get-object-annotation` replacing an existing file of mode 0644, with no AES256 ETag and no checksum to verify | Exit 0 with a warning that integrity could not be verified; the new file had mode 0600 (findings 17 and 20). |
| A bucket that does not exist (404 `NoSuchBucket`), and an S3 path without a key | Missing bucket: exit 4 for `head-bucket`, `get-bucket-tagging`, `get-object-tagging`, `put-object-annotation` and `restore-object`; exit 1 for `put-bucket-tagging`, `delete-bucket-tagging`, `put-bucket-cors`, `put-object-tagging`, `cp` and `rename`. No key: exit 1 for `presign`, `head-object`, `rm` and `get-object-tagging` (finding 16). |
| `batch-run --parallel 1025`; a script line consisting only of `--` | Rejected when arguments are parsed, exit 2 (1024 accepted); the `--` line failed as invalid, exit 2. |

#### Findings, ordered by potential impact

Severity levels used here: **medium** means that, under conditions an operator can plausibly meet, the problem can lead to an S3 operation the operator did not intend, an exit status of 0 for work that was not done, a secret value in a log, or a process that does not stop on Ctrl-C; **low** means a wrong exit code, summary or log output in narrower conditions, a check that an unusual invocation can bypass, resource growth, or a problem in the test suite rather than the program; **informational** means behavior an operator should know about, or an observation about maintenance, not a defect.

1. **Medium: with `--parallel`, `batch-run` can start one more line after the failure limit is reached.** In `run_parallel` and `run_parallel_streaming`, the loop that starts lines checks the failure flag before it waits for a free worker slot, but after it gets the slot it re-checks only the interrupt flag (`src/batch_run/executor.rs:398-418` and `:546-577`). When all slots are busy and a running line reaches the limit, the slot that line frees is used to start the next line. In the experiment, with an invalid line 1 and the default limit of one failure, line 3 (`--parallel 2`) or line 5 (`--parallel 4`) started in each of three runs, although the README states that parallel mode "stops spawning new commands once N failures have been recorded". The streaming variant has the same gap in its code; the experiment did not trigger it there. The default sequential mode (`--parallel 1`) is not affected. The extra line can be any line of the script, including one that deletes or overwrites. Fix: check the failure flag again after getting the slot.
2. **Medium: an interrupted `batch-run` can exit 0.** The batch exit code is the worst code among the lines that ran (`src/batch_run/executor.rs:294-314`); lines that never started because of Ctrl-C are counted as skipped but do not affect it, and a unit test (`sequential_interrupt_stops_before_first_command`) asserts exit 0 for a run in which every line was skipped. This follows the README's rule (the exit code is "the worst seen across all executed commands"), but it means that when SIGINT arrives between lines, or during a line that has no Ctrl-C handling of its own (for example `head-bucket`, `rm`, or a `get-*`, `put-*` or `delete-*` command) and that line then completes, the batch exits 0 although lines were not run. In the experiment, summaries such as "1 succeeded, 0 failed, 0 warnings, 2 skipped" came with exit 0 in sequential, `--parallel 2` and `--streaming` mode. A caller that checks only the exit status, such as `s7cmd batch-run a.txt && next-step`, treats the interrupted run as complete. When the running line handles Ctrl-C itself (`cp`, `mv`, `sync`, `clean`, and `ls` when listing objects), it returns 130 and the batch exits 130. Workaround: after an interruption, read the summary line. Fix: report 130 when the interrupt flag has stopped the run.
3. **Medium: a misspelled credential option in a `batch-run` line is logged with its value.** `redact_secrets` (`src/batch_run/redact.rs:29-71`) masks the value that follows an option whose name contains `access-key`, `session-token` or `sse-c-key`. A line with a misspelled option is rejected by the parser and logged at error level, which is visible at the default verbosity, in plain text and in `--json-tracing` output; `--check-format` does the same. The value was logged in full for `--target-secret-acces-key`, `--target-secret-key`, `--target-sessiontoken` and `--TARGET-SECRET-ACCESS-KEY` (the match is case-sensitive), for a correctly spelled option name written in quotes on a line that also has an unbalanced quote, and for `--target-secret-access-key= VALUE` (a space after `=`). The value appeared in the logged line text, not in the logged error reason. Workaround: pass credentials through AWS profiles or environment variables, not script lines. Fix: when a line is rejected, mask the value after any unknown option whose name contains `key`, `secret` or `token`, or omit the line text.
4. **Medium: with trace-level AWS SDK logging, the SSE-C key is written to the log.** With `--aws-sdk-tracing` and `-vvv`, the log filter enables the SDK's `aws_sigv4` module at trace level (`src/util_bin/tracing_init.rs:55-58`; the frontends of the other three engines use the same filter), and the SDK's "signing request" event (`aws-sigv4` 1.5.1, `src/http_request/sign.rs:263`) lists every request header, including `x-amz-server-side-encryption-customer-key` and its MD5. The key appeared in full for real requests of `head-object`, `sync`, `cp` and a `batch-run` line; `RUST_LOG=trace` has the same effect. It did not appear at `-vv`, without SDK tracing, or in dry runs; `sync`'s configuration trace at `-vvv` contains the key's MD5 but not the key. Because the key travels in a request header, it makes no difference whether it was given as an option or through an environment variable, and `batch-run`'s masking of its own line log does not cover this SDK event. The secret access key and the session token did not appear in any log examined. Workaround: do not combine SSE-C with trace-level SDK logging, or treat such logs as secret. Fix: cap `aws_sigv4` below trace level in the filter, or document the exposure.
5. **Medium for unattended use: a request to an endpoint that stops answering waits indefinitely, and a second Ctrl-C does not end `ls`, `cp` or `batch-run`.** All four engines set SDK timeouts only when `--operation-timeout-milliseconds`, `--operation-attempt-timeout-milliseconds`, `--connect-timeout-milliseconds` or `--read-timeout-milliseconds` is given (`build_timeout_config` in each engine's `src/storage/s3/client_builder.rs`); otherwise only the SDK's default connect timeout of 3.1 s applies (`aws-smithy-runtime` 1.14.0, `src/client/defaults.rs`). Against an endpoint that accepts connections and never answers, `head-bucket`, `cp` and `ls` were still waiting after 60 s. The Ctrl-C handlers of `cp` and `ls` act on the first SIGINT only (`mv` uses `cp`'s code), and cancelling the operation did not interrupt a request that was waiting for its response; `sync` and `clean` ended at the first SIGINT with exit 130. Standalone single-request subcommands install no handler and end at the first SIGINT, but inside `batch-run` they are not interrupted, because `batch-run`'s listener has taken over SIGINT. With `--operation-timeout-milliseconds 3000`, the stalled command failed with exit 1 after about 3 s, also as a `batch-run` line. Workaround: set a timeout option for unattended runs, and stop a stalled run with SIGTERM. Fix: exit on a second SIGINT, or set a default read or operation timeout.
6. **Low (test suite): seven offline tests send signed write requests to the default S3 endpoint when AWS credentials are available.** `enabled_alone_with_valid_bucket_parses_ok` and `suspended_alone_with_valid_bucket_parses_ok` (in both `tests/cli_put_bucket_versioning.rs` and `tests/cli_put_bucket_accelerate_configuration.rs`), `requester_alone_with_valid_bucket_parses_ok` and `bucket_owner_alone_with_valid_bucket_parses_ok` (`tests/cli_put_bucket_request_payment.rs`), and `standard_tier_with_days_parses_ok` (`tests/cli_restore_object.rs`) run a write command against a bucket named `example` without an endpoint override, without `--dry-run`, and without the environment scrubbing that the shared helpers in `tests/common/mod.rs` provide. They assert only that the exit code is not 2, so they pass whatever happens to the request. With a default AWS profile present, the experiment recorded seven signed requests; without credentials, as on CI runners, the same commands fail before a request is sent. `tests/README.md` states that the offline tests talk only to a loopback mock endpoint, and the headers of these four files state that they run without AWS credentials or network access. This affects people who run `cargo test` on a machine with AWS credentials, not users of the binary. Fix: add `--dry-run` or the mock-server arguments to these tests.
7. **Low: every subcommand except `batch-run` accepts a hidden trailing argument.** `cli_command()` removes the long name of the `--auto-complete-shell` option that each upstream argument struct declares (`src/cli.rs:471-477`), so that the option is rejected on subcommands; clap then treats that argument, which has neither a long nor a short name, as a positional argument (`[SHELL]`, or `[AUTO_COMPLETE_SHELL]` for `ls` and `clean`). In all 55 affected subcommands it comes after the other positional arguments, so it receives only an extra word at the end. A trailing shell name (`bash`, `elvish`, `fish`, `powershell` or `zsh`) is accepted and ignored: `rm s3://b1/k zsh` sent the DELETE request and exited 0. Any other extra word is rejected with "invalid value '…' for '[SHELL]'" (or `[AUTO_COMPLETE_SHELL]`), which does not say that the word was unexpected. The argument does not appear in `--help`. Fix: reject a value for this argument after parsing.
8. **Low: the `mv` self-move check compares endpoints and bucket names as text.** `check_not_self_move` (`src/util_bin/cli/mv.rs:46-98`) treats the source and target as different services whenever their endpoint strings differ (`:65-77`), as its own comment notes. With the same local endpoint written as `http://127.0.0.1:<port>` for the source and `http://localhost:<port>` for the target, the check did not fire, and the dry run reported that it would delete the source object. A real run would copy the object onto itself and then delete the source, which, as the function's comment states, destroys the object on an unversioned bucket. Without endpoint options, or with identical spellings, the check fired.
9. **Low: with `batch-run -qq` or `-qqq`, the first `cp`, `mv`, `sync`, `ls` or `clean` line sets the log level for the rest of the run.** At these levels `batch-run` installs no log subscriber (`src/batch_run/args.rs:18-35`); these five subcommands then install one from their own per-line settings (`src/dispatch.rs:37-39, 63, 79, 100, 116`), and it stays for the rest of the process. After a `cp -vv` line, the following lines' info-level events, including `batch-run`'s own per-line events, were printed. The README states that per-line `-v`/`-q` is ignored because the subscriber is installed once at the top of the run; that holds at `batch-run`'s default level and at `-q`.
10. **Low: each `cp`, `mv`, `sync`, `ls` or `clean` line in a batch leaves a Ctrl-C listener task running.** These frontends start a task that waits for Ctrl-C or for cancellation of the operation's token (for example `src/util_bin/cli/ctrl_c_handler.rs:32-52`). The pinned engines cancel the token on errors and stop conditions, not after a normal completion, so the task stays until the process exits. Single invocations are unaffected. In a batch, memory grows with the number of such lines, by about 0.9 KiB per line in the measurement above (33 to 59 MiB from 2,000 to 32,000 `cp --dry-run` lines, against 44 to 45 MiB for `put-bucket-tagging --dry-run` lines). Fix: stop the task or cancel the token when the operation returns.
11. **Low: in `--streaming` mode, a script read error is ranked by numeric value.** When the script reader fails (a line over 16 KiB, bytes that are not UTF-8, or a panic in the reader task), the run returns `code.max(1)` (`src/batch_run/mod.rs:410-420`). After a line that exited 3 or 4, the batch exits with that code instead of 1, and the summary does not count the read error as a failure (experiment: exit 4, "0 failed"); the error itself is logged. The default mode reads the whole script before running any line and exits 1 without running anything, and without writing the summary line that the README says is written when a run completes or aborts. Fix: use `worse_of(code, 1)` and count the read error as a failure.
12. **Low: `batch-run --streaming -` does not exit until the writer closes stdin.** The reader uses tokio's stdin, whose documentation states that the read runs on a separate thread and cannot be cancelled, so process shutdown waits for it. After Ctrl-C, the run stopped and wrote its summary at 2 s, but the process exited only when the writer closed stdin, at 12 s. After the failure limit is reached, the executors also keep reading until the end of input (`src/batch_run/executor.rs:493-515` and `:600-603`), so with an invalid first line the summary and the exit status (2) came only at 12 s. Workaround: close the writer as well.
13. **Low: `sync` lines are checked against the local filesystem when the script is validated.** Validation builds the s3sync configuration (`src/batch_run/validate.rs:122-132`), and s3sync rejects a local source that does not exist. In the default mode every line is validated before the first one runs, so a `sync` line that reads a directory created by an earlier line of the same script fails with exit 2, and `--check-format` rejects it. `cp` lines are not checked for this at validation time. Workaround: create the directory before running the script.
14. **Low: color codes are written to stderr when it is not a terminal.** `src/util_bin/tracing_init.rs:46` enables ANSI output unless `--disable-color-tracing` is given, while the `ls`, `clean` and `sync` frontends also require their output stream to be a terminal. This affects the s3util-rs family of subcommands and `batch-run`; the upstream s3util-rs 1.10.5 program has the same code. Workaround: pass `--disable-color-tracing` when logs are redirected.
15. **Low: inside `batch-run`, Ctrl-C at `clean`'s confirmation prompt does not end the process.** Standalone, `clean` installs its Ctrl-C handler only after the prompt (`src/clean_bin/mod.rs:66-87`), so Ctrl-C at the prompt ends the process, as the README describes. In `batch-run`, the batch's own SIGINT listener is already installed (`src/batch_run/mod.rs:295-303` and `:346-354`); in the experiment the prompt kept waiting after Ctrl-C, and answering `no` printed "Deletion cancelled." while the batch counted the line as succeeded and exited 0. Nothing was deleted. The prompt appears only when stdin is a terminal; without one, `clean` refuses to run without `--force`.
16. **Low: the same condition leads to different exit codes in different subcommands.** A bucket that does not exist gave exit 4 ("not found") for `head-bucket`, `get-bucket-tagging`, `get-object-tagging`, `put-object-annotation` and `restore-object`, but exit 1 for `put-bucket-tagging`, `delete-bucket-tagging`, `put-bucket-cors`, `put-object-tagging`, `cp` and `rename`: the `put-*` and `delete-*` bucket-configuration and object-tagging runners return plain errors, which `src/dispatch.rs:474-482` maps to 1, and `rename` turns its not-found results into errors (`src/util_bin/cli/rename.rs:52-57`). An S3 path without a key, which these commands reject after argument parsing, gave exit 1 for `presign`, `head-object`, `rm` and `get-object-tagging`, although the README maps invalid arguments to 2; a unit test (`dispatch_presign_bucket_only_path_returns_one`) pins exit 1 for `presign`. In `batch-run` with `--continue-on-warning`, a missing bucket therefore lets the run continue after some commands (exit 4 counts as a warning) and stops it after others. Apart from log levels, the runners involved match the upstream s3util-rs 1.10.5 programs.
17. **Informational: exit code 3 means "warning" in `batch-run`, and some warnings concern the data.** `batch-run` treats 3 and 4 as warnings (`src/batch_run/executor.rs:281-287`): `--continue-on-warning` continues past them and the summary counts them as warnings, not failures; without that option, exit 3 stops a batch like a failure. `cp` exited 3 when a downloaded object's ETag did not match, logged that the file "may be corrupted", and kept the file; an upload whose ETag did not match exited 1. `mv` exits 1 in the warning case and keeps its source, unless `--no-fail-on-verify-error` is given. `create-bucket` exits 3 when the bucket was created but tagging it failed (`src/util_bin/cli/create_bucket.rs:83-91`), and `clean` exits 3 when some objects could not be deleted (s3rm-rs maps a partial failure to 3). Exit 0 does not always mean that integrity was verified: `get-object-annotation` writes the payload and exits 0, with a warning, when neither an AES256 ETag nor an additional checksum is available (`src/util_bin/cli/get_object_annotation.rs:341-359`).
18. **Informational: on a versioned source bucket, `mv` removes the copied version permanently.** `mv` deletes the source with the version ID captured when the copy started (`src/util_bin/cli/mv.rs:129-146`), so no delete marker is created and bucket versioning keeps no copy of that version at the source; the data exists at the target. Deleting the captured version, instead of whatever is current, means that a newer version written to the key during the copy is not deleted. On Amazon S3, the most recent remaining version of the key then becomes current. `rm` without `--source-version-id` sends a delete without a version ID, which on a versioned bucket adds a delete marker.
19. **Informational: what a dry run does not check.** `put-bucket-policy --dry-run` reads the policy file but does not parse it; the other eight `put-*` commands that take a JSON file parse it before the dry-run exit. `cp --dry-run` and `mv --dry-run` do not check that a local source file exists. Some dry runs send read-only requests (see the table). Most commands build their AWS client before checking `--dry-run`; when no region is configured, building it queried the EC2 instance metadata service for the region (three token requests to a redirected metadata endpoint in the experiment, none with `--target-region` or `AWS_REGION` set). No S3 request is involved.
20. **Informational: other observations.** `get-object-annotation` replaces `<OUTFILE>` with a file created with owner-only permissions (mode 0600, the `tempfile` crate's default), whatever the previous file's mode was, and does not call `fsync` before the rename. The access key ID can appear in full in debug- and trace-level logs (see the table). `src/batch_run/validate.rs:24-33` checks `mv` lines for stdin/stdout, which s3util-rs already rejects, so the branch is not reachable. Comments in `src/cli.rs` (lines 492-494 and 630-631) name upstream releases (s3sync 1.61.0, s3rm-rs 1.5.0, s3ls-rs 1.2.0, s3util-rs 1.9.0) as the ones s7cmd pins, which is no longer the case. In CI, clippy does not lint test code (`cargo clippy` without `--all-targets`), the test job does not use `--locked`, and the live-AWS test code is not compiled. The release workflow (`cd.yml`) does not wait for CI results, and all GitHub Actions but one are referenced by version tag rather than by commit SHA. The `Dockerfile` builds with the floating `rust:1-trixie` image and without `--locked`. `.cargo/min-publish-age.toml` takes effect only when it is passed explicitly to nightly Cargo with `--config`, as its own comment states. The README's example of the `--json-tracing` summary lists its keys in a different order from the program's output, which sorts them alphabetically; JSON readers are not affected.

No finding involves s7cmd acting on an object or bucket other than one named on its command line or in a script line.

#### What this assessment does not establish

- The live-AWS tests were not run. Behavior against Amazon S3 is covered by those tests, which the maintainer runs, and by the engine crates' own tests; the experiments here used loopback servers, which show which requests s7cmd sends, not how Amazon S3 answers them.
- The engine crates and the AWS SDK were read only where s7cmd depends on them. Defects inside them are outside this assessment.
- Experiments ran on macOS (arm64) only. CI runs the offline tests on six more targets.
- S3-compatible services other than Amazon S3 were not tested.
- No fuzzing, sanitizer run or formal verification was performed.
- Tests and experiments can show that defects exist, not that none remain. This assessment applies to the source code at commit `8e91eeb` only.

#### Conclusion: can s7cmd be relied on?

**Answer: yes, for its documented purpose, if the precautions below are followed.** For single commands, the precautions are to set a timeout option for unattended runs (finding 5) and not to combine trace-level AWS SDK logging with SSE-C keys (finding 4). For `batch-run` scripts, findings 1 to 3 add three more. Within these conditions, the evidence gathered here supports relying on s7cmd. It does not show that s7cmd is free of defects: sixteen were found (findings 1 to 16), five of them medium, and none of them made s7cmd act on an object or bucket that the command or script did not name.

In plain terms:

- **What "reliable" was taken to mean.** The tool does what the command asks, does not change or delete anything it was not asked to, and reports failure through its exit status (the number a program returns when it ends, which scripts and schedulers use to decide whether it worked).
- **What was checked.** Every file of s7cmd's own code was read: about 9,000 lines of program code, 8,900 lines of built-in tests, and the 94 separate test files. The 1,249 automated tests that run without an AWS account were run, and all passed. The formatting and code-analysis tools reported no problems, and the dependency audit found no known security advisory among the 402 packages in `Cargo.lock`. According to the supplied coverage data, the tests, including the maintainer's tests against real AWS, execute about 98.5% of s7cmd's program code (test code excluded); the tests that run automatically on every change execute about 88%. The program was also run in the experiments listed above against a stand-in server on this machine, to observe what it actually does rather than what the documentation says.
- **What held.** In preview mode (`--dry-run`), none of the 34 commands that offer it sent a request that changes anything, although a preview does not catch every mistake (finding 19). `mv` deleted the original only after the copy had succeeded and passed verification, unless told otherwise with `--no-fail-on-verify-error`; it deleted exactly the version it had copied, and it refused to move an object onto itself when both sides were written the same way. `clean` refused to bulk-delete without `--force` when no one could confirm, and deleted nothing when the answer at its prompt was not `yes`. The secret access key and the session token did not appear in help text or in any log examined, except in the cases of finding 3. Every command that failed in the experiments ended with a non-zero exit status; an interrupted `batch-run` is the exception described in finding 2.
- **What did not hold.**
  - With `--parallel`, `batch-run` can start one more command after its failure limit has been reached (finding 1).
  - A `batch-run` stopped with Ctrl-C (the interrupt key) can report success although some lines never ran (finding 2).
  - A misspelled credential option in a script is written to the log together with the secret value (finding 3).
  - With the most detailed AWS SDK logging switched on, an SSE-C encryption key is written to the log (finding 4).
  - If the storage service stops answering, a command waits indefinitely unless a timeout option is given, and for some commands pressing Ctrl-C again does not stop it (finding 5).
  - Seven of the project's own tests that are meant to run offline contact AWS when the computer running them has AWS credentials (finding 6). This concerns the test suite, not the program.

Each of the first five has a workaround: keep the default one-line-at-a-time mode for scripts that delete or overwrite, and check scripts with `batch-run --check-format` before running them; after interrupting a `batch-run`, read its summary line instead of relying on the exit status; keep credentials in AWS profiles or environment variables, not in scripts; do not combine SSE-C with trace-level SDK logging, or treat such logs as secret; set `--operation-timeout-milliseconds` (or another timeout option) for unattended runs, and stop a stalled run with SIGTERM (for example `kill <pid>`). As with any tool that deletes or overwrites data, preview destructive work with `--dry-run`, and keep a backup or bucket versioning for data that cannot be recreated, bearing in mind finding 18 for `mv`.

This assessment judged the program by its code and by its observed behavior, neither of which depends on who or what wrote the code. It does not verify the development process described elsewhere in this README.

</details>

### AI assessment of safety and correctness (by Codex)

<details>
<summary>Click to expand the full assessment</summary>

#### Scope and evidence

Assessed on **2026-09-26**, for **s7cmd 1.8.5**, at source commit `37b6c7598f6109f8c2233188242c8bee454c60a2`.
The application-source review covered **all 85 Rust files under `src/` (17,903 physical lines, including embedded tests)** and `build.rs`: command parsing and dispatch, every batch module, every object and bucket command, all four command frontends, cancellation, tracing, output, and progress reporting. Build and dependency configuration, the Dockerfile, CI/release workflows, test infrastructure, and relevant process-level tests were also examined. Findings below come from this source review, the supplied coverage artifacts, and fresh local checks.

The reviewed repository delegates storage operations, transfer algorithms, retries, configuration conversion, and substantial validation to dependencies. `Cargo.toml` and `Cargo.lock` specify `s3sync 1.62.3`, `s3util-rs 1.10.5`, `s3rm-rs 1.6.4`, and `s3ls-rs 1.3.4`. Reviewing every application source file does **not** constitute a complete audit of those libraries, the AWS SDK, their transitive dependencies, or AWS itself. No claim of defect-free operation follows from this review.

#### Latest coverage and fresh verification

The supplied [`lcov.info`](lcov.info) and `lcov_report.txt`, identified for this assessment as the latest combined E2E/unit coverage results, agree on file, line, and function totals. Summing the per-file report rows also reproduces its region totals:

| Metric | Executed | Total | Unexecuted | Coverage |
|---|---:|---:|---:|---:|
| Lines | 10,713 | 10,876 | 163 | 98.50% |
| Functions | 1,166 | 1,201 | 35 | 97.09% |
| Regions | 14,886 | 15,262 | 376 | 97.54% |
| Branches | No measurements | No measurements | Not established | Not established |

There are **83 source-file records**. The two `src/` files without records, `sync_bin/mod.rs` and `util_bin/mod.rs`, contain only module declarations. `build.rs` and dependency implementations are outside these reported totals. The totals **include embedded unit-test functions and test helpers**, including fake-storage methods; they are not production-code-only percentages. In particular, 100% line coverage for `mv.rs` does not establish that every move scenario is safe.

These artifacts record execution, not whether every assertion passed or whether the assertions were sufficient. They provide no separate E2E-only and unit-only percentages, no measured branch coverage, and no embedded source revision or run configuration establishing exact provenance. A line executed once can still behave incorrectly for another input, scheduling order, or service response.

Fresh checks on macOS with Rust/Cargo 1.98.1 produced these results:

- `cargo fmt --all --check`: passed.
- `cargo clippy --all-features --all-targets --locked --offline -- -D warnings`: passed.
- `cargo test --all-features --locked --offline`: **1,249 passed, 0 failed, 0 ignored**: 506 binary unit tests and 743 integration tests. The initial sandboxed attempt failed while loading native TLS root certificates; the same command passed outside the sandbox.

The fresh test run did **not** enable `cfg(e2e_test)`, so live-AWS E2E tests were not rerun. The supplied coverage remains separate evidence from that fresh run. Some E2E tests return early when optional account/role configuration is absent; these artifacts alone cannot establish that those scenarios ran. Local reproductions below used synthetic secrets, invalid commands, or dry-runs and did not mutate live S3 data.

#### Safeguards present in the code

- **Moves check the copy outcome before deletion.** [`mv.rs`](src/util_bin/cli/mv.rs) retains the source after a reported copy failure, cancellation, or verification warning unless the warning override is enabled. It checks cancellation again before deletion and uses an explicit source version, or the version returned by the transfer, when available. It also rejects matching source/destination bucket, resolved key, and endpoint strings, subject to its explicit-version exception. The endpoint limitation is described below.
- **Mutating commands have dry-run paths.** The object and bucket runners return before their mutating API calls; the move dry-run omits source deletion. Sync status reporting forces dry-run. Dry-run is not necessarily network-free: existence checks and annotation inspection can still read remote state. It does not establish that a later write will be authorized or succeed.
- **Annotation downloads protect an existing output file during verification.** [`get_object_annotation.rs`](src/util_bin/cli/get_object_annotation.rs) limits payload size, checks reported length and applicable integrity metadata, writes to a neighboring temporary file, rereads it, and persists it only after those checks. Verification errors leave the existing destination untouched. This path uses `flush`, not a file-and-directory durability protocol, so it does not establish survival of a power failure.
- **Batch lines are parsed as arguments, not executed by a shell.** The parser limits each input line to 16 KiB. Validation rejects nested batches and specified stdin-consuming operations. Explicit parallelism is limited to 1,024. Dispatch returns command statuses, and batch execution catches unwinding panics from command futures. These mechanisms do not undo completed writes or make a batch transactional.
- **Output errors have explicit handling.** Report printing and listing treat a closed output pipe as normal pipeline termination. Annotation payload output propagates write/flush errors. Credential environment values are hidden in CLI help, and batch logging masks ordinary spellings of known credential flags. That masking has confirmed gaps below.

#### Confirmed findings and their consequences

**1. The move self-target check compares endpoint strings, not storage identity.** In [`check_not_self_move`](src/util_bin/cli/mv.rs), unequal endpoint strings bypass the same-bucket/key rejection. A local `mv --dry-run s3://b/k s3://b/k` with source endpoint `https://s3.ap-northeast-1.amazonaws.com` and target endpoint `https://s3.ap-northeast-1.amazonaws.com/` exited 0 and reported both a copy and source deletion. This confirms the guard bypass, not an observed live deletion. If two endpoint spellings reach the same unversioned object and the copy succeeds, the subsequent source deletion targets that same object. The existing guard therefore does not cover every self-move.

**2. Batch secret redaction is incomplete.** [`redact.rs`](src/batch_run/redact.rs) scans the raw line for literal flag substrings before tokenizing it. The spelling `--target-secret-access-\key` is accepted by shell-style tokenization as `--target-secret-access-key`, but misses that raw substring check. A format-check invocation with this spelling and an invalid argument printed the synthetic secret in its error log. A quoted credential flag followed by malformed quoting also exposed the synthetic value through the whitespace fallback. Thus batch diagnostics cannot be treated as reliably free of inline credentials.

**3. Parallel fail-fast can dispatch another line after the failure threshold is reached.** Both parallel executors in [`executor.rs`](src/batch_run/executor.rs) check the failure flag before waiting for a worker permit, but do not recheck that flag after the wait. Already-running tasks are intentionally allowed to finish; additionally, the waiting producer can start another task after a failure releases a permit. With eight invalid lines and the default first-error threshold, a local run recorded one failure and seven skips sequentially, but three failures and five skips with `--parallel 2`. A failure threshold is therefore not a strict boundary on subsequent command starts or side effects.

**4. SIGINT does not always produce a non-success batch exit.** Batch exit status is derived from command results; the interrupt flag itself does not force exit 130. A local streaming batch waiting for its first command received SIGINT, then had stdin closed, and exited **0** with no commands recorded. This matters to automation that interprets exit 0 as uninterrupted completion. Transfer frontends have their own cancellation handling, so this finding is not a claim that every interrupted transfer returns success.

**5. Streaming does not bound total queued work or guarantee immediate fail-fast exit.** [`batch_run/mod.rs`](src/batch_run/mod.rs) sends prepared lines through an unbounded channel. The streaming executors drain remaining input after reaching their error threshold, so an input producer that keeps the stream open can delay termination. Read-all mode retains the script in memory; parallel executors also retain completed task results until their spawning loop finishes. The line-length and worker-count limits do not impose a total memory limit.

#### Other limits relevant to reliability

`--check-format` checks parsing and selected batch restrictions, not all execution-time validation. Local checks accepted `put-bucket-versioning s3://b` without a state flag and `delete-bucket not-an-s3-url`, returning “format OK”; execution rejects those inputs later. A successful format check is not authorization to assume a script will execute successfully. Likewise, `put-bucket-policy --dry-run` reads its input but does not parse the policy JSON in the runner.

Several successful or partially successful outcomes require interpretation:

- `cp --skip-existing` returns success when the target exists without comparing its contents. Its existence check and later transfer are separate operations, not an atomic reservation.
- `get-object-annotation` logs a warning but returns success when no applicable integrity check can verify the payload. For `mv`, `--no-fail-on-verify-error` permits deletion despite the warning flag and returns success if deletion succeeds.
- A move's copy and deletion are separate operations. Deletion failure can leave both copies; there is no rollback. Without a specific source version, the runner does not condition deletion on the copied object's identity, so concurrent source changes require consideration.
- Bucket creation followed by tagging is also separate: tagging failure leaves the bucket created and returns warning status 3. Batch continuation options do not reverse earlier operations. Parallel commands have no dependency ordering or per-destination coordination, and their stdout results have no batch-level record framing.

The repository's CI configuration includes formatting, tests, linting, and dependency-policy checks; release builds use `--locked` and produce checksum/provenance artifacts. Those workflow definitions do not prove that a particular release passed those checks. This assessment did not perform a fresh vulnerability-database audit, verify distributed binaries, or establish behavior on every supported platform, S3-compatible service, or configuration.

#### Conclusion: is the software reliable?

**The evidence supports reliability in the scenarios actually tested, but s7cmd is not established as a fail-safe tool for unattended destructive work.** The passing tests and implemented safeguards provide concrete evidence of working behavior. The reproduced redaction, stopping, interruption-status, and self-move-guard problems prevent an unconditional statement that the software is reliable or safe in all supported uses.

For a non-engineer: the program has extensive automated checks, but it can still expose a secret in a log, do more batch work after an error than expected, report success after an interruption, or fail to recognize a move onto the same stored object. It also cannot undo a completed deletion. **The current evidence does not justify relying on it as the sole protection for irreplaceable data.** Recoverable copies, permissions that limit destructive actions, and checking the resulting data remain necessary where a mistake would be costly. Neither the AI origin of the code nor a 98.50% coverage figure supplies a guarantee; no real-world failure rate has been established here.

</details>

### AI assessment of safety and correctness (by Gemini)

<details>
<summary>Click to expand the full assessment</summary>

> Assessment date: 2026-09-26.
>
> Assessed version: s7cmd 1.8.5, source code at commit `443ea60` (branch `fix/annotation-stdout-flush`). Its Rust source code is identical to commits `37d493d`, `8e91eeb`, and `37b6c75`.
>
> Evaluator metadata: LLM Name: Gemini | Model: Gemini 3.8 Flash | Effort: High.
>
> Scope and method of evidence: This evaluation was conducted entirely from scratch for version 1.8.5. All 85 Rust source files under `src/` (17,903 physical lines) were audited in their entirety: the top-level entry point and dispatch (`src/main.rs`, `src/cli.rs`, `src/dispatch.rs`, `src/pipe_safe.rs`), the core `batch-run` execution engine (`src/batch_run/`, 8 files, 4,746 lines), and the four vendored CLI frontends (`src/util_bin/` 59 files, `src/sync_bin/` 6 files, `src/ls_bin/` 3 files, `src/clean_bin/` 5 files). The complete test corpus of 1,496 test annotations was analyzed across 94 test files under `tests/` (25,465 lines; 732 offline tests in 65 files, 258 live-AWS e2e tests in 28 files, and 1 shared test helper) and 506 unit tests in `src/`. Pinned engine dependencies (`s3sync = "=1.62.3"`, `s3util-rs = "=1.10.5"`, `s3rm-rs = "=1.6.4"`, `s3ls-rs = "=1.3.4"`), build configuration (`Cargo.toml`, `Cargo.lock` with 402 locked packages, `build.rs`), supply-chain configuration (`deny.toml`), container definitions (`Dockerfile`), and GitHub Actions workflows (`ci.yml`, `cd.yml`, `cargo-deny.yml`) were thoroughly examined. Verification checks were executed locally: `cargo fmt --all --check` produced 0 diffs; `cargo clippy --all-features --all-targets --locked -- -D warnings` and `RUSTFLAGS="--cfg e2e_test" cargo clippy --all-features --all-targets --locked -- -D warnings` produced 0 warnings; `cargo deny -L error check` confirmed all advisories, bans, licenses, and sources clean; and `RUST_MIN_STACK=16777216 cargo test --all-features --locked` passed all 1,249 offline tests cleanly (506 unit tests and 743 integration tests across 94 test binaries). Latest code coverage artifacts `lcov_report.txt` and `lcov.info` were analyzed record by record.
>
> Limits of evidence: Static source audit and deterministic local test execution. Live-AWS E2E tests (`cfg(e2e_test)`) were examined and statically checked, but live runs require maintainer AWS credentials and were not re-executed against active AWS infrastructure. Coverage measurements indicate code path execution during automated runs, not formal mathematical proof of correctness across all possible network timings, distributed S3 states, or untyped third-party S3 implementations.

#### Scope and Core Architectural Guarantees

s7cmd provides 56 subcommands: 21 read-only subcommands, 34 mutating subcommands, and one batch execution engine (`batch-run`). For 55 subcommands, s7cmd translates command-line arguments and configuration structures to delegate execution to four exact-pinned upstream engine crates. The 56th subcommand, `batch-run`, is s7cmd's own batch execution engine.

A review of the complete production source code in `src/` establishes the following architectural guarantees:

- **Complete Non-Exiting Dispatch Contract**: Production code in `src/` contains zero calls to `std::process::exit`. In the vendored frontends (`src/util_bin/`, `src/sync_bin/`, `src/clean_bin/`, `src/ls_bin/`), upstream process-terminating calls (`load_config_exit_if_err`, state validation exits) were refactored into structured returns of numeric exit statuses (`i32` or `ExitStatus`). In `main()`, `dispatch::dispatch` returns a numeric exit code, which is converted to `std::process::ExitCode` (`src/main.rs:80`). This ensures that an invalid command or runtime error inside one subcommand cannot abruptly terminate the host process, allowing `batch-run` to maintain loop control and execute subsequent commands.
- **Subcommand Future Pinning**: In `src/dispatch.rs:23,41`, large future types generated by underlying engines (`cp`, `mv`, `sync`, `clean`) are heap-allocated via `Box::pin`. This keeps dispatch stack frames bounded and prevents stack overflow during deep async execution on worker threads.
- **Pipe Safety and Output Resilience**: Report generation, object listing, and shell completion scripts route stdout and stderr writes through `pipe_safe` wrappers (`src/pipe_safe.rs`, `src/main.rs:91-103`). When downstream consumers close the pipe early (e.g. `head -n 1`), `ErrorKind::BrokenPipe` is caught and treated as a normal exit 0 rather than a fatal panic. Conversely, on data delivery paths such as downloading annotation payloads to stdout (`src/util_bin/cli/get_object_annotation.rs:362-378`), broken pipes are explicitly propagated as errors (exit 1) and stdout is flushed prior to reporting status, preventing silent truncation of binary data.

#### `batch-run` Engine Architecture and Fault Isolation

The `batch-run` module (`src/batch_run/`) provides original execution logic for serial and parallel script execution with several defensive barriers:

- **Bounded Incremental Line Buffering**: Script lines are read via `read_line_capped` (`src/batch_run/parser.rs:101-135`), which enforces a strict 16 KiB limit (`MAX_LINE_LEN`) incrementally using `BufRead::fill_buf`. Malformed or hostile single-line inputs exceeding 16 KiB abort the line reader without allocating gigabytes of memory.
- **Pre-Execution Argument Validation**: In `src/batch_run/validate.rs`, parsed line structures are validated prior to execution. Validation disallows recursive `batch-run` invocations, stdio dash operands (`-`) on subcommands that require an interactive pipe, and per-line logging/tracing flags (`-v`, `--tracing-log-format`). Validation failures synthesize exit 2 and count toward failure limits without crashing the batch.
- **Panic Boundary Containment**: Command futures in `batch-run` are isolated using `futures::FutureExt::catch_unwind` (`src/batch_run/executor.rs:130`). Any unexpected panic within an individual subcommand is caught, logged with line numbers and redacted command text, assigned synthetic exit code 101, and counted toward error limits. This containment is guaranteed by `panic = "unwind"` across all profiles in `Cargo.toml`.
- **Parallel Concurrency Bounds**: The `--parallel` option is constrained to `0..=1024` by a custom clap parser (`src/cli.rs:162-170`), preventing process-aborting panics in Tokio's semaphore allocation. A Tokio `LocalSet` drives execution on the current thread, accommodating non-`Send` futures across await boundaries.
- **Phased Signal Handling**: `batch-run` installs its process-wide SIGINT handler only after script parsing and validation are complete (`src/batch_run/mod.rs`), preventing signal handlers from interfering with terminal interrupts during initial configuration.

#### Operator Controls, Data Transfer Integrity & Mutation Safeguards

- **Dry-Run Enforcement**: All 34 mutating subcommands implement `--dry-run`; all 21 read-only subcommands reject `--dry-run` (`tests/cli_dry_run.rs`). Mutating wrappers abort execution before invoking S3 mutation APIs, while automatically escalating minimum logging verbosity to `info` level (`src/main.rs:136-200`) so `[dry-run]` preview notices are emitted to the operator.
- **Bulk Deletion Safeguards**: The `clean` subcommand (`src/clean_bin/mod.rs`) mandates either the `--force` flag or interactive confirmation via standard input. In non-interactive environments (no TTY), running without `--force` is rejected immediately with exit 2. In interactive terminals, any response other than `"yes"` aborts deletion cleanly.
- **Four-Gate Transfer-and-Delete Verification in `mv`**: The `mv` command implements a copy-then-delete workflow guarded by a 4-gate verification tree (`src/util_bin/cli/mv.rs:100-162`):
  1. Transfer cancellation status check: aborts if cancellation occurred during copy.
  2. Copy outcome validation: aborts if the copy operation returned an error.
  3. Integrity check: aborts if checksum/ETag verification reported a warning, unless explicitly overridden by `--no-fail-on-verify-error`.
  4. Final cancellation token re-check: aborts if SIGINT was signaled between transfer completion and deletion.
  When all four gates pass, `mv` deletes the exact version ID read or created during the copy step, preventing race conditions against newly written versions.
- **Object Annotation Atomic Verification**:
  - `put-object-annotation` (`src/util_bin/cli/put_object_annotation.rs`) validates the 1-byte..=1-MiB payload limit, computes Content-MD5 and CRC64NVME locally, and verifies the returned CRC64NVME from S3 before reporting success.
  - `get-object-annotation` (`src/util_bin/cli/get_object_annotation.rs`) enforces the 1 MiB limit, streams payload bytes into a neighboring temporary file (`NamedTempFile`), computes on-disk checksums/ETags against the local file, and executes an atomic rename (`tmp.persist`) only after disk verification succeeds. If verification fails, the temporary file is deleted and any existing destination file remains intact.
- **Credential Sanitization**: The CLI parser strips environment variable secrets from `--help` listings across all subcommands (`src/cli.rs:hide_credential_env_values`). In `batch-run`, `redact_secrets` (`src/batch_run/redact.rs`) masks access keys, secret keys, session tokens, and SSE-C encryption keys in logged command lines. Pinned engine crates derive `Zeroize`/`ZeroizeOnDrop` for credential structures and return masked strings in `Debug` implementations.

#### Test Corpus & Coverage Analysis

The test corpus comprises 1,496 test annotations across three categories:
1. **Embedded Unit Tests**: 506 test annotations in `src/` modules (testing argument parsing, dispatch mappings, batch execution, secret redaction, and annotation validation).
2. **Offline CLI & Batch Integration Tests**: 732 test annotations across 65 files under `tests/` (64 `tests/cli_*.rs` files and `tests/batch_run.rs`). These tests execute the compiled `s7cmd` binary against local loopback mock HTTP servers without AWS credentials or network access.
3. **Live-AWS E2E Tests**: 258 test annotations across 28 files (`tests/e2e_*.rs`), gated behind `--cfg e2e_test`, verifying live S3 round-trips against AWS infrastructure using isolated per-test UUID bucket names.

The latest combined coverage artifacts (`lcov_report.txt` and `lcov.info`) record:
- **Executable Lines**: 98.50% (10,713 executed / 10,876 total; 163 missed)
- **Functions**: 97.09% (1,166 executed / 1,201 total; 35 missed)
- **Regions**: 97.54% (14,886 executed / 15,262 total; 376 missed)
- **Branch Coverage**: Not recorded (Rust's LLVM source-based code coverage instrumentation does not measure condition/decision branch coverage)

Analysis of the machine-readable `lcov.info` artifact reveals the distribution of the test suite:
- `lcov.info` contains 10,192 physical line records across 83 source files (the 2 files without records, `sync_bin/mod.rs` and `util_bin/mod.rs`, contain only module declarations).
- Of the 10,192 physical line records, 10,113 were executed (99.22%) and 79 were never executed (0.78%).
- **Separation of Production Code and Test Modules**: 59% of all line records in `lcov.info` (6,026 records) belong to embedded `#[cfg(test)]` modules inside `src/` (with 18 unexecuted records, 99.70% executed). Production code accounts for 4,166 records (with 61 unexecuted records, 98.54% executed). The headline 98.50% figure reflects the combined execution of production logic and internal unit-test assertions.
- **Breakdown of 61 Unexecuted Production Line Records**: Auditing the unexecuted lines across the 21 affected files shows they fall into distinct edge-case categories:
  - OS signal registration failure handlers (10 lines across `ctrl_c_handler.rs` modules).
  - Broken pipe and stream flush error paths (8 lines across `main.rs`, `pipe_safe.rs`, `clean_bin/tracing_init.rs`, `ls_bin/tracing_init.rs`, `util_bin/tracing_init.rs`).
  - Worker panic handling inside Tokio `JoinSet` tasks (8 lines in `src/batch_run/executor.rs:449-452, 613-616`).
  - Single-core CPU detection branch during parallel batch validation (1 line in `src/batch_run/mod.rs:631`).
  - Unreachable validation safeguards, such as checking `mv` for stdio which s3util-rs already rejects at parse time (3 lines in `src/batch_run/validate.rs:29-31`).
  - Interactive clean prompt rejection where user types a response other than "yes" (3 lines in `src/clean_bin/mod.rs:76-78`).
  - Copy exit 3 upon integrity verification warning (1 line in `src/util_bin/cli/cp.rs:42`).
  - Remaining lines correspond to unreachable enum arms, defensive error propagation, and downstream engine error conversions.

#### Confirmed Technical Findings & Operational Limitations

1. **Parallel Executor Spawns Additional Line After Error Limit Reached**: In `run_parallel` and `run_parallel_streaming` (`src/batch_run/executor.rs:406-418`, `551-577`), the execution loop checks `fail_cancel` before calling `sem.clone().acquire_owned().await`, but does not re-check `fail_cancel` after acquiring the permit. If all worker permits are occupied and a failing command releases its permit, the loop acquires that permit, checks only the `interrupt` flag, and dispatches the next queued command. Under parallel execution, `--max-errors N` does not strictly prevent command spawns beyond the Nth error.
2. **Interrupted Batch Run Exits With Status 0**: In `src/batch_run/executor.rs:294-314`, batch exit status is calculated from executed lines using `worse_of`. Commands skipped due to SIGINT are counted as skipped and do not alter the exit code. If SIGINT arrives between commands or during a command that does not install a SIGINT handler and succeeds (such as `head-bucket` or `put-bucket-tagging`), the batch exits 0 despite unfinished commands. Callers checking only `$?` or exit codes in CI pipelines will observe success.
3. **Credential Flag Exposure on Misspelled Options in `batch-run`**: `redact_secrets` (`src/batch_run/redact.rs:29-71`) searches for exact substrings `"access-key"`, `"session-token"`, and `"sse-c-key"`. Misspelled flag names (e.g. `--target-secret-acces-key`, `--target-secret-key`) or bash-escaped syntax (`--target-secret-access-\key`) do not match these substrings and bypass masking. When clap subsequently rejects the invalid argument, `batch-run` logs the entire raw line at error level, exposing the credential value in plain text.
4. **SSE-C Encryption Key Exposure Under Trace-Level SDK Logging**: Combining `--aws-sdk-tracing` and `-vvv` (or setting `RUST_LOG=trace`) configures the AWS SDK's `aws_sigv4` module at trace level (`src/util_bin/tracing_init.rs:55-58`). The SDK's canonical request logging outputs all signed HTTP headers, printing `x-amz-server-side-encryption-customer-key` and its MD5 digest in plain text in terminal output and trace logs.
5. **Indefinite Blocking on Stalled Connections Without Explicit Timeouts**: The underlying AWS SDK client builder configures only a default connect timeout (~3.1 s), without default read or total operation timeouts. If an established TCP connection to an S3 endpoint stops transmitting data, `head-bucket`, `cp`, `ls`, and batch commands wait indefinitely. Furthermore, the SIGINT handler in `cp` and `ls` only acts once; a socket blocked in a synchronous read may fail to terminate on SIGINT, requiring SIGTERM.
6. **Bypass of `mv` Self-Move Protection via Non-Canonical Endpoint Strings**: In `src/util_bin/cli/mv.rs:65-77`, `check_not_self_move` checks endpoint identity via raw string equality (`source_endpoint != target_endpoint`). Syntactically different strings referencing the same endpoint (such as `https://s3.amazonaws.com` versus `https://s3.amazonaws.com/`, or an IP address versus a hostname) bypass the self-move check. On an unversioned bucket, `mv` will copy the object onto itself and then delete the source key, destroying the object.
7. **Hidden Positional Argument `[SHELL]` Accepted by Subcommands**: In `src/cli.rs:471-477`, removing the long name and environment variable for `--auto-complete-shell` on subcommands causes clap to interpret the field as an optional positional argument `[SHELL]`. A trailing shell name (e.g. `rm s3://b/k zsh`) is silently accepted and ignored; any other trailing string causes clap to reject the command with a misleading error referring to `[SHELL]`.
8. **Unbounded Memory in Large Batch Runs**: While individual lines are capped at 16 KiB, sequential `batch-run` buffers all parsed lines in memory, and `--streaming` queues parsed lines into an unbounded channel (`src/batch_run/mod.rs`). Additionally, background Ctrl-C tasks spawned by `cp`, `mv`, `sync`, `clean`, and `ls` lines persist until process termination (~0.9 KiB per line).
9. **Uncapped JSON File Reads in Nine Bucket Configuration Commands**: Nine bucket configuration commands (`put-bucket-policy`, CORS, lifecycle, encryption, website, logging, notification, replication, public-access-block) read input files using unbounded `read_to_string`, unlike annotation payloads which enforce a 1 MiB cap.
10. **`put-bucket-policy --dry-run` Does Not Validate JSON**: `put-bucket-policy` sends policy files verbatim to S3 without local JSON validation (`src/util_bin/cli/put_bucket_policy.rs`), allowing invalid JSON to pass `--dry-run` with exit 0. Other bucket configuration commands parse JSON locally before checking `--dry-run`.
11. **Mode 0600 on Saved Object Annotations**: `get-object-annotation` uses `NamedTempFile`, which creates files with mode 0600 (owner read/write only). Existing destination files replaced by annotations lose their original file permissions.
12. **Permanent Source Version Deletion in `mv`**: On versioned buckets, `mv` deletes the exact source version ID captured during copy (`src/util_bin/cli/mv.rs:129-146`), permanently purging that specific version rather than creating a soft delete marker.

#### Conclusion: Is the Software Reliable?

**Technical Conclusion**: s7cmd is **conditionally reliable** for its documented command-line and batch operations when operated within defined boundaries. The codebase demonstrates disciplined defensive engineering: exhaustive subcommand dispatch, non-exiting batch execution, panic containment via unwind boundaries, four-gate copy-before-delete verification, and dry-run previews across all mutating commands. However, it is **not fail-safe** for unattended, unmonitored destructive operations on irreplaceable data without operational safeguards. The presence of confirmed edge cases—specifically parallel error-limit overrun, exit 0 reporting on interrupted batches, self-move guard bypass via non-canonical endpoints, and indefinite network waits without explicit timeouts—precludes an unqualified certification of reliability.

**Plain-Language Explanation for Non-Engineers**:

To determine whether software is "reliable", we ask three fundamental questions:
1. *Does it do what you told it to do?*
2. *Does it protect your data from accidental loss or corruption?*
3. *Does it tell the truth when something goes wrong?*

Here is how s7cmd answers those questions:

- **What works reliably**:
  - For normal day-to-day operations (uploading, downloading, synchronizing, listing, and moving files), s7cmd performs consistently.
  - Before making changes, all 34 file-modifying commands allow you to run a preview (`--dry-run`) to see what would happen without touching your data.
  - When moving files (`mv`), it copies the file first and confirms the copy succeeded before deleting the original. If the copy fails or the data looks corrupted, it keeps your original file safe.
  - When deleting large numbers of files (`clean`), it refuses to run unless you type `"yes"` or provide an explicit `--force` flag.
  - The software has been tested with nearly 1,500 automated tests covering over 98% of its code.
- **Where it can fail or mislead you**:
  - **Interrupted Batches Can Report Success**: If you stop a batch of commands halfway through by pressing `Ctrl-C`, the program can report exit code `0` (which normally means "everything succeeded"), even though some commands were skipped. If an automated script relies solely on this exit code, it may assume all work was finished when it was not.
  - **Parallel Stopping Delay**: If you run commands in parallel and tell it to stop after an error, it may still start one additional command after reaching the error limit.
  - **Accidental Self-Deletion on Moves**: If you ask it to move a file to the exact same file, it tries to stop you. However, if you type the server address slightly differently for the source and destination (for example, adding a `/` at the end), it fails to recognize that they are the same file, overwrites the file, and then deletes it. On a storage bucket without versioning, the file is permanently lost.
  - **Frozen Network Connections**: If your network stops responding after a connection is opened, the tool will wait indefinitely unless you explicitly specify a timeout.
  - **Leaked Secrets in Error Logs**: If you misspell a password or secret key flag in a batch file, that secret can be printed in error logs.

**Operational Safeguards for Users**:
1. **Enable S3 Bucket Versioning**: Always enable versioning on buckets containing critical or irreplaceable data. This ensures that even an accidental deletion can be undone.
2. **Always Preview Destructive Commands**: Run with `--dry-run` (and `batch-run --check-format` for batch scripts) before running real deletions or bulk moves.
3. **Use Canonical Endpoints**: When using `mv`, ensure source and destination endpoint URLs are spelled identically.
4. **Set Explicit Timeouts**: In automated scripts or CI/CD pipelines, always pass `--operation-timeout-milliseconds` to prevent indefinite hangs.
5. **Inspect Batch Summary Lines**: For automated batch scripts, parse the summary text line (`succeeded`, `failed`, `skipped`) on stderr rather than relying solely on the process exit code.
6. **Pass Credentials via Environment or Profiles**: Configure credentials using AWS configuration profiles or environment variables, never inline in script lines.

</details>

### Scope

s7cmd is designed to cover **Amazon S3 object operations and bucket
management** — listing (`ls`), single- and bulk-object transfers
(`cp` / `mv` / `rm`), atomic server-side rename (`rename`), recursive
synchronization (`sync`), bulk delete (`clean`), archive restoration
(`restore-object`), pre-signed URL generation (`presign`), and the
common bucket-level configurations
(tagging, versioning, policy, policy-status, lifecycle, encryption,
CORS, public-access-block, website, logging, notification,
replication, transfer acceleration, request payment). For any S3 use
case outside that scope, use a more comprehensive tool such as the
[AWS CLI](https://aws.amazon.com/cli/) (`aws s3api`).

s7cmd targets **Amazon S3** as its primary platform and is
optimized for Amazon S3 performance.

> Note that "optimized for Amazon S3 performance" includes the
> allowed request rate: Amazon S3 supports at least 3,500
> PUT/COPY/POST/DELETE or 5,500 GET/HEAD requests per second per
> partitioned prefix, and s7cmd's parallel `ls`, `clean`, and
> `sync` engines assume that capacity. The request rate allowed by
> S3-compatible storage may differ significantly, so running `ls`,
> `clean`, or `sync` against S3-compatible storage can exceed the
> service's rate limit. The request rate can be capped with each
> command's rate-limit option (`--rate-limit-api` for `ls`,
> `--rate-limit-objects` for `clean` and `sync`).

S3-compatible storage (MinIO, Cloudflare R2, Backblaze B2, Wasabi,
Ceph RGW, DigitalOcean Spaces, IBM COS, and similar) is supported on
a **best-effort basis**. Such services are generally usable via
`--endpoint-url` (and `--source-force-path-style` /
`--target-force-path-style` when path-style addressing is required),
but they are not part of the official test matrix, so behavior can
differ between services and change between releases.

This is a structural consequence of building on `aws-sdk-rust`, which
is generated from AWS service models and assumes Amazon S3 semantics
(checksum headers, endpoint resolution, signing variants, response
schemas). Features that depend on AWS-specific semantics may be
unavailable or behave differently against non-AWS endpoints — notably
CRC64NVME checksums and newer S3 API additions, `rename`,
`restore-object`, and the object-annotation subcommands, and parts of
the bucket-configuration family, whose management APIs such services
implement partially or not at all. Core object operations (`ls`,
`cp`, `mv`, `rm`, `sync`, `clean`, `presign`) are the most likely to
work as documented.

Bug reports about S3-compatible storage are welcome and will be looked
at on a best-effort basis, but they are lower priority than Amazon S3
issues, fixes are not guaranteed, and problems that originate in the
storage service itself belong with that service's operator.

s7cmd is **not** intended to be a drop-in replacement for, or
behaviorally compatible with, any other S3 client — including the
AWS CLI (`aws s3`, `aws s3api`) and tools such as `s3cmd`, `s4cmd`,
`s5cmd`, `s6cmd`, and `rclone`. Its command-line flags, transfer semantics,
verification rules, and exit codes are designed around the
underlying libraries' own scope and design principles — not
interoperability with another tool's interface. Output formats and
flag names will not be adjusted to match any external tool, and
scripts written against another S3 client should not be expected to
work with `s7cmd` unmodified. The numeric progression in the name
(`s3cmd` → `s4cmd` → `s5cmd` → `s6cmd` → `s7cmd`) does **not** imply
succession or compatibility.

## Maintenance Model

s7cmd is maintained as a personal project. The project is considered
functionally complete, and development going forward is limited to
maintenance. New features are not actively solicited. If you need
guaranteed enterprise support, this is not the tool for you.

**Dependency update policy**

The AWS SDK for Rust and the other dependencies are updated on a
regular, roughly monthly cadence, and sooner when a security advisory
requires it.

When an update introduces new S3 features, API additions, or client
settings, they are evaluated and adopted as needed — that is, when
they matter for correctness, safety, or the existing feature set. Not
every new SDK capability will be surfaced as a s7cmd option; additions
that fall outside the [Scope](#scope) above are intentionally left out.

Critical bug fixes are applied on a best-effort basis.

## Contributing

- Bug reports are welcome, but responses are not guaranteed.
- Since this project is considered functionally complete, I will not accept any feature requests.
- If you find this project useful, feel free to fork and modify it as you wish.

**Issue and PR lifecycle**

To keep the tracker focused, an issue or PR with no activity for 30 days is labeled `stale` and closed 7 days later unless a new comment (or, for PRs, a new commit) is added. Items labeled `pinned` or `security` are exempt; PRs are also exempt from `pinned`. Closed items can always be reopened.

## License

Apache-2.0
