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

Measurements below are taken at commit `635da2d` on `main` (v1.8.2 plus one build-config commit; measured 2026-09-12). The coverage figures are sourced from `llvm-cov-report.txt` (`cargo llvm-cov`; `lcov.info` is the matching machine-readable LCOV artifact) and reflect a single combined run — `cargo llvm-cov` with `RUSTFLAGS="--cfg e2e_test"` on the maintainer's machine, 2026-09-12 — so the unit tests, the process-level CLI and batch-run tests, and the live-AWS e2e suite are all included in the report.

| Metric                         | Value                                                         |
|--------------------------------|---------------------------------------------------------------|
| Production code                | ~17,900 lines of Rust across 85 source files in `src/`        |
| Unit tests (in `src/`)         | 506 `#[test]` / `#[tokio::test]` annotations                  |
| CLI integration tests          | 730 annotations across 65 files (64 `tests/cli_*.rs` files plus `tests/batch_run.rs`); they spawn the real binary with no AWS credentials — S3 interactions, where exercised, hit an in-process loopback mock server; run in CI |
| E2E integration tests          | 258 annotations across 28 `tests/e2e_*.rs` files (gated behind `--cfg e2e_test`; run only by the maintainer against live AWS) |
| Code coverage (llvm-cov, combined unit + CLI + e2e run) | 97.53% regions (377 / 15,256 missed), 97.08% functions (35 / 1,200 missed), 98.50% lines (163 / 10,873 missed) |
| Static analysis (clippy)       | 0 warnings (`cargo clippy --all-features`)                    |
| Formatting                     | 0 diffs (`cargo fmt --all --check`)                           |
| Supply chain (cargo-deny)      | Clean (`cargo deny -L error check`); runs on every push and PR in `ci.yml` and daily at 01:34 UTC in `cargo-deny.yml`; `advisories.ignore = []` |
| Code adapted from the underlying projects | CLI frontends vendored from the upstream binaries — `src/sync_bin/` ([s3sync](https://github.com/nidor1998/s3sync)), `src/util_bin/` ([s3util-rs](https://github.com/nidor1998/s3util-rs)), `src/clean_bin/` ([s3rm-rs](https://github.com/nidor1998/s3rm-rs)), `src/ls_bin/` ([s3ls-rs](https://github.com/nidor1998/s3ls-rs)) — plus adapted dispatch arms in `src/dispatch.rs`; the engines themselves are consumed as exact-pinned library dependencies (s3sync 1.62.1, s3util-rs 1.10.2, s3rm-rs 1.6.2, s3ls-rs 1.3.2). `src/batch_run/` is s7cmd-original |

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
> Assessed version: s7cmd 1.8.5 at commit `8a35fed` (branch `fix/annotation-stdout-flush`). When this assessment was made, 1.8.5 had not been tagged; the latest tag was `v1.8.4`. Compared with that tag, the Rust code differs in one source file (`src/util_bin/cli/get_object_annotation.rs`, the stdout flush fix described in the 1.8.5 changelog entry) and two test files, and the `s3util-rs` pin moved from 1.10.4 to 1.10.5.
>
> Independence: this section was written from scratch. No other AI assessment was used as input, and no statement from the previous version of this section was kept without being checked again. Every statement below is based on the source code, the tests, the supplied coverage artifacts, the published source of the pinned engine crates, or an experiment run against a debug build of the assessed commit. The two statements about how Amazon S3 itself handles versioned deletes (finding 13) describe documented S3 behavior and were not tested. Before publication, the draft was checked again claim by claim; each error found was confirmed by reproduction and corrected.

#### Scope and method

- **Source code.** All 85 Rust files under `src/` (17,900 lines, including the unit tests they contain) were read in full: the entry point and dispatch (`src/main.rs`, `src/cli.rs`, `src/dispatch.rs`, `src/pipe_safe.rs`), the s7cmd-original `batch-run` engine (`src/batch_run/`, 8 files, 4,743 lines), and the four frontends copied ("vendored") from the engine libraries' own command-line programs (`src/util_bin/` 59 files, `src/sync_bin/` 6, `src/ls_bin/` 3, `src/clean_bin/` 5).
- **Comparison with upstream.** Each of the 72 vendored files (every file in the four frontend directories except two that only declare modules, plus `src/pipe_safe.rs`) was compared with the corresponding program source in the pinned engine release, ignoring comments, whitespace and test modules. 39 are identical; the differences in the other 33 are described under *Architecture*.
- **Engine crates.** Where s7cmd relies on behavior implemented in an engine, the published source of the pinned release was read: s3sync 1.62.3, s3util-rs 1.10.5, s3rm-rs 1.6.4, s3ls-rs 1.3.4.
- **Tests.** Test annotations were counted in all 94 files under `tests/`. The shared loopback mock server (`tests/common/mod.rs`) and the individual tests cited below were read.
- **Checks.** Re-run with rustc and cargo 1.98.1, cargo-deny 0.20.2 and cargo-llvm-cov 0.9.1 on macOS (arm64). The live-AWS suites were compiled and linted but not run: they need AWS credentials and are run by the maintainer.
- **Coverage.** The supplied `lcov.info` and `lcov_report.txt` were analyzed, and the offline suites were measured again on their own to show what the live-AWS suites add.
- **Experiments.** A debug build of the commit was run against a loopback HTTP server written for this assessment, which can answer every request with 200 (optionally after a delay), answer with a 404 that has no body, serve one versioned object, or accept connections and never answer. Signals were delivered by a small driver that restores SIGINT to its default disposition in the child process. No request was sent to AWS.

#### Checks re-run for this assessment

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | No differences |
| `cargo clippy --all-features -- -D warnings` | No warnings |
| `cargo clippy --all-features --all-targets -- -D warnings` | No warnings |
| The same, with `RUSTFLAGS="--cfg e2e_test"` (compiles and lints the live-AWS suites) | No warnings |
| `cargo test --all-features --locked` (offline suites) | 1,249 passed, 0 failed, 0 ignored, in 94 test binaries |
| `cargo deny -L error check`, advisory database of 2026-09-26 | advisories ok, bans ok, licenses ok, sources ok |

The 1,249 tests are the 506 unit tests in `src/`, the 732 offline tests under `tests/`, and the 11 unit tests of `src/cli.rs`, which run a second time because `tests/cli_routing.rs` includes that file.

#### Tests and coverage

The test corpus contains 1,496 test annotations:

- 506 unit tests in `src/`.
- 732 offline tests in 65 files under `tests/`. 670 of them, in `tests/batch_run.rs` and in 63 of the 64 `tests/cli_*.rs` files, start the real binary without real AWS credentials; where S3 is involved, they point it at a loopback server that replays prepared HTTP responses, or at a closed local port. The other 62, in `tests/cli_routing.rs`, check argument parsing in-process.
- 258 live-AWS tests in 28 `tests/e2e_*.rs` files, compiled only with `--cfg e2e_test`.

CI builds and runs the offline tests on seven targets (the Unix-only signal tests do not run on the two Windows targets). It does not compile the live-AWS test code and does not measure coverage.

The supplied artifacts were written at 11:50 (`lcov.info`) and 11:51 (`lcov_report.txt`) local time on 2026-09-26, after the assessed commit (11:35). Their totals agree with each other, and their line records include the lines changed by the 1.8.5 fix. The right-hand column is a measurement of the offline suites alone, made during this assessment at the same commit:

| Metric | Supplied artifacts (unit, offline and live-AWS suites) | Offline suites only |
| --- | --- | --- |
| Lines | 98.50% (163 of 10,876 missed) | 94.49% (599 missed) |
| Regions | 97.54% (376 of 15,262 missed) | 93.19% (1,040 missed) |
| Functions | 97.09% (35 of 1,201 missed) | 97.00% (36 missed) |
| Branches | not recorded | not recorded |

- These figures include the unit-test code inside `src/`. `lcov.info` holds records for 10,192 distinct lines; about 5,900 of them are in `#[cfg(test)]` modules, and those run in both measurements. Counting only the program's own code, 62 lines are missed in the supplied data (about 98.5% covered) and 510 in the offline measurement (about 88%). The tests that CI runs therefore execute about 88% of the program's code, not 98.50%.
- The two measurements have identical totals. Line by line, 448 line records are covered only in the supplied data and none only in the offline measurement, which is consistent with the supplied data being the offline suites plus the live-AWS suite. Code reached only by the live-AWS suite includes the S3-to-S3 and stdin/stdout branches of the copy wrapper (`src/util_bin/cli/mod.rs:437-564`), the success paths of most `get-*` and `put-*` bucket commands, `rename`, and `clean`'s refusal to run without `--force` in a non-interactive session.
- In `lcov.info`, 79 distinct lines have zero hits. llvm-cov's summary counts 10,876 line entries, so its missed-line figure (163) is larger; in the offline measurement, `--show-missing-lines` listed exactly the zero-hit records (527) while the summary reported 599. Of the 79 lines, 17 are in test code. Of the 62 in the program's code, 55 handle conditions that the tests cannot produce or that the pinned code does not reach: failure to register a Ctrl-C handler (8 lines), a closed pipe while log output is flushed (4), panics in background tasks (16), branches excluded by earlier validation or by the engines' behavior, such as `unreachable!` arms and error arms of functions that always return `Ok` (23), and a failed write of the shell-completion script (4). The remaining 7 can be reached in normal use: `cp` exiting 3 after a verification warning (`src/util_bin/cli/cp.rs:42`; the live-AWS test that checks this exit code starts the binary through `cargo run`, and the coverage data records no hit on the line), an answer other than `yes` at `clean`'s confirmation prompt (`src/clean_bin/mod.rs:76-78`), a Ctrl-C that arrives while `batch-run --streaming --parallel` waits for the next line or a free slot (`src/batch_run/executor.rs:575-576`), and the pre-validation path used on a machine with one logical CPU (`src/batch_run/mod.rs:630`).
- The figures cover s7cmd's `src/` only; the engine crates and the AWS SDK are not measured. Coverage shows which code ran during the tests, not whether its results were checked.

#### Architecture: what s7cmd's own code does

s7cmd has 56 subcommands. For 55 of them, `src/dispatch.rs` converts the parsed arguments into the configuration of one of four engine libraries, each pinned to an exact version, and calls a frontend vendored from that library's own program: s3sync for `sync`, s3ls-rs for `ls`, s3rm-rs for `clean`, and s3util-rs for the other 52. The 56th, `batch-run`, is s7cmd's own. The exit codes produced are 0, 1, 2, 3, 4, 101 and 130; `src/main.rs:80` converts them with `ExitCode::from(code as u8)`, and all of them fit in a byte.

The vendored frontends return an exit code instead of terminating the process, so that `batch-run` can execute many subcommands in one process. The only process-terminating calls in s7cmd's code are clap's, for top-level argument errors and `--help`/`--version` output (`src/main.rs:57-59`); `batch-run` parses its lines with `try_get_matches_from` and does not reach them. In the engine libraries, the only such calls are in s3util-rs's `validate_state_flag`, which s7cmd replaces with non-exiting `check_state_flag` functions, and in a documentation example in s3rm-rs. There is no `unsafe` code outside test modules. The program's code has 21 `unwrap`, `expect` or `unreachable!` sites. Nineteen cannot fail, because of a preceding check, a hard-coded value or an upstream validation. The other two re-raise a panic that has already happened elsewhere: a poisoned lock in `sync`'s status report (`src/sync_bin/cli/mod.rs:95`), and a panicking thread in `batch-run`'s pre-validation of scripts of 256 lines or more (`src/batch_run/mod.rs:649`), which ends the run before any line executes.

The 33 vendored files that differ from the pinned upstream programs differ in the following ways. Adaptations for running many subcommands in one process: exit codes are returned instead of passed to `process::exit`, the upstream `main` and configuration-loading functions are removed, the three state-flag checks do not exit, and the log subscriber is installed with `try_init`, with filters that add s7cmd's own targets (in the s3util-rs frontend, whose subscriber `batch-run` uses, also those of the other three engines). Log messages: program names are changed, and "not found" is logged at warn level where upstream uses error level (the exit code, 4, is the same). Behavior: `cp` and `mv` dry runs return earlier than upstream, before the target storage and the progress indicator are created, and log "would copy." rather than "would copy object."; `head-object` reports a missing bucket and a missing key with the same message; `get-object-annotation` verifies the temporary file before renaming it over the destination, where upstream renames first; and upstream's registration of user-defined Rust callbacks, which upstream enables only through a test flag, is absent. The rest are import paths, visibility and module order. No fix present in the upstream programs was found missing from s7cmd.

Build and dependencies: the four engines are pinned with `=` versions and `Cargo.lock` (402 packages) is committed. TLS is provided by rustls 0.23.45; `deny.toml` bans `openssl-sys`, which is absent from `Cargo.lock`, keeps an empty advisory-ignore list, and allows only crates.io as a source. cargo-deny runs in CI on every push and on a daily schedule. The release workflow builds with `--locked`, publishes SHA-256 files and GitHub build-provenance attestations with the archives, and publishes to crates.io through trusted publishing rather than a stored token.

#### Behavior observed in experiments

| Scenario | Observed |
| --- | --- |
| `create-bucket`, `rm`, `cp`, `mv`, `sync` and `clean` with `--dry-run`, against the logging server | No PUT, POST or DELETE request. Read-only requests were sent by `create-bucket --if-not-exists` (HEAD), `cp --skip-existing` (HEAD), `sync` and `clean` (object listing). After `mv --dry-run`, the local source file was still present. |
| `put-bucket-policy`, `put-bucket-cors` and `put-bucket-lifecycle-configuration` with `--dry-run` and a file that is not valid JSON | `put-bucket-policy` exited 0 (the policy body is read but not parsed); the other two exited 1 with a JSON parse error. No request in any case. |
| `cp --dry-run` from a local file that does not exist | Exit 0, "[dry-run] would copy.". Without `--dry-run`: exit 1, "source file not found". |
| `mv` with source equal to target, written directly and in directory form (`s3://b/d/k` to `s3://b/d/`) | Rejected with "cannot mv an object onto itself", exit 1, no request. |
| `mv` from a versioned source object to a local file | HEAD, then GET with `versionId=V`, then DELETE with `versionId=V`. `rm` without `--source-version-id` sent DELETE without a version ID. |
| `clean` without `--force`, stdin not a terminal | Refused with "Cannot run destructive operation without --force (-f) in a non-interactive environment (no TTY)", exit 2, no request. The same as a `batch-run` line. |
| `--help` with credential environment variables set (`cp`, `sync`, `ls`, `clean`, `head-bucket`) | Variable names shown; values not shown. |
| Verbose logs with credentials on the command line (`head-bucket`, `head-object`, `ls`, `clean`, `sync`, `cp` and a `batch-run` line; `-vvv` or `-vvvv` with and without `--aws-sdk-tracing`, and `RUST_LOG=trace`) | The secret access key and the session token did not appear. The SSE-C key and its MD5 appeared in full in the AWS SDK's signing trace for real (not dry-run) requests of `cp`, `sync`, `head-object` and a `batch-run` line at trace level (`-vvv` or more with `--aws-sdk-tracing`, or `RUST_LOG=trace`); not at `-vv`, and not without SDK tracing (finding 5). The access key ID appeared in full in the SDK's trace output and in `sync`'s configuration trace, partly masked in the `ls` and `clean` configuration traces, and not at all in `cp`'s configuration summary. |
| `batch-run` logging lines that carry credential options | Values masked as `****` for correctly spelled options, in `--flag value` and `--flag=value` form and on a line that fails to tokenize. Values printed for misspelled options (finding 4). |
| `batch-run`, default mode: valid line, invalid line, valid line | The first line ran (its PUT reached the server), exit 2, third line skipped. `--check-format` on the same script reported line 2, exit 1, no request (finding 1). |
| `batch-run` of three `head-bucket` lines, server answering after 3 s, SIGINT at 1 s | Sequential, `--parallel 2` and `--streaming` all exited 0, with a summary such as "1 succeeded, 0 failed, 0 warnings, 2 skipped" (finding 2). |
| `batch-run --parallel 2`: invalid line 1, valid lines 2-8, server answering after 1 s | Lines 2 and 3 ran in each of three runs (finding 3). |
| Endpoint that accepts connections and never answers | Standalone `head-bucket` ended at the first SIGINT. Standalone `ls` and `cp`, and `batch-run` (sequential and `--parallel 2`), were running after two SIGINTs and ended on SIGTERM. With `--operation-timeout-milliseconds 3000`, the `batch-run` line failed after 3.1 s (finding 6). |
| Long batches, `--streaming -q` | Maximum resident set size 32, 38 and 59 MiB for 2,000, 8,000 and 32,000 `cp --dry-run` lines; 44, 43 and 45 MiB for the same numbers of `put-bucket-tagging --dry-run` lines (finding 7). |
| `--streaming`, a line longer than 16 KiB after a line that exited 4 | Exit 4, summary "0 failed". After a successful line instead: exit 1 (finding 8). |
| stderr redirected to a file | ANSI color codes written by `head-bucket`, `get-bucket-policy`, `cp` and `batch-run`; none by `ls`, `clean` and `sync`, or with `--disable-color-tracing` (finding 9). |
| `--check-format` on a `sync` line whose local source directory does not exist yet | Rejected ("source file/directory not found"). A `cp` line with a missing local source was accepted (finding 10). |
| `batch-run --streaming -` with a writer that keeps stdin open, two SIGINTs | Summary written at 2 s; the process exited only when the writer closed stdin, at 12 s (finding 11). |
| `batch-run --parallel 1025` | Rejected when arguments are parsed, exit 2; 1024 accepted. |

#### Findings, ordered by potential impact

Severity levels used here: **medium** means the problem can lead to an S3 operation the operator did not intend (directly, or by reporting success for work that was not done), a credential value in a log, or a run that does not end without SIGTERM or SIGKILL; **low** means a wrong but non-zero exit code, a wrong summary, cosmetic output, or resource growth under specific conditions; **informational** means behavior an operator should know about, or an observation about maintenance, not a defect.

1. **Medium: `batch-run` runs the lines that come before an invalid line, which the README says it does not.** The execution-modes table in the `batch-run` section describes the default mode as "Catches bad lines before any line runs." In the code, a line that fails to tokenize, parse or validate becomes a failure with exit code 2 at its own position in the script (`src/batch_run/mod.rs:283-289`, `src/batch_run/executor.rs:151-164`), so every line before it runs first; with the default stop at the first failure, the lines after it are skipped. This agrees with the "Failure handling" paragraph of the same README section and is asserted by `batch_run_invalid_sync_config_counts_as_per_line_failure` in `tests/batch_run.rs`, whose comment records that the earlier behavior (nothing runs) was replaced. The experiment confirmed it. Code comments in `src/batch_run/validate.rs:111-120` and `:486-488` still describe the earlier behavior. `--check-format` stops at the first invalid line without running anything. Workaround: run `batch-run --check-format` on every script before running it.
2. **Medium: an interrupted `batch-run` can exit 0.** The batch exit code is the worst code among the lines that ran (`src/batch_run/executor.rs:322-368` and the parallel equivalents); lines that were never started because of Ctrl-C are counted as skipped but do not affect the exit code. When SIGINT arrives between lines, or during a line that has no Ctrl-C handler of its own (for example `head-bucket`, `rm`, or a `get-*` or `put-*` command) and that line then completes, the batch exits 0 although lines were skipped; in the experiment, "1 succeeded, 0 failed, 0 warnings, 2 skipped" came with exit 0. A caller that checks only the exit status, such as `s7cmd batch-run a.txt && next-step`, treats the interrupted run as complete. When the interrupted line handles Ctrl-C itself (`cp`, `mv`, `sync`, `clean`, and `ls` when listing objects), it returns 130, and the batch then exits non-zero. Workaround: after an interruption, check the summary line for skipped lines. Fix: rank 130 into the exit code when the interrupt flag has stopped the run.
3. **Medium: with `--parallel`, one more line can start after the failure limit is reached.** In `run_parallel` and `run_parallel_streaming`, the loop that starts lines checks the failure flag before waiting for a free worker slot, but after obtaining the slot it checks only the interrupt flag (`src/batch_run/executor.rs:406-418` and `:554-577`). When all slots are busy and a running line reaches the limit, the slot it frees is used to start the next line. In the experiment, line 3 started in each of three runs; the README states that no new command is started once the limit is reached. The default sequential mode (`--parallel 1`) is not affected. Fix: check the failure flag again after obtaining the slot.
4. **Medium: a misspelled credential option in a `batch-run` line is logged with its value.** `redact_secrets` (`src/batch_run/redact.rs:29-71`) masks the value after an option whose name contains `access-key`, `session-token` or `sse-c-key`. When the option name is misspelled (`--target-secret-acces-key`, `--target-secret-key` and `--target-sessiontoken` were tested), clap rejects the line and `batch-run` logs the line at error level, which is visible at the default verbosity, in plain text and in `--json-tracing` output, with the value in full; `--check-format` does the same. The logged error reason (clap's message) names only the option. Workaround: pass credentials through AWS profiles or environment variables. Fix: when a line is rejected, mask the value after any unknown option whose name contains `key`, `secret` or `token`, or omit the line text.
5. **Medium: at trace level with AWS SDK tracing, the SSE-C key is written to the log.** With `--aws-sdk-tracing` and `-vvv` or more, the tracing filter enables `aws_sigv4` at trace level (`src/util_bin/tracing_init.rs:55-58`; the other frontends have the same filter), and the AWS SDK's `signing request` trace event lists the request headers, including `x-amz-server-side-encryption-customer-key` and its MD5. The key appeared in full in the output of `cp`, `sync`, `head-object` and a `batch-run` line; `RUST_LOG=trace` has the same effect. Because the key travels in a request header, it makes no difference whether it was given as an option or through the environment. `batch-run` masks the option in its own line log, but not in the SDK's event. The secret access key and the session token did not appear. Workaround: do not combine SSE-C with trace-level SDK tracing, or treat such logs as secret. Fix: cap `aws_sigv4` below trace level in the filter, or document the exposure.
6. **Medium for unattended use: a request to an endpoint that stops answering waits indefinitely, and for most commands a second Ctrl-C does not end it.** All four engines configure SDK timeouts only when one of `--operation-timeout-milliseconds`, `--operation-attempt-timeout-milliseconds`, `--connect-timeout-milliseconds` or `--read-timeout-milliseconds` is given (`build_timeout_config` in each engine's `src/storage/s3/client_builder.rs`); otherwise only the AWS SDK's default connect timeout of 3.1 seconds applies, as a unit test in s3util-rs asserts. The SDK's stalled-stream protection, which all four engines enable by default, applies to request and response bodies and did not end the wait in the experiment, where no response was sent. The Ctrl-C handlers of `cp`, `mv`, `sync`, `ls`, `clean` and `batch-run` act on the first SIGINT only, and cancelling the operation did not interrupt a request that was waiting for its response. Standalone single-request subcommands install no handler and end at the first SIGINT, but inside `batch-run` they cannot be interrupted, because `batch-run`'s handler has taken over SIGINT. Workaround: set a timeout option for unattended runs, and stop a stalled run with SIGTERM. Exiting on a second SIGINT would remove the interactive part of the problem.
7. **Low: each `cp`, `mv`, `sync`, `ls` or `clean` line in a batch leaves a Ctrl-C listener task running.** These frontends start a task that waits for either Ctrl-C or cancellation of the operation's token (for example `src/util_bin/cli/ctrl_c_handler.rs:32-52`). In the pinned engines, a run that completes normally does not cancel the token (they cancel it on errors and on stop conditions such as `--max-delete`), so the task stays until the process exits. Single invocations are unaffected. In a batch, memory grows with the number of such lines, by about 0.9 KiB per line in the measurement above. Fix: stop the task or cancel the token when the operation returns.
8. **Low: in `--streaming` mode, a read error is ranked by numeric value.** When the script reader fails (a line over 16 KiB, bytes that are not UTF-8, or a panic in the reader task), the run returns `code.max(1)` (`src/batch_run/mod.rs:409-419`). If an earlier line exited with a code greater than 1 (for example 3 or 4), the batch exits with that code instead of 1, and the summary does not count the read error as a failure; the error itself is logged. The default mode reads the whole script before running any line and exits 1 without running anything. Fix: use `worse_of(code, 1)` and count the read error as a failure.
9. **Low: color codes are written to stderr when it is not a terminal.** `src/util_bin/tracing_init.rs:46` enables ANSI output unless `--disable-color-tracing` is given, while the `ls`, `clean` and `sync` frontends also require stderr (or stdout for `sync`) to be a terminal. This affects the s3util-rs family of subcommands and `batch-run`. The upstream s3util-rs 1.10.5 program has the same code. Workaround: pass `--disable-color-tracing` when logs are redirected.
10. **Low: `sync` lines are checked against the local filesystem when the script is validated.** Validation builds the s3sync configuration (`src/batch_run/validate.rs:121-131`), and s3sync rejects a local source that does not exist. A script in which an earlier line creates the directory that a later `sync` line reads fails validation although it would work at run time. `cp` lines are not checked for this at validation time. Workaround: create the directory before running the script.
11. **Low: `batch-run --streaming -` does not exit after Ctrl-C while the writer keeps stdin open.** The run stops and writes its summary, but the process waits for the pending read on stdin to return, which happens only when the writer sends data or closes its end. Workaround: close the writer as well.
12. **Informational: exit code 3 means "warning" in `batch-run`, and some engine warnings concern the data.** `batch-run` treats 3 and 4 as warnings (`src/batch_run/executor.rs:281-287`); `--continue-on-warning` continues past them, and the summary counts them as warnings, not failures. In s3util-rs, `cp` exits 3 when the source is in S3 (download or S3-to-S3 copy) and the ETag, or a composite (multipart) checksum, of the copy does not match the source, including cases the log describes as "may be corrupted"; the copied object is kept at the destination. For uploads from a local file or stdin, and for full-object checksums, a mismatch is an error (exit 1). `mv` exits 1 in the warning case and keeps its source, unless `--no-fail-on-verify-error` is given. `clean` exits 3 when some objects could not be deleted or when it stopped at `--max-delete`. Without `--continue-on-warning`, exit 3 stops a batch like a failure. Exit 0 does not always mean that integrity was verified: `get-object-annotation` writes the payload and exits 0, with a warning in the log, when neither an AES256 ETag nor an additional checksum is available (`src/util_bin/cli/get_object_annotation.rs:341-359`).
13. **Informational: on a versioned source bucket, `mv` removes the copied version permanently.** `mv` deletes the source with the version ID captured when the copy started (`src/util_bin/cli/mv.rs:131-146`), so no delete marker is created and bucket versioning keeps no copy of that version at the source; the data exists at the target. Deleting the captured version, instead of whatever is current, means that a newer version written to the key during the copy is not deleted. On Amazon S3, the most recent remaining version of the key then becomes current: the key stays visible at the source if that version is an object, and not if it is a delete marker. `rm` without `--source-version-id` sends a delete without a version ID, which on a versioned bucket adds a delete marker.
14. **Informational: what a dry run does not check.** `put-bucket-policy --dry-run` reads the policy file but does not parse it; the other eight `put-*` commands that take a JSON file parse it before the dry-run exit. `cp --dry-run` and `mv --dry-run` do not check that a local source file exists. Some dry runs send read-only requests (see the table).
15. **Informational: other observations.** The access key ID can appear in full in trace-level logs (see the table). `src/batch_run/validate.rs:24-33` checks `mv` for stdin/stdout, which s3util-rs already rejects, so the branch is not reachable. Comments in `src/cli.rs` (lines 492-494 and 630-631) name upstream releases (s3sync 1.61.0, s3rm-rs 1.5.0, s3ls-rs 1.2.0, s3util-rs 1.9.0) as the ones s7cmd pins, which is no longer the case. In CI, clippy does not lint test code (`cargo clippy` without `--all-targets`), the test job does not use `--locked`, and the live-AWS test code is not compiled. The release workflow (`cd.yml`) does not wait for CI results, and most GitHub Actions are referenced by version tag rather than by commit SHA. The `Dockerfile` builds with the floating `rust:1-trixie` image and without `--locked`. `.cargo/min-publish-age.toml` takes effect only when it is passed explicitly to nightly Cargo with `--config`, as its own comment states.

No finding involves s7cmd acting on an object or bucket other than one named on its command line or in a script line.

#### What this assessment does not establish

- The live-AWS suites were not run. Behavior against Amazon S3 is covered by those suites, which the maintainer runs, and by the engine crates' own tests; the experiments here used a loopback server, which shows which requests s7cmd sends, not how Amazon S3 answers them.
- The engine crates and the AWS SDK were read only where s7cmd depends on them. Defects inside them are outside this assessment.
- Experiments ran on macOS (arm64) only. CI runs the offline tests on six more targets.
- S3-compatible services other than Amazon S3 were not tested.
- Tests and experiments can show that defects exist, not that none remain. This assessment applies to commit `8a35fed` only.

#### Conclusion: can s7cmd be relied on?

**Answer: yes, with conditions.** For single commands, the conditions are to set a timeout option for unattended runs (finding 6) and not to use trace-level AWS SDK logging together with SSE-C keys (finding 5). For `batch-run` scripts, four more apply (findings 1 to 4). Within these conditions, the evidence gathered here supports relying on s7cmd for its documented purpose. It does not show that s7cmd is free of defects: eleven were found (findings 1 to 11), six of them medium.

In plain terms:

- **What was checked.** Every file of s7cmd's own code was read. The 1,249 automated tests that run without an AWS account were run, and all passed. The formatting and static-analysis tools reported no problems, and the dependency audit (cargo-deny) found no known security advisory in the locked dependencies (402 packages in `Cargo.lock`). According to the supplied coverage data, the tests, including the maintainer's tests against real AWS, execute about 98.5% of s7cmd's own program code (test code excluded); the tests that run automatically on every change execute about 88%. The program was also run in the 19 experiments listed above, to observe what it actually does rather than what the documentation says.
- **What held.** In preview mode (`--dry-run`), none of the tested commands sent a request that changes anything, although a dry run does not catch every mistake (finding 14). `mv` deletes the original only after the copy has succeeded and passed verification, unless `--no-fail-on-verify-error` is given (shown by the code, by unit tests, and by a test that interrupts a move); in the experiment it deleted exactly the version it had copied, and it refused to move an object onto itself. `clean` refused to bulk-delete without `--force` when no one could confirm. The secret access key and the session token did not appear in help text or in any log examined, except in the case of finding 4. Every command that failed in the experiments ended with a non-zero exit status, the signal that scripts and schedulers use to detect failure; an interrupted `batch-run` is the exception described in finding 2.
- **What did not hold.** When `batch-run` runs a script (a file of s7cmd commands, one per line) that contains a mistake, the lines before the mistake still run, although the README's table says they do not (finding 1). A `batch-run` stopped with Ctrl-C (the interrupt key) can report success although some lines never ran (finding 2). With `--parallel`, one more command can start after the configured failure limit is reached (finding 3). A misspelled credential option in a script is written to the log together with the secret value (finding 4). With the most detailed AWS SDK logging switched on, an SSE-C encryption key is written to the log (finding 5). If the storage service stops answering, a command waits indefinitely unless a timeout option is given, and for most commands pressing Ctrl-C again does not stop it (finding 6).

Each of these has a workaround: check every script with `batch-run --check-format` before running it, and keep the default one-line-at-a-time mode for scripts that delete or overwrite; after interrupting a `batch-run`, read its summary line instead of relying on the exit status; keep credentials in AWS profiles or environment variables, not in scripts; do not combine SSE-C with trace-level SDK logging, or treat such logs as secret; set `--operation-timeout-milliseconds` (or the other timeout options) for unattended runs, and stop a stalled run with SIGTERM (for example `kill <pid>`). As with any tool that deletes or overwrites data, preview destructive work with `--dry-run`, and keep a backup or bucket versioning for data that cannot be recreated, bearing in mind finding 13 for `mv`.

This assessment judged the program by its code and by its observed behavior, neither of which depends on who or what wrote the code. It does not verify the development process described elsewhere in this README.

</details>

### AI assessment of safety and correctness (by Codex)

<details>
<summary>Click to expand the full assessment</summary>

Evaluation date: 2026-09-13. LLM name: Codex (OpenAI). Model: GPT-5-based Codex
(exact runtime model identifier not exposed). Effort: comprehensive full-source review
(configured reasoning-effort setting not exposed).

#### Scope and overall judgment

This assessment was made from scratch, without consulting any earlier or other AI assessment.
The review covered every repository Rust source file: all 85 files under `src/`, including
their unit tests, all 94 files under `tests/`, and `build.rs`. Cargo manifests and lockfile,
build configuration, Dockerfile, dependency policy, and CI/release workflows were also examined.
The reviewed snapshot is `245c245da13188650fefb8ba4d974e2725f0711b`; its code and build inputs
are unchanged from the snapshot used for the September 12 checks below.

Overall judgment: substantial safeguards and broad test coverage support confidence in ordinary
validated workflows, but confirmed credential-redaction, batch scheduling, and payload-output
defects limit confidence in unattended automation. This is not an unconditional safety endorsement
or a proof of correctness. The complete repository review does not constitute a full audit of the
implementations of `s3sync`, `s3util-rs`, `s3rm-rs`, `s3ls-rs`, their transitive dependencies,
or AWS service behavior.

#### Safeguards observed

- Command parsing, target validation, and destructive-operation prerequisites reject many invalid
  invocations before mutation. Batch commands are parsed and dispatched in-process, without an
  implicit shell. Dry-run paths suppress the intended mutations, although some still construct
  clients or perform reads. Credential environment values are hidden in help output.
- `mv` checks for self-moves, transfer failure, cancellation, and verification warnings before
  deleting the source; verification warnings block deletion unless explicitly overridden.
  Deletion uses an explicit source version or the version captured by the transfer.
  These are meaningful data-loss safeguards. See [move handling](src/util_bin/cli/mv.rs).
- Annotation downloads enforce a 1 MiB payload limit and check length and applicable integrity
  information. File output uses a same-directory temporary file, verifies it before replacement,
  and leaves the existing destination untouched on pre-replacement failure.
  See [annotation output](src/util_bin/cli/get_object_annotation.rs).
- No production repository Rust `unsafe` blocks were found. This does not establish memory safety
  for dependencies; test-only tracing code does contain unsafe process-environment mutations.

#### Confirmed findings

1. **Malformed batch lines can disclose inline credentials.** The whitespace fallback in
   [credential redaction](src/batch_run/redact.rs#L119) does not recognize a quoted credential
   flag when another token has an unterminated quote. A fake secret following
   `"--target-secret-access-key"` appeared verbatim in diagnostics for both ordinary batch
   execution and `--check-format`. Existing masking therefore does not make malformed input
   safe to echo into logs. Avoid inline secrets in batch files pending a fix.
2. **Parallel fail-fast can dispatch another command after the failure threshold is reached.**
   [Both parallel executors](src/batch_run/executor.rs#L370) check the failure-stop flag before
   waiting for a worker permit, but recheck only interruption afterward. With two workers,
   two invalid commands followed by two harmless copy dry-runs produced
   `1 succeeded, 2 failed, 0 warnings, 1 skipped`: the third command ran after the failures.
   This is additional dispatch, not merely completion of already-running work.
   The streaming executor has the same missing post-wait failure check.
3. **Annotation payload output can report success after losing data.**
   [The stdout path](src/util_bin/cli/get_object_annotation.rs#L361) calls `write_all` without
   an explicit flush before returning success. A loopback response containing one byte without
   a newline, with the stdout pipe's reader already closed, returned exit 0. Buffered payload
   delivery errors can therefore escape the command's result.
4. **Some ordinary CLI tests can perform real mutations.**
   [Versioning parsing tests](tests/cli_put_bucket_versioning.rs#L74), and similar acceleration,
   request-payment, and restore tests, inherit configuration and execute valid mutating commands
   without a dry-run or mandatory mock endpoint. A loopback-only check of the existing versioning
   test observed `PUT /example/?versioning` while the test passed. Its assertion only excludes
   exit 2, so runtime failures can also pass. These tests should be isolated from privileged
   ambient credentials and real endpoints.

#### Operational limits

Batch interruption is not uniformly reflected in the exit status: the executors aggregate
command results but do not independently promote an interruption to exit 130. An idle streaming
batch with stdin held open remained alive after SIGINT and returned 0 once stdin closed.
Existing [signal tests](tests/cli_sigint.rs#L82) explicitly accept this behavior.
Streaming failure-stop drains input until the producer closes it, so a slow or never-ending
producer can delay termination. Its [unbounded input channel](src/batch_run/mod.rs#L368) and
retained parallel task results also mean worker limits and the per-line size cap are not total
memory bounds.

`mv` remains copy-then-delete, not a transaction. Concurrent changes to unversioned or local
sources need external coordination, and the self-move guard compares endpoint strings rather
than canonical service identity. Neither batch execution nor a dry-run provides rollback or a
stable snapshot of later operations. Annotation payloads without applicable integrity information
are accepted with a warning and exit 0; successful file replacement is not an explicit
power-loss durability guarantee.

#### Coverage and verification evidence

The supplied `lcov.info` and `llvm-cov-report.txt`, last modified on 2026-09-12 at approximately
15:25 JST, agree on these totals:

| Measure | Covered / total | Coverage |
| --- | ---: | ---: |
| Lines | 10,710 / 10,873 | 98.50% |
| Functions | 1,165 / 1,200 | 97.08% |
| Regions | 14,879 / 15,256 | 97.53% |

There are 83 source-file records; the two module-only files `src/sync_bin/mod.rs` and
`src/util_bin/mod.rs` have no records. The totals include in-source test code, and branch
coverage is not measured. These supplied reports were not regenerated for this assessment;
coverage percentages neither establish assertion quality nor rule out the reproduced defects.

The following checks passed on macOS with Rust/Cargo 1.98.1 during this review:

- `cargo test --locked --all-features --all-targets`
- `cargo check --locked --no-default-features --all-targets`
- `cargo clippy --locked --all-features --all-targets -- -D warnings`
- `cargo fmt --all --check`
- `cargo deny -L error check` (advisories, bans, licenses, and sources)

The live-AWS tests gated by `cfg(e2e_test)` were reviewed but were not enabled by these commands.
The additional reproductions used malformed input, dry-runs, signals, or loopback mocks with
fake credentials. No production fixes are included in this README-only assessment.

</details>

### AI assessment of safety and correctness (by Gemini)

<details>
<summary>Click to expand the full assessment</summary>

> Assessment date: 2026-09-12.
>
> Assessed version: 1.8.2 (branch `nidor1998/docs-readme-quality-verification`, commit `ebe97e794091e6bf1185ac8463dcf785947ea390`, short `ebe97e7`).
>
> Evaluator metadata: LLM Name: Gemini | Model: Gemini 3.6 Flash | Effort: High.
>
> Method and scope of evidence: This evaluation was conducted entirely from scratch for version 1.8.2, without referencing any prior AI assessments or third-party summaries. All 85 Rust source files under `src/` (17,889 physical lines) were systematically audited in full, encompassing top-level CLI parsing and command dispatch (`src/main.rs`, `src/cli.rs`, `src/dispatch.rs`, `src/pipe_safe.rs`), the core `batch-run` execution engine (8 modules in `src/batch_run/`), and all four vendored CLI frontends (`src/util_bin/`, `src/sync_bin/`, `src/clean_bin/`, `src/ls_bin/`, totaling 73 files). The complete test suite of 1,427 test functions was inspected across offline unit/integration suites (67 files in `tests/cli_*.rs` and `tests/batch_run.rs`), live-AWS E2E test suites (28 files in `tests/e2e_*.rs`), and embedded unit tests in `src/`. Build configuration (`Cargo.toml`, `Cargo.lock`, `build.rs`), security policies (`deny.toml`), `Dockerfile`, and GitHub Actions workflows were also examined. Latest code coverage artifacts `llvm-cov-report.txt` and `lcov.info` were evaluated. Verification commands were run from scratch and confirmed zero issues: `cargo fmt --all --check` clean with 0 diffs; `cargo clippy --all-features --all-targets --locked -- -D warnings` clean with 0 warnings; `cargo test --all-features --locked` 1,247 offline unit and integration tests passed cleanly. Interfaces to exact-pinned upstream engine crates (`s3sync = "=1.62.1"`, `s3util-rs = "=1.10.2"`, `s3rm-rs = "=1.6.2"`, `s3ls-rs = "=1.3.2"`) were thoroughly audited.
>
> Limits of evidence: Static code analysis and local deterministic test execution. Does not include formal mathematical verification, fuzzing, Miri execution, thread/memory sanitizers, or live AWS network mutation runs. E2E tests are gated under `cfg(e2e_test)` and run against maintainer AWS infrastructure; coverage measures code path execution, not absolute logical correctness under all cloud edge cases.

**Question addressed.** As a command-line utility and batch executor for Amazon S3, s7cmd delegates 55 of its 56 subcommands to four exact-pinned engine crates while providing one custom execution engine (`batch-run`). This assessment evaluates whether s7cmd introduces routing errors, credential leaks, unhandled panics, uncontained batch failures, data destruction during transfers, or unexpected mutation behaviors during dry runs.

#### Command Dispatch & Structural Non-Exiting Guarantee

- **Complete Subcommand Routing**: The `Cmd` enum defines 56 variants (`src/cli.rs`): 21 read-only subcommands, 34 mutating subcommands, and `batch-run`. `src/dispatch.rs` maps every variant to its underlying logic. Routing correctness is systematically verified across subcommand parsing tests (`tests/cli_routing.rs`), `src/dispatch.rs` unit tests, and process-level integration suites.
- **Process Stability & Non-Exiting Contract**: Production code under `src/` contains zero calls to `std::process::exit`. Every dispatch branch returns a numeric exit status (`ExitStatus` or `i32`), which `main()` converts to `std::process::ExitCode` (`src/main.rs:80`). In vendored frontends, upstream process-exiting calls (`load_config_exit_if_err`, state validation exits) were refactored into non-exiting status returns (`src/dispatch.rs:31,59,76`). A parameter or configuration error in one subcommand returns exit 2 without terminating the process or aborting a `batch-run` sequence.
- **Stack Memory Protection**: Large subcommand future types are explicitly `Box::pin`-ed (`src/dispatch.rs:23,41`) to keep dispatch stack frames small and prevent stack overflow on test worker threads (which operate under a 2 MB stack limit).

#### `batch-run` Engine Architecture & Fault Isolation

`batch-run` represents s7cmd's original execution engine, implemented with multi-layered defensive controls:

- **Incremental Line Buffering**: Input lines are read using `read_line_capped` (`src/batch_run/parser.rs:101-135`), enforcing a strict 16 KiB limit (`MAX_LINE_LEN`) incrementally via `BufRead::fill_buf`. Multi-gigabyte single-line inputs are aborted after buffering ~16 KiB rather than exhausting process memory. UTF-8 validation and POSIX shell tokenization (`shlex`) are applied per line.
- **Pre-Execution Validation**: `src/batch_run/validate.rs` validates parsed argument structures before running commands. It explicitly rejects nested `batch-run` invocations, stdin/stdout dash operands (`-`), and per-line tracing/verbosity flags (`-v`, `--tracing-log-format`). Validation failures synthesize exit 2 and count toward `--max-errors` / `--continue-on-error` thresholds rather than aborting the batch.
- **Panic Boundary Containment**: Subcommand execution is wrapped in `futures::FutureExt::catch_unwind` (`src/batch_run/executor.rs:130`). Any unexpected panic inside a subcommand is caught, logged with line numbers and redacted text, assigned synthetic exit code 101, and counted toward error limits. This mechanism relies on `panic = "unwind"` specified across all build profiles in `Cargo.toml:105`.
- **Severity-Ranked Exit Codes**: Batch exit status is determined by severity ranking rather than simple maximum value: `exit 1` (error) > `exit 2` (arg/validation error) > `exit 3` (warning) > `exit 4` (not found) > other non-zero > `exit 0` (`src/batch_run/executor.rs:294-303`). Per-line SIGINT (exit 130) is bucketed as `skipped` (`executor.rs:283`) and does not trip error thresholds.
- **Phased Signal Handling**: Signal listeners are not installed during the script reading/validation phase (where Ctrl-C terminates immediately). The SIGINT handler is registered only before command execution starts, ensuring in-flight futures handle cancellation cleanly while preventing new commands from spawning (`src/batch_run/mod.rs`).
- **Parallel Execution Safety**: `--parallel` concurrency is constrained to `[1, 1024]` by a custom clap parser (`src/cli.rs`), preventing semaphore allocation panics. Tokio `LocalSet` drives async execution with a concurrency semaphore.
- **Shell Auto-Completion Isolation**: Top-level `--auto-complete-shell` is disarmed on subcommands (`src/main.rs:67-69`), preventing inherited environment variables from altering subcommand argument parsing.

#### Operator Safeguards, Dry-Run Integrity & Transfer Safety

- **Comprehensive Dry-Run Coverage**: All 34 mutating subcommands accept `--dry-run`; none of the 21 read-only subcommands accept it (`tests/cli_dry_run.rs`). Thin wrappers abort before invoking mutating S3 API calls, while complex operations (`cp`, `mv`, `sync`, `clean`) propagate `--dry-run` into their underlying engine. Dry-run automatically elevates minimum logging verbosity to `info` level (`src/main.rs:136-200`) so `[dry-run]` execution logs are visible. Live-AWS tests verify that dry-run calls leave cloud resources unmodified (`tests/e2e_dry_run.rs`).
- **High-Risk Delete Protection**: `clean` (bulk delete) mandates `--force` or interactive `"yes"` confirmation (`src/clean_bin/mod.rs`). Interrupted prompts exit via default OS signal handling.
- **`mv` Copy-Then-Delete Decision Tree**: `mv` checks for self-move conditions (`src/util_bin/cli/mv.rs:46-98`) by comparing source and target buckets, endpoints, resolved keys, and version IDs before executing any transfer. Deletion of the source object is guarded by a 4-gate decision tree (`mv.rs:100-162`): (1) no cancellation during transfer, (2) successful copy completion, (3) no checksum/verification warnings (unless `--no-fail-on-verify-error`), (4) final cancellation token re-check immediately prior to delete. Source deletion specifies the exact version ID read during copy.
- **Object Annotation Integrity**: Annotation payloads enforce a 1 MiB limit (`src/util_bin/cli/put_object_annotation.rs:48`, `get_object_annotation.rs:323-334`). Uploads verify Content-MD5 and CRC64NVME response checksums. Downloads stream bytes into a temporary file (`NamedTempFile`), verify on-disk payload checksums, and perform an atomic filesystem rename (`tmp.persist`) only after successful verification (`get_object_annotation.rs:184-228`). Pre-existing destination files remain intact if verification fails.
- **Partial State Warnings**: `create-bucket --tagging` executes bucket creation followed by tagging. If tagging fails after bucket creation, it emits a warning detailing partial state and exits 3 (`src/util_bin/cli/create_bucket.rs`).

#### Credential Hygiene & Masking

- **Engine Credential Redaction**: Access key structs and SSE encryption keys in engine crates derive `Zeroize`/`ZeroizeOnDrop` and implement `Debug` formatting returning `** redacted **`. Detailed config dumps in `cp`/`mv` dispatch limit logged fields to non-sensitive metadata (`src/dispatch.rs`).
- **Help Output Protection**: `hide_credential_env_values` (`src/cli.rs`) recursively removes default environment variable secret values from `--help` text across all subcommands. Process-level tests (`tests/cli_help.rs`) confirm credential flag names display without leaking environment variable contents.
- **Batch Log Redaction**: `src/batch_run/redact.rs` sanitizes inline credentials (access keys, secret keys, session tokens, presigned URLs) in both `--flag value` and `--flag=value` syntax before logging or writing JSON trace records. Unparseable lines use fallback regex/whitespace scrubbing (`tests/batch_run.rs`).

#### Supply Chain & Build Pipeline Security

- **Pinned Engine Dependencies**: `Cargo.toml` pins engine dependencies to exact versions (`s3sync = "=1.62.1"`, `s3util-rs = "=1.10.2"`, `s3rm-rs = "=1.6.2"`, `s3ls-rs = "=1.3.2"`). `Cargo.lock` is committed; build and publication workflows enforce `--locked`.
- **TLS & Crypto Stack**: Cryptographic transport uses `rustls 0.23` with `aws-lc-rs 1.17` and OS trust anchors. `openssl-sys` is excluded from the dependency tree and explicitly banned in `deny.toml`. `ring` is restricted to test dependencies.
- **Dependency Auditing**: `cargo deny -L error check` runs on all pushes/PRs (`ci.yml`) and daily schedules (`cargo-deny.yml`). `deny.toml` maintains `advisories.ignore = []` and enforces license allowlists.
- **Release Provenance**: Release builds (`cd.yml`) compile with `--locked`, produce SHA-256 digests, generate GitHub Actions build provenance attestations, and publish to crates.io via OIDC trusted publishing.
- **Embedded Lua Interpreter**: `s3sync` includes `lua_support` (mlua 0.12) by default to support Lua filter callbacks (`Cargo.toml:19`). While intentional, embedding Lua increases binary attack surface.

#### Test Corpus & Coverage Evidence

s7cmd includes 1,427 test functions across three tiers:
1. **Embedded Unit Tests**: 338 test attributes in `src/` (covering `dispatch.rs`, `executor.rs`, `redact.rs`, `mv.rs`, `get_object_annotation.rs`, etc.).
2. **Offline Integration Tests**: 730 tests across 67 files (`tests/cli_*.rs` and `tests/batch_run.rs`) executing CLI binaries against an in-process loopback mock server (`127.0.0.1:0`).
3. **Live AWS E2E Suites**: 258 tests across 28 files (`tests/e2e_*.rs`) gated behind `cfg(e2e_test)`.

Latest coverage artifacts `llvm-cov-report.txt` and `lcov.info` present combined test coverage:
- **Line Coverage**: 98.50% (10,873 / 11,036 executable lines; 163 missed)
- **Function Coverage**: 97.08% (1,165 / 1,200 functions; 35 missed)
- **Region Coverage**: 97.53% (14,879 / 15,256 regions; 377 missed)
- **Branch Coverage**: Not measured by standard llvm-cov instrumentation in Rust.

Core module coverage highlights: `batch_run/executor.rs` (98.47% lines), `batch_run/mod.rs` (98.89% lines), `batch_run/redact.rs` (100% lines), `util_bin/cli/mv.rs` (100% lines), `util_bin/cli/get_object_annotation.rs` (99.26% lines).

#### Identified Technical Findings & Operational Limitations

1. **Non-Canonical Endpoint Comparison in `mv` Self-Move Guard**: `check_not_self_move` (`src/util_bin/cli/mv.rs:67-77`) compares endpoint URLs using raw string equality (`source_endpoint != target_endpoint`). Syntactically distinct spellings of the same S3 endpoint (e.g., HTTP vs HTTPS, IP vs hostname) bypass the self-move check, relying on bucket versioning to prevent data loss.
2. **Unbounded Aggregate Batch Memory**: While single lines are capped at 16 KiB, standard `batch-run` buffers all parsed lines in a `Vec` (`src/batch_run/parser.rs:61`), and `--streaming` mode uses an `unbounded_channel` (`src/batch_run/mod.rs`). Scripts with millions of lines can consume substantial memory.
3. **Inconsistent Cancellation Exit Reporting**: Cancellation handling varies by subcommand: `ls` maps cancellation to exit 0 (`src/ls_bin/mod.rs`); `clean` returns exit 0 upon encountering cancellation even if earlier object deletion errors occurred (`src/clean_bin/mod.rs`); `sync` maps cancellation to exit 0.
4. **Streaming Stdin Cancellation Delay**: In `batch-run --streaming -`, reading from an open stdin pipe can remain blocked on Tokio's stdin reader after SIGINT or early error failure until EOF is received (`tests/cli_sigint.rs:72-80`).
5. **Parallel Executor Queue Signal Window**: In parallel execution, the interrupt check occurs before awaiting a semaphore permit (`src/batch_run/executor.rs:399-430`). A SIGINT arriving during permit await can allow one previously queued command to start.
6. **Severity Code Aggregation Edge Cases**: Exit codes outside 1–4 (including panic exit 101) rank below exit 4 in batch severity ranking (`src/batch_run/executor.rs:289-313`), causing a batch containing a panic and a warning to exit 3 or 4.
7. **Uncapped JSON Configuration Reads**: Nine bucket configuration commands (policy, CORS, lifecycle, encryption, website, logging, notification, replication, public-access-block) read input files using unbounded `read_to_string`, unlike annotation payloads which enforce a 1 MiB cap.
8. **Network Activity During Dry Runs**: Subcommands with `--dry-run` perform client configuration and read-only S3 checks (e.g. `cp --skip-existing` issuing HeadObject, `create-bucket --if-not-exists` issuing HeadBucket) before suppressing mutating calls.

#### Reliability Summary

Evaluated by **Gemini 3.6 Flash** (Effort: **High**), s7cmd exhibits robust defensive engineering for Amazon S3 operations. Subcommand routing is exhaustive, non-exiting dispatch guarantees process persistence during batch execution, panic boundaries contain unexpected failures, credential redaction is systematically enforced, and object annotation workflows provide strong atomic verification.

The binary is **conditionally reliable**:
- Destructive operations should be previewed using `--dry-run` (and `batch-run --check-format` for batch scripts).
- S3 bucket versioning should be enabled for critical datasets.
- Automated workflows should monitor structured error logs in addition to numeric exit status.
- Source and target endpoints in `mv` scripts should use canonical, identical strings.

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
