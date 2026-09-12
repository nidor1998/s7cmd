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
| (default) | Read the whole script first, validate every line, then execute. Catches bad lines before any line runs. Shows a progress bar when stderr is a TTY. |
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

Discussions about the legitimacy, licensing, or ethics of AI-generated code in general are out of scope for this issue tracker. Issues opened on those grounds — without a concrete, reproducible defect in s7cmd's behavior — will be closed.

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

> Assessment date: 2026-09-12.
>
> Assessor: Claude Fable 5.1 by Anthropic (model ID `claude-fable-5-1`), running at Effort: max.
>
> Assessed version: 1.8.2 (branch `nidor1998/docs-readme-quality-verification`, commit `407c749`). The commit is four commits past tag `v1.8.2` (`1a7c8aa`); the difference touches only `.cargo/min-publish-age.toml`, the issue templates, and this README, so the Rust sources, `Cargo.toml`, and `Cargo.lock` are identical to the tagged release.
>
> Independence: this assessment was produced from scratch at the commit above. The other AI assessments in this README were not read and were not used as input, and nothing was carried forward from any earlier assessment. Every statement below traces to the source, the tests, the coverage artifacts, the pinned engine crates, or an experiment run against a binary built from this commit during the assessment.

#### Method and scope of evidence

- **Source.** All 85 Rust files under `src/` (17,889 lines) were read in full and directly, with no sampling and no delegated partial passes: `src/main.rs`, `src/cli.rs`, `src/dispatch.rs`, `src/pipe_safe.rs`, the s7cmd-original `src/batch_run/` engine (eight files), and the four vendored frontends `src/util_bin/` (59 files), `src/sync_bin/`, `src/ls_bin/`, and `src/clean_bin/`.
- **Tests.** All test files were read: `tests/common/mod.rs` (the loopback mock S3 server), `tests/batch_run.rs`, the 64 offline `tests/cli_*.rs` files, and the 28 live-AWS `tests/e2e_*.rs` files gated behind `cfg(e2e_test)`.
- **Build, supply chain, CI.** `Cargo.toml`, `build.rs`, `deny.toml`, `.cargo/config.toml`, `.cargo/min-publish-age.toml`, `Dockerfile`, `SECURITY.md`, `CLAUDE.md`, and all five GitHub Actions workflows.
- **Coverage artifacts.** `lcov.info` and `llvm-cov-report.txt` were used as supplied. Their totals agree with each other (10,873 lines, 1,200 functions), they were generated at 15:25 local time on 2026-09-12, and the only commit after that time changed this README, so they describe the assessed sources.
- **Engine crates.** Where a guarantee lives in an engine rather than in s7cmd, the pinned published sources were consulted directly: s3sync 1.62.1, s3util-rs 1.10.2, s3rm-rs 1.6.2, s3ls-rs 1.3.2 (credential `Debug` redaction, the `clean` safety prompt, tracing initialisation, and how network timeouts are configured).
- **Checks re-run during the assessment** (rustc and cargo 1.98.1, cargo-deny 0.19.4, cargo-llvm-cov 0.8.7): `cargo fmt --all --check` clean; `cargo clippy --all-features -- -D warnings` clean; `cargo clippy --all-features --all-targets -- -D warnings` under `RUSTFLAGS="--cfg e2e_test"` clean, so the live-AWS corpus compiles and lints; `cargo test` (offline suites) 1,247 passed, 0 failed, 0 ignored; `cargo deny -L error check` clean against the advisory database of the assessment date. The e2e suites were not executed (they require live AWS credentials and are reserved for the maintainer).
- **Experiments.** A debug binary built from the commit was driven against local loopback servers (one that accepts TCP connections and never answers, one that answers every request with a bodiless 404) and hand-written batch scripts to test the claims that reading alone cannot settle: credential masking with misspelled flags, Ctrl-C behaviour with and without cancellation handlers, streaming-mode exit codes, colour escapes on non-terminal stderr, `clean` without `--force` under a non-interactive stdin, and `head-bucket` against a bodiless 404.

#### Architecture: what the wrapper does and does not do

s7cmd exposes 56 subcommands. Fifty-five hand off to one of four exact-pinned engine libraries (s3sync for `sync`, s3ls-rs for `ls`, s3rm-rs for `clean`, s3util-rs for everything else), and one, `batch-run`, is s7cmd's own. `src/cli.rs` declares the clap surface, `src/dispatch.rs` translates each parsed subcommand into the engine's configuration and calls a vendored frontend, and `src/main.rs` converts the returned code with `ExitCode::from`. Every code in use (0, 1, 2, 3, 4, 101, 130) fits in a byte, so nothing is truncated.

The design decision that shapes the safety story is that the vendored frontends never call `process::exit`. Each file carries a provenance header naming the upstream binary version it was taken from and enumerating the adjustments (returning the exit code, a process-global "Ctrl-C received" flag, a shared signal-test lock). The consequence is that one process can execute many subcommands in sequence, which is what makes `batch-run` possible, and also what makes panic containment, secret masking, and a single tracing subscriber necessary.

#### `batch-run`, the s7cmd-original engine

- **Parsing.** Lines are tokenised with `shlex` (shell-style quoting), so values containing spaces or shell metacharacters can be quoted per line. Blank lines and `#` comments are skipped. A 16 KiB per-line cap is enforced incrementally through `BufRead::fill_buf`, so an oversized or endless line cannot grow memory without bound.
- **Two intake modes.** In the default read-all mode the whole script is read, parsed, and validated before any line executes; a script with a syntax error or a forbidden line runs nothing. In `--streaming` mode a reader task feeds lines through a channel as they arrive; an invalid line then synthesises exit 2 and is counted against the stop policy like any other failure.
- **Validation rules** (also available without execution through `--check-format`): a nested `batch-run` is rejected; stdin/stdout transfers (`-` as a `cp`/`mv` endpoint) are rejected because the script owns stdin; reading a configuration file from `-` is rejected; per-line tracing flags are rejected because the process has one subscriber; and engine-level configuration errors (for example an invalid `sync` argument combination) are reported with the line number.
- **Execution.** Sequential by default; `--parallel N` (capped at 1,024) runs lines on a `LocalSet` with a semaphore. Each line runs inside `catch_unwind` (`panic = "unwind"` is set for every profile in `Cargo.toml`), so a panic inside an engine becomes a per-line exit 101 with an error-level log entry carrying the line number and the masked line text, not a dead batch.
- **Stop policy and exit code.** The default stops at the first failure; `--max-errors N` allows N failures; `--continue-on-error` never stops; `--continue-on-warning` lets exit 3 (warning) and exit 4 (not found) pass. Lines that return 130 (a Ctrl-C'd `sync`/`ls`/`clean`/`cp`/`mv`) are bucketed as skipped and never trip a threshold. The batch exit code is the worst code seen, ranked 1 > 2 > 3 > 4 > any other non-zero > 0 (`severity_rank` and `worse_of` in `src/batch_run/executor.rs`), so a run mixing exit 1 and exit 130 exits 1. The ranking is unit-tested at its edges.
- **Ctrl-C.** `batch-run` installs its own tokio `ctrl_c` handler that sets an interrupt flag; no further line starts after it, and the summary counts the rest as skipped. Only lines whose subcommand has a cancellation handler are actually interrupted (see finding 2 for what this means for single-request lines).
- **Logging.** One tracing subscriber is installed for the process, with a filter covering s7cmd and all four engine crates; `--json-tracing` switches to structured output; the summary line reports succeeded/failed/warnings/skipped counts; every place that logs a script line passes it through `redact_secrets` first.

#### Wrapper fidelity and protection against operator mistakes

- **Dry run.** Every mutating subcommand accepts `--dry-run`. Configuration-file inputs (policies, lifecycle, CORS, and similar JSON) are read and parsed before the dry-run short-circuit, so a dry run also validates the input document. The offline dry-run suite (67 tests) exercises the mutating subcommands under `--dry-run` against the loopback server.
- **`mv`.** The frontend refuses a move whose source and destination resolve to the same object (the comparison is endpoint- and version-id-aware, and the literal `null` version id is excluded), and it runs the delete phase only after a copy phase that finished with no error, no warning, no cancellation, and not under `--dry-run`.
- **`clean`.** The s3rm-rs safety checker requires `--force` or an interactive `yes`. Verified: with a non-interactive stdin and no `--force`, `clean` refuses with exit 2 and the message "Cannot run destructive operation without --force (-f) in a non-interactive environment (no TTY)", both standalone and as a `batch-run` line. `--dry-run` skips the prompt because nothing is deleted.
- **Annotations.** `get-object-annotation` downloads into a temporary file, verifies it with a bounded 1 MiB read, and only then renames it into place; `put-object-annotation` sends Content-MD5 and CRC64NVME so a corrupted upload is rejected by the service.
- **Pipes and terminals.** Output goes through pipe-safe writers; the closed-stderr, broken-pipe, and tracing-to-stderr suites show that a consumer that exits early produces no panic and no abort. Help output hides credential values that would otherwise be echoed from environment variables.
- **Bodiless errors.** `head-bucket` against a 404 with no body reports exit 4 (not found), confirmed experimentally, so the not-found class does not depend on an XML error body.

#### Credential handling

Three layers were verified. First, `--help` never prints the value of a credential environment variable. Second, the configuration types of all four pinned engine crates redact access keys, session tokens, and SSE-C keys in their `Debug` output, and s7cmd's own configuration trace logs a hand-picked summary rather than the whole configuration. Third, `batch-run` masks the value of any flag whose name contains `access-key`, `session-token`, or `sse-c-key` (both `--flag=value` and `--flag value` forms, quoting-aware, with a whitespace fallback when a line is not shell-parseable) before the line text reaches any log. The masking is keyed on the flag name, which is the root of finding 1.

Credentials passed as command-line flags are visible to other processes on the same host through the process argument list; this is inherent to any CLI and is one more reason to prefer profiles and environment variables, which s7cmd supports everywhere.

#### Supply chain, build, and CI

- The four engine crates are pinned with `=` versions and `Cargo.lock` is committed, so a build of a tag resolves the same graph the tests ran against. `rust-version` is declared (1.94.1), edition 2024.
- `deny.toml` keeps an empty advisory ignore list, allows a fixed license set, bans `openssl-sys` (the TLS stack is rustls), and restricts sources to crates.io. `cargo deny` runs in CI on every push and on a daily schedule, so a newly published advisory surfaces without a code change.
- CI runs rustfmt, clippy with `-D warnings`, and the offline test suites on a seven-target matrix (Linux glibc and musl on x86_64 and aarch64, Windows on x86_64 and aarch64, macOS on aarch64). A separate workflow uploads clippy results as SARIF to code scanning.
- The release workflow builds per-target archives, publishes SHA-256 sums, attaches GitHub build-provenance attestations (`actions/attest-build-provenance`), and publishes to crates.io through trusted publishing rather than a stored token.
- `.cargo/min-publish-age.toml` opts dependency updates into a seven-day minimum publish age on nightly, a mitigation against freshly published malicious crate versions; on stable it is inert by design.
- The `Dockerfile` builds in `rust:1-trixie` and runs as an unprivileged user on `debian:trixie-slim` with only `ca-certificates` added. The builder tag floats across Rust minor versions, so container builds are not reproducible and may compile with a newer toolchain than CI tested (informational).
- CI does not compile the `cfg(e2e_test)` corpus; a broken e2e test would be noticed only on the maintainer's machine. It compiled cleanly during this assessment.

#### Test corpus and coverage

The counts in the Quality verification table were recounted independently and match: 506 test annotations in `src/` (unit tests), 730 in the 65 offline process-level files (`tests/cli_*.rs` and `tests/batch_run.rs`), and 258 in the 28 `tests/e2e_*.rs` files. The offline corpus spawns the real binary against a loopback mock S3 or purpose-built fake servers and asserts on exit codes, stdout, and stderr; it pins subcommand routing (62 tests), help text (71), argument validation, dry-run behaviour (67), error paths, pipe safety, SIGINT handling and interruption exit codes (a fake S3 that stalls mid-transfer, 17 tests), and the `batch-run` semantics above (68). The e2e corpus covers object, bucket-configuration, annotation, stdio, cancellation, exit-code, and `batch-run` paths against live AWS.

Coverage as reported by the artifacts:

| Metric | Covered | Missed |
| --- | --- | --- |
| Regions | 97.53% | 377 of 15,256 |
| Functions | 97.08% | 35 of 1,200 |
| Lines | 98.50% | 163 of 10,873 |

The Quality verification section states that the figures include the live-AWS suites; this assessment confirmed that the two artifacts agree with each other and correspond to the assessed commit, but it could not attribute individual hits to the e2e run rather than to the offline suites. The 163 uncovered lines were examined one by one. They are: error arms for a failed Ctrl-C handler registration or a failed tracing-subscriber flush in each of the four frontends; `unreachable!` and `let ... else { panic!() }` arms, most of them inside test helpers; the "cancelled, but not by Ctrl-C" arms in `ls` and `clean`, which the binary cannot reach because only the Ctrl-C handler cancels the token; `clean`'s interactive "no" answer and its indicator-task-panicked arm; the `batch-run` arms for a task that panics outside `catch_unwind` and for a panicking reader task; the completion-script write failure in `main.rs`; the `mv` stdio validation branch that s3util-rs rejects earlier (dead code); and `cp`'s warning return (`ExitStatus::Warning`), which is exercised by an e2e test through a `cargo run` subprocess that coverage instrumentation does not see. None of these is a primary operational path; the measured figures therefore understate real coverage slightly rather than overstate it. The usual caveats apply: coverage counts executed lines, not verified behaviour, and the engine crates' internals are outside the measurement.

#### Findings, ordered by potential impact

1. **Secret masking in `batch-run` depends on the flag being spelled correctly (medium).** `redact_secrets` (`src/batch_run/redact.rs`) masks the token after a flag whose name contains one of three substrings. A misspelled credential flag, for example `--target-secret-acces-key SECRET`, `--target-secret-key SECRET`, or `--target-sessiontoken SECRET`, is rejected by clap, and `batch-run` then logs the rejected line at error level (and in `--json-tracing` output) with the value unmasked. With the correct spelling the value is masked even when the line fails for an unrelated reason (verified with an extra unknown flag). Because a typo is exactly the situation in which the line is logged, the masking is weakest where it is most needed. Suggested fix: when clap rejects a line, log clap's message without the raw line, or mask the value of every unknown flag whose name contains `key`, `secret`, or `token`. Until then, supply credentials through profiles or environment variables in scripts.
2. **No hard stop on Ctrl-C, and no network timeout unless requested (medium for unattended use).** The four vendored handlers and `batch-run` use tokio's `signal::ctrl_c`, which replaces the default SIGINT disposition for the rest of the process and fires once: it sets the flag and cancels the token, and a second Ctrl-C does nothing. Cancellation is honoured at the engines' own check points (between operations or body chunks), not inside a request that is still waiting for the server to answer, and the engines build an SDK `TimeoutConfig` only when one of `--operation-timeout-milliseconds`, `--operation-attempt-timeout-milliseconds`, `--connect-timeout-milliseconds`, or `--read-timeout-milliseconds` is given. Observed: a standalone `ls` against an endpoint that accepts the connection and never answers was still running six seconds after one and after two SIGINTs; inside `batch-run`, two such `head-bucket` lines survived two SIGINTs, and the run ended (1 failed, 1 skipped, exit 1) only when the fake endpoint was shut down; with `--operation-timeout-milliseconds 3000` the same line failed after three seconds. Standalone single-request subcommands, which install no handler, terminate on the first Ctrl-C as documented, but the same line inside `batch-run` cannot be interrupted at all because the batch's handler owns SIGINT. Consequence: an unattended batch against a stalled endpoint hangs until SIGTERM or SIGKILL. Mitigation: set the timeout flags in automation; a second-Ctrl-C hard exit (130) would remove the operator-facing half of the problem.
3. **Streaming-mode reader failures are ranked numerically, not by severity (low).** In `src/batch_run/mod.rs`, when the streaming reader fails (an over-long or non-UTF-8 line, or a panicking reader task) the run returns `code.max(1)`. If an earlier line exited 3 or 4, the numeric maximum keeps that code, the summary reports 0 failed, and the reader error is visible only in the error log; with an earlier success the same script exits 1. Read-all mode is unaffected because it rejects the script before any line runs. Suggested fix: use `worse_of(code, 1)` and count the reader error as a failure.
4. **Colour escapes reach non-terminal stderr for the s3util-rs family and `batch-run` (low).** `src/util_bin/tracing_init.rs` enables ANSI output with `!disable_color_tracing` alone, whereas the `ls`, `clean`, and `sync` frontends also require `stderr().is_terminal()`. Logs redirected to a file from `cp`, `mv`, `head-*`, `get-*`, `put-*`, and `batch-run` contain escape sequences unless `--disable-color-tracing` is passed. This matches the upstream s3util-rs 1.10.2 binary, so it is an inherited trait rather than a vendoring error.
5. **Validation of `sync` lines checks the local filesystem at validation time (low).** Both `--check-format` and the pre-execution validation build the engine configuration, and s3sync rejects a local source that does not exist yet. A script whose earlier line creates the directory that a later `sync` reads from fails validation even though it would run; `cp` lines are only checked at execution time. Workaround: create the directory before running the script.
6. **`batch-run --streaming -` cannot be interrupted while blocked on an open stdin (low).** The reader blocks in a read that only returns when the producer writes or closes; a Ctrl-C between lines is noticed only when the next line arrives. The behaviour is documented in `tests/cli_sigint.rs`. Operators cancelling a streaming batch must also close the producer.
7. **Vendoring drift (informational).** The frontends are snapshots of earlier upstream binaries with individually ported fixes (`sync_bin` from s3sync 1.57.1/1.58.6, `ls_bin` from s3ls-rs 0.4.1 plus 1.3.0 ports, `clean_bin` from s3rm-rs 1.3.3/1.3.4 plus 1.6.0 ports, `util_bin` spanning s3util-rs 0.2.0 to 1.10.0) while the engine libraries are 1.62.1, 1.3.2, 1.6.2, and 1.10.2. Nothing wrong was found in the current snapshots, and the offline corpus pins the observable behaviour, but a flag or behaviour that upstream adds to its binary after the snapshot version does not appear in s7cmd automatically. The provenance headers make an audit tractable.
8. **Minor observations (informational).** The `mv` stdio branch in `src/batch_run/validate.rs` is unreachable because the s3util-rs parser rejects `-` for `mv` first; the floating `rust:1-trixie` builder tag in the `Dockerfile`; and the absence of an e2e compile check in CI (see above).

No finding identifies a path that mutates or deletes the wrong object, sends a mutating request under `--dry-run`, weakens an engine guarantee, or loses a per-line failure in the default read-all mode. Findings 1 and 2 are the ones that change operating practice; the rest mis-report or mis-format rather than mis-act.

#### Verdict: is s7cmd reliable enough for unattended and destructive use?

Yes, within stated conditions. The wrapper is thin and faithful: routing, configuration translation, and exit-code propagation are tested per subcommand against a mock server, the engines' own safeguards (dry run, `mv` delete gating, the `clean` confirmation, checksum verification) pass through intact, and the one piece of original logic, `batch-run`, is designed around containment: nothing runs before validation in the default mode, a panic costs one line, secrets are masked on every log path, the stop policy is explicit, and the exit code is severity-ranked and tested. The static gates (rustfmt, clippy at `-D warnings`, cargo-deny with no ignored advisories, exact pins, provenance attestations) are stronger than most CLI tools of this size carry, and the coverage figures are backed by a corpus that asserts on behaviour, with the small uncovered remainder accounted for above. That the code was produced with AI assistance is not, by itself, evidence in either direction; the evidence is the design, the tests, and the checks re-run here.

The conditions are the ones this assessment found: preview destructive work with `--dry-run` and scripts with `batch-run --check-format`; keep credentials in profiles or environment variables rather than inline flags, which also closes finding 1; set `--operation-timeout-milliseconds` (or the finer-grained timeout flags) on every line of an unattended batch; pass `--disable-color-tracing` when stderr is a file; read error-level logs as well as the exit code, since the streaming reader case and panics are fully visible only there; keep bucket versioning on for irreplaceable data; and apply least-privilege IAM. What this assessment cannot establish is the absence of defects inside the engine crates, the behaviour of S3-compatible endpoints that deviate from Amazon S3, or correctness beyond what the tests and experiments above exercised. Responsibility for whether the binary fits a given workflow remains with the operator who runs it.

</details>

### AI assessment of safety and correctness (by Codex)

<details>
<summary>Click to expand the full assessment</summary>

> Assessment date: 2026-07-22.
>
> Assessed tree: `main` at `839acc3`. The Rust implementation and build inputs are identical to v1.6.0 (`c454694`); the commits after that tag change README content only.
>
> Complete review boundary: all 84 Rust files under `src/` were examined in full, including their 477 embedded tests, rather than sampling high-risk modules. The review also covered all 63 offline process-test files, `tests/common/mod.rs`, all 28 `e2e_*.rs` suites, `Cargo.toml`, `Cargo.lock`, `build.rs`, `.cargo/config.toml`, `deny.toml`, `Dockerfile`, and all five GitHub Actions workflows. The four exact-pinned engine crates were checked at the interfaces and guarantees on which this wrapper relies; they remain separate dependencies, not source owned by this repository. Conclusions were derived from code and executable evidence, not from the neighboring assessment text.
>
> Coverage evidence: the supplied `lcov.info` (SHA-256 `2da2901598bffa46b6fda6718c815e34701d24870c3e9f9a984a2da1b28bb22a`) and `llvm-cov-report.txt` (SHA-256 `f7fa71e841669bbb65673fe311832c7f12752b5218b3c9258a45e7a0053daffb`) were parsed independently. Their line and function totals agree exactly.
>
> Limits: this was source review plus deterministic local verification, not formal proof. No fuzzing, Miri, sanitizers, fault injection, penetration test, or fresh live-AWS run was performed. The E2E configuration was compiled and linted, while its prior live execution is represented only by the supplied coverage artifacts.

#### Bottom line

No critical vulnerability was found in s7cmd's own code, and the ordinary Amazon S3 paths show strong defensive engineering. The wrapper has complete dispatch coverage, contains per-line batch failures instead of terminating the process, protects credential values in help and batch logs, and preserves dry-run and transfer-integrity controls. There is no `unsafe` block in production code and no production subprocess execution or shell evaluation; `batch-run` parses and dispatches commands in-process.

I would nevertheless describe v1.6.0 as **conditionally reliable, not safety-certified**. One narrow `mv` case can still delete an object, and batch input/cancellation behavior has availability and result-reporting defects. These are concrete open findings, not hypothetical objections to AI-generated code.

#### Open findings, ordered by operational impact

| Impact / priority | Finding | Consequence and boundary |
|---|---|---|
| High impact, narrow trigger | The `mv` self-move guard compares source and target endpoint strings literally (`src/util_bin/cli/mv.rs:65-77`). Equivalent spellings of the same service therefore bypass the guard. | With the same bucket and resolved key on an unversioned or version-suspended bucket, copy-then-delete can delete the object just written. The operator must explicitly construct this endpoint mismatch; bucket versioning is the strongest mitigation. Endpoint canonicalization or an explicit same-service override would close it. |
| Moderate | Batch memory is not bounded in aggregate. Default mode collects every parsed line in a `Vec` (`src/batch_run/parser.rs:60-88`), while streaming mode feeds an `unbounded_channel` (`src/batch_run/mod.rs:361`). | The 16 KiB per-line limit prevents a single-line allocation attack but not a huge script, and a fast streaming producer can outpace a slow S3 command until memory is exhausted. A total-line/byte cap and a bounded channel would make the resource guarantee real. |
| Moderate | Cancellation is reported inconsistently. `ls` maps cancellation to 0 (`src/ls_bin/mod.rs:75-78`); `sync` can complete a cancelled pipeline without a distinct cancellation status; and `clean` returns 0 as soon as it encounters any cancellation error, even if the same collected error set contains real deletion failures (`src/clean_bin/mod.rs:98-124`). | Automation and `batch-run` can classify interrupted work as success. In `clean`, a genuine error may be masked. Error aggregation should retain real errors and use one cancellation code, preferably 130, across engines. |
| Moderate | Streaming stdin is not promptly cancellable. After fail-fast or SIGINT the executor drains the channel and awaits its closure, then `run_streaming` awaits the reader (`src/batch_run/executor.rs:484-505,579-582`; `src/batch_run/mod.rs:399-412`). Tokio's stdin read can remain blocked until the producer closes the pipe. | `batch-run --streaming -` can hang after SIGINT or an early failure when stdin stays open. The process test explicitly documents that EOF is load-bearing (`tests/cli_sigint.rs:68-105`). A cancellable reader design or documented external pipe closure is required. |
| Low to moderate | Parallel stop checks occur before awaiting the next channel item or semaphore permit, with no re-check after the await (`src/batch_run/executor.rs:398-430,537-577`). | A SIGINT or reached error threshold can allow one already-queued command to start afterward. Because that command installs its own handler after the signal, it may run to completion. |
| Low to moderate | Exit aggregation understates some failures. All nonstandard codes, including caught panic 101, rank below warning 3 and not-found 4 (`src/batch_run/executor.rs:289-313`); streaming reader failure uses numeric `max(code, 1)` (`src/batch_run/mod.rs:402-411`). | A batch containing a panic plus a warning can exit 3/4, and a reader error after a warning can retain the warning code. Error-level logs remain accurate, but exit-code-only automation is not. |
| Low, availability | Nine JSON file/stdin inputs are read with unbounded `read_to_string` before parsing or sending: policy, CORS, encryption, lifecycle, logging, notification, replication, website, and public-access-block. | A mistaken or hostile multi-gigabyte input can exhaust memory. The annotation payload paths correctly demonstrate the bounded-read pattern and should be reused. |

Additional correctness edges are smaller but real. `rename` reports missing source/bucket as general error 1 while many sibling commands use not-found 4 (`src/util_bin/cli/rename.rs:52-58`). `mv --no-fail-on-verify-error` can delete the source and return success despite a verification warning, whereas `cp` returns warning 3. Annotation file output flushes, re-reads, verifies, and atomically renames, but does not `sync_all`, so its promise covers process crashes and detected corruption rather than power-loss durability (`src/util_bin/cli/get_object_annotation.rs:181-224`). Positional arguments inherited from the engine CLIs are environment-backed, so exported `SOURCE`, `TARGET`, or policy variables can silently fill destructive command arguments. There is no confirmation prompt outside `clean`.

`--dry-run` prevents mutations, but it is not a no-I/O or no-network guarantee. Client construction happens first and may perform credential-provider work. `sync`, `clean`, and transfer planning can enumerate or inspect S3 state; thin-wrapper examples include `cp --skip-existing`, annotation synchronization, and `create-bucket --if-not-exists`, which perform read-only S3 checks before deciding what they would do (`src/util_bin/cli/cp.rs:17-33`; `src/util_bin/cli/create_bucket.rs:45-64`). This is safe with respect to S3 state but matters in isolated environments and when metadata/SSO credential providers are enabled.

#### Safety controls that held under full-source review

- The 56-variant command enum has a corresponding non-exiting dispatch path. Configuration failures return 2 instead of invoking upstream `clap::Error::exit`, so one bad batch line cannot terminate its siblings (`src/dispatch.rs`). The large transfer futures are boxed to avoid known small-stack overflows.
- All 34 mutating command surfaces expose `--dry-run`. Thin wrappers return before their mutation calls, while `cp`, `mv`, `sync`, and `clean` carry the flag into their pinned engines. The live E2E corpus contains a state-unchanged case for every mutating command. Read-only setup and planning can still contact S3 as described above.
- `mv` normally has a sound deletion decision tree: no delete after cancellation, copy error, or verification warning without the explicit override; it checks cancellation again immediately before delete and pins deletion to the source version actually read (`src/util_bin/cli/mv.rs:100-161`). The endpoint-equivalence gap above is before that tree, not a failure of those gates.
- Annotation upload and download enforce the 1 MiB limit with bounded streaming reads. Upload sends Content-MD5 and CRC64NVME and verifies the returned checksum; download verifies content length plus available checksums, rejects unsupported returned algorithms cleanly, writes beside the target, re-verifies the saved bytes, then atomically persists. A pre-existing destination survives verification failure (`src/util_bin/cli/put_object_annotation.rs:44-100`; `get_object_annotation.rs:87-224,315-394`).
- `batch-run` enforces a 16 KiB line limit incrementally, rejects invalid UTF-8, tokenizes with `shlex`, rejects nested batches, stdin/stdout conflicts and per-line tracing, caps `--parallel` at 1024, and converts parse/validation errors into per-line exit 2. Dispatch is wrapped in `catch_unwind`, and all build profiles retain `panic = "unwind"`, so a subcommand panic becomes a recorded exit 101 instead of tearing down the batch (`src/batch_run/`; `Cargo.toml:103-113`).
- Credential values are hidden from env-aware help output across the complete clap tree (`src/cli.rs:486-517`). Batch log redaction handles both `--flag value` and `--flag=value`, including the malformed-quoting fallback (`src/batch_run/redact.rs`). The pinned engine credential types redact `Debug` output and zeroize secret fields, so the remaining trace-level full-config logs do not print secret keys or session tokens. Access-key identifiers may remain partially visible by design.
- Production source contains no direct `unsafe`, no shell invocation, and no process-spawning API. Lua support is inherited from s3sync's default features and deliberately expands the binary's attack surface, but Lua runs only operator-supplied filter code rather than data received from S3.

#### Tests, coverage, and what the numbers mean

The supplied report totals are 97.63% regions (14,282/14,629), 96.94% functions (1,110/1,145), and 98.55% lines (10,297/10,448). There are no branch records: both artifacts report zero measured branches. The line and function figures are internally consistent, but two qualifications prevent treating 98.55% as production-only coverage:

1. `lcov.info` includes functions named under `::tests::` and line records from `#[cfg(test)]` modules—for example, the `mv` fake storage and tests above line 900. The 10,448-line denominator therefore mixes product and test implementation. Similarly, the README's 16,949 physical lines under `src/` include the 477 embedded tests; they are not all production lines.
2. The loopback mock records method and request target, then drains request bodies without retaining them and exposes no captured request headers (`tests/common/mod.rs:200-259`). Most offline wrapper tests therefore prove parsing, routing, status mapping, and response handling, but not exact outbound JSON or signed headers. Live-AWS round trips provide the stronger payload evidence and are not a CI gate.

The corpus is broad: 477 embedded test annotations, 692 annotations in 63 offline process-test files, and 258 annotations in 28 gated live-AWS files. Some tests are explicitly coverage-oriented rather than behavioral—for example, local sync tests ignore `run`'s result—so count and coverage must be read alongside assertion quality.

During this assessment, these commands passed without warnings or failures:

- `cargo fmt --all --check`
- `cargo test --all-features --locked`
- `cargo clippy --all-features --all-targets --locked -- -D warnings`
- `RUSTFLAGS="--cfg e2e_test" cargo clippy --all-features --all-targets --locked -- -D warnings`
- `cargo deny -L error check` (`advisories`, `bans`, `licenses`, and `sources` all clean)

The E2E tests were compiled by the second clippy run but not executed because doing so mutates a configured AWS account. Default `cargo test` correctly sees those gated files as zero-test targets.

#### Dependency and delivery assessment

The four engine crates are exact-pinned and the lockfile is committed. Release and publish jobs use `--locked`, produce SHA-256 files, and attest release archives. The resolved HTTP stack uses rustls 0.23.42; `openssl-sys` is absent and explicitly denied. Cargo-deny rejects unknown registries and git sources and has no ignored advisories.

The remaining supply-chain weaknesses are conventional rather than runtime defects: most GitHub Actions are referenced by mutable major-version tags rather than commit SHAs; CI follows moving `stable`; the gating clippy job omits `--all-features --all-targets`; CI does not compile the `e2e_test` configuration or gate on coverage; release jobs do not rerun tests; and the Dockerfile uses mutable base tags and builds without `--locked`. These do not invalidate the tested source tree, but they weaken reproducibility and increase the trust placed in workflow dependencies.

#### Reliability conclusion

For routine, supervised Amazon S3 use, the evidence supports the tool's core claims: routing is exhaustive, mutation previews exist everywhere, credential handling is deliberate, transfer deletion is normally gated, annotation bytes receive unusually strong integrity treatment, and failures generally become structured nonzero results. I found no path in the ordinary same-endpoint flow that silently mutates the wrong S3 resource, no designed logging path that emits credential secrets, and no dry-run path that invokes an S3 mutation.

For unattended destructive automation, the conditions matter. Canonicalize and keep source/target endpoints identical for same-service `mv`, enable bucket versioning for valuable data, bound or trust batch inputs, close streaming stdin on cancellation, and monitor error logs in addition to the final exit code. Until the `mv` endpoint comparison, aggregate batch bounds, and cancellation/error aggregation are fixed, those controls are part of the safety model rather than optional operational advice.

</details>

### AI assessment of safety and correctness (by Gemini)

<details>
<summary>Click to expand the full assessment</summary>

> Assessment date: 2026-07-22.
>
> Assessed version: 1.6.0 (branch `main`, commit `837518b`, tagged `v1.6.0` at `c454694`).
>
> Method and scope of evidence: This assessment was performed completely from scratch for v1.6.0, independently reviewing the entire codebase without relying on prior summaries. All 84 Rust source files under `src/` (16,949 physical lines) were systematically inspected in full, including the `s7cmd`-original `src/batch_run/` engine (8 modules), top-level CLI parsing and dispatch (`src/main.rs`, `src/cli.rs`, `src/dispatch.rs`), and all four vendored CLI frontends (`src/util_bin/`, `src/sync_bin/`, `src/clean_bin/`, `src/ls_bin/`, total 73 files). Additionally, all 63 offline process-level test suites (`tests/cli_*.rs` and `tests/batch_run.rs`), 28 live-AWS E2E test suites (`tests/e2e_*.rs`), test harness helpers (`tests/common/mod.rs`), build configuration (`Cargo.toml`, `Cargo.lock`, `build.rs`, `.cargo/config.toml`), security policies (`deny.toml`), `Dockerfile`, and all five GitHub Actions workflows were examined. Code coverage artifacts `llvm-cov-report.txt` and `lcov.info` were verified for internal consistency (line and function totals match exactly). Local verification commands were run from scratch and confirmed clean: `cargo fmt --all --check` zero diffs; `cargo clippy --all-features --all-targets --locked -- -D warnings` zero warnings; `RUSTFLAGS="--cfg e2e_test" cargo clippy --all-features --all-targets --locked -- -D warnings` zero warnings; `cargo test --all-features --locked` 477 unit tests passed; `cargo deny -L error check` clean with zero advisories ignored. Interfaces to the four exact-pinned upstream engine crates (`s3sync = "=1.60.0"`, `s3util-rs = "=1.8.0"`, `s3rm-rs = "=1.4.0"`, `s3ls-rs = "=1.1.0"`) were audited.
>
> Limits of evidence: This review constitutes static code audit and deterministic local test execution. It does not include formal mathematical proof, fuzzing, Miri execution, memory/thread sanitizers, or live AWS network mutation runs. E2E tests are gated under `cfg(e2e_test)` and run against maintainer AWS infrastructure; coverage measures code execution paths, not absolute logical correctness under arbitrary cloud edge cases.

**Question addressed.** As a CLI wrapper and batch runner for Amazon S3, s7cmd delegates 55 of its 56 subcommands to four exact-pinned engine crates while providing one custom engine (`batch-run`). This evaluation examines whether s7cmd introduces routing errors, credential leaks, unhandled panics, uncontained batch failures, data destruction during transfers, or unexpected mutation behaviors during dry runs.

#### Command Dispatch & Structural Non-Exiting Guarantee

- **Complete Subcommand Routing**: The `Cmd` enum defines 56 variants (`src/cli.rs:270-426`): 21 read-only subcommands, 34 mutating subcommands, and `batch-run`. `src/dispatch.rs` maps every variant to its underlying logic. Routing correctness is validated across 62 subcommand parsing tests (`tests/cli_routing.rs`), 63 unit tests in `src/dispatch.rs`, and E2E suites.
- **Process Stability & Non-Exiting Contract**: Production code under `src/` contains zero calls to `std::process::exit`. Every dispatch branch returns a numeric status (`ExitStatus` or `i32`), which `main()` converts to `std::process::ExitCode` (`src/main.rs:44-45`). In the vendored frontends, upstream process-exiting calls (`load_config_exit_if_err`, state flag validation exits) were deliberately refactored into non-exiting status returns (`dispatch.rs:278,401,418`). A configuration or parameter error in one subcommand returns exit 2 without terminating the parent process or killing a `batch-run` sequence.
- **Stack Memory Protection**: Large subcommand future types are explicitly `Box::pin`-ed (`src/dispatch.rs:16-22`) to keep dispatch stack frames well under the 2 MB stack limit of test worker threads.

#### `batch-run` Engine Architecture & Fault Isolation

`batch-run` represents s7cmd's original execution engine, built with multi-layered defensive controls:

- **Incremental Line Buffering**: Input lines are read using `read_line_capped` (`src/batch_run/parser.rs:101-135`), which enforces a strict 16 KiB limit (`MAX_LINE_LEN`) incrementally via `BufRead::fill_buf`. Pathological multi-gigabyte single-line inputs are aborted after buffering ~16 KiB rather than exhausting process memory. UTF-8 validation and POSIX shell tokenization (`shlex`) are applied to every line.
- **Pre-Execution Validation**: `src/batch_run/validate.rs` validates parsed argument structures before running commands. It explicitly rejects nested `batch-run` invocations, stdin/stdout dash operands (`-`), and per-line tracing/verbosity flags (`-v`, `--tracing-log-format`). Validation failures synthesize exit 2 and count toward `--max-errors` / `--continue-on-error` thresholds rather than aborting the batch.
- **Panic Boundary Containment**: Every subcommand execution is wrapped in `futures::FutureExt::catch_unwind` (`src/batch_run/executor.rs:125-167`). Any unexpected panic inside a subcommand is caught, logged with line numbers and redacted text, assigned exit code 101, and counted toward error limits. This mechanism relies on `panic = "unwind"` specified across all build profiles in `Cargo.toml:103-113`.
- **Severity-Ranked Exit Codes**: Batch exit status is determined by severity ranking rather than simple maximum value: `exit 1` (error) > `exit 2` (arg/validation error) > `exit 3` (warning) > `exit 4` (not found) > other non-zero > `exit 0` (`src/batch_run/executor.rs:294-303`). Per-line SIGINT (exit 130) is bucketed as `skipped` (`executor.rs:281-287`) and does not trip error thresholds.
- **Phased Signal Handling**: Signal listeners are not installed during the script reading/validation phase (where Ctrl-C terminates immediately). The SIGINT handler is registered only before command execution starts, ensuring in-flight futures handle cancellation cleanly while preventing new commands from spawning (`src/batch_run/mod.rs:253-331`).
- **Parallel Execution Safety**: `--parallel` concurrency is constrained to `[1, 1024]` by a custom clap parser (`src/cli.rs:148-170`), preventing semaphore allocation panics. Tokio `LocalSet` drives async execution with a concurrency semaphore.
- **Shell Auto-Completion Isolation**: Top-level `--auto-complete-shell` is disarmed on subcommands (`src/main.rs:26-34`), preventing inherited environment variables from altering subcommand argument parsing.

#### Operator Safeguards, Dry-Run Integrity & Transfer Safety

- **Comprehensive Dry-Run Coverage**: All 34 mutating subcommands accept `--dry-run`; none of the 21 read-only subcommands accept it (`tests/cli_dry_run.rs`). Thin wrappers abort before invoking mutating S3 API calls, while complex operations (`cp`, `mv`, `sync`, `clean`) propagate `--dry-run` into their underlying engine. Dry-run automatically elevates minimum logging verbosity to `info` level (`src/main.rs:62-134`) so `[dry-run]` execution logs are visible. 34 E2E live-AWS tests verify that dry-run calls leave cloud resources unmodified (`tests/e2e_dry_run.rs`).
- **High-Risk Delete Protection**: `clean` (bulk delete) mandates `--force` or interactive `"yes"` confirmation (`src/clean_bin/mod.rs:57-69`). Interrupted prompts exit via default OS signal handling.
- **`mv` Copy-Then-Delete Decision Tree**: `mv` checks for self-move conditions (`src/util_bin/cli/mv.rs:46-98`) by comparing source and target buckets, endpoints, resolved keys, and version IDs before executing any transfer. Deletion of the source object is guarded by a 4-gate decision tree (`mv.rs:100-162`): (1) no cancellation during transfer, (2) successful copy completion, (3) no checksum/verification warnings (unless `--no-fail-on-verify-error`), (4) final cancellation token re-check immediately prior to delete. Source deletion specifies the exact version ID read during copy.
- **Object Annotation Integrity**: Annotation payloads enforce a 1 MiB limit (`src/util_bin/cli/put_object_annotation.rs:44-61`, `get_object_annotation.rs:316-333`). Uploads verify Content-MD5 and CRC64NVME response checksums. Downloads stream bytes into a temporary file (`tempfile`), verify on-disk payload checksums, and perform an atomic filesystem rename only after successful verification (`get_object_annotation.rs:181-225`). Pre-existing destination files remain intact if verification fails.
- **Partial State Warnings**: `create-bucket --tagging` executes bucket creation followed by tagging. If tagging fails after bucket creation, it emits a warning detailing partial state and exits 3 (`src/util_bin/cli/create_bucket.rs:83-91`).

#### Credential Hygiene & Masking

- **Engine Credential Redaction**: Access key structs and SSE encryption keys in engine crates derive `Zeroize`/`ZeroizeOnDrop` and implement `Debug` formatting returning `** redacted **`. Detailed config dumps in `cp`/`mv` dispatch limit logged fields to non-sensitive metadata (`src/dispatch.rs:453-465`).
- **Help Output Protection**: `hide_credential_env_values` (`src/cli.rs:503-517,636-663`) recursively removes default environment variable secret values from `--help` text across all subcommands. Process-level tests (`tests/cli_help.rs:611-737`) confirm credential flag names display without leaking environment variable contents.
- **Batch Log Redaction**: `src/batch_run/redact.rs` sanitizes inline credentials (access keys, secret keys, session tokens, presigned URLs) in both `--flag value` and `--flag=value` syntax before logging or writing JSON trace records. Unparseable lines use fallback regex/whitespace scrubbing (`tests/batch_run.rs:1309-1410`).

#### Supply Chain & Build Pipeline Security

- **Pinned Engine Dependencies**: `Cargo.toml` pins engine dependencies to exact versions (`s3sync = "=1.60.0"`, `s3util-rs = "=1.8.0"`, `s3rm-rs = "=1.4.0"`, `s3ls-rs = "=1.1.0"`). `Cargo.lock` is committed; build and publication workflows enforce `--locked`.
- **TLS & Crypto Stack**: Cryptographic transport uses `rustls 0.23.42` with `aws-lc-rs 1.17.3` and OS trust anchors. `openssl-sys` is excluded from the dependency tree and explicitly banned in `deny.toml`. `ring` is restricted to `ureq` test dependencies.
- **Dependency Auditing**: `cargo deny -L error check` runs on all pushes/PRs (`ci.yml`) and daily schedules (`cargo-deny.yml`). `deny.toml` maintains `advisories.ignore = []` and enforces license allowlists.
- **Release Provenance**: Release builds (`cd.yml`) compile with `--locked`, produce SHA-256 digests, generate GitHub Actions build provenance attestations, and publish to crates.io via OIDC trusted publishing.
- **Embedded Lua Interpreter**: `s3sync` includes `lua_support` (mlua 0.12.0) by default to support Lua filter callbacks (`Cargo.toml:14-18`). While intentional, embedding Lua increases binary attack surface.

#### Test Corpus & Coverage Evidence

s7cmd includes 1,427 test annotations across three tiers:
1. **Embedded Unit Tests**: 477 tests in `src/` (covering `dispatch.rs`, `executor.rs`, `redact.rs`, `mv.rs`, `get_object_annotation.rs`, etc.).
2. **Offline Integration Tests**: 692 tests across 63 files (`tests/cli_*.rs` and `tests/batch_run.rs`) executing CLI binaries against an in-process loopback mock server (`127.0.0.1:0`).
3. **Live AWS E2E Suites**: 258 tests across 28 files (`tests/e2e_*.rs`) gated behind `cfg(e2e_test)`.

`llvm-cov-report.txt` and `lcov.info` present combined test coverage (unit + CLI + live-AWS E2E run):
- **Line Coverage**: 98.55% (10,297 / 10,448 executable lines; 151 missed)
- **Function Coverage**: 96.94% (1,110 / 1,145 functions; 35 missed)
- **Region Coverage**: 97.63% (14,282 / 14,629 regions; 347 missed)
- **Branch Coverage**: Not measured by standard llvm-cov instrumentation in Rust.

Core module coverage highlights: `batch_run/executor.rs` (98.73% lines), `batch_run/mod.rs` (98.87% lines), `batch_run/redact.rs` (100% lines), `util_bin/cli/mv.rs` (100% lines), `util_bin/cli/get_object_annotation.rs` (99.26% lines).

#### Identified Technical Findings & Operational Limitations

1. **Non-Canonical Endpoint Comparison in `mv` Self-Move Guard**: `check_not_self_move` (`src/util_bin/cli/mv.rs:67-77`) compares endpoint URLs using string equality (`source_endpoint != target_endpoint`). Syntactically distinct spellings of the same S3 endpoint (e.g., HTTP vs HTTPS, IP vs hostname) bypass the self-move check, relying on bucket versioning to prevent data loss.
2. **Unbounded Aggregate Batch Memory**: While single lines are capped at 16 KiB, standard `batch-run` buffers all parsed lines in a `Vec` (`src/batch_run/parser.rs:61`), and `--streaming` mode uses an `unbounded_channel` (`src/batch_run/mod.rs:361`). Scripts with millions of lines can consume substantial memory.
3. **Inconsistent Cancellation Exit Reporting**: Cancellation handling varies by subcommand: `ls` maps cancellation to exit 0 (`src/ls_bin/mod.rs:75-78`); `clean` returns exit 0 upon encountering cancellation even if earlier object deletion errors occurred (`src/clean_bin/mod.rs:98-124`); `sync` maps cancellation to exit 0.
4. **Streaming Stdin Cancellation Delay**: In `batch-run --streaming -`, reading from an open stdin pipe can remain blocked on Tokio's stdin reader after SIGINT or early error failure until EOF is received (`tests/cli_sigint.rs:72-80`).
5. **Parallel Executor Queue Signal Window**: In parallel execution, the interrupt check occurs before awaiting a semaphore permit (`src/batch_run/executor.rs:399-430`). A SIGINT arriving during permit await can allow one previously queued command to start.
6. **Severity Code Aggregation Edge Cases**: Exit codes outside 1–4 (including panic exit 101) rank below exit 4 in batch severity ranking (`src/batch_run/executor.rs:289-313`), causing a batch containing a panic and a warning to exit 3 or 4.
7. **Uncapped JSON Configuration Reads**: Nine bucket configuration commands (policy, CORS, lifecycle, encryption, website, logging, notification, replication, public-access-block) read input files using unbounded `read_to_string`, unlike annotation payloads which enforce a 1 MiB cap.
8. **Network Activity During Dry Runs**: Subcommands with `--dry-run` perform client configuration and read-only S3 checks (e.g. `cp --skip-existing` issuing HeadObject, `create-bucket --if-not-exists` issuing HeadBucket) before suppressing mutating calls.

#### Reliability Summary

s7cmd exhibits robust defensive engineering for Amazon S3 management. Subcommand routing is complete, non-exiting dispatch guarantees process persistence during batch runs, panic boundaries prevent process crashes, credential redaction is systematically implemented, and object annotation workflows provide strong atomic verification.

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

s7cmd targets **Amazon S3** as its only supported platform and is
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
Ceph RGW, DigitalOcean Spaces, IBM COS, and similar) is provided
strictly **as-is**, with **absolutely no support or assistance** —
such services may work via `--endpoint-url` (and
`--source-force-path-style` / `--target-force-path-style` when
path-style addressing is required), but they are not part of the
official test matrix and behavior may change between releases. This
is a structural consequence of building on `aws-sdk-rust`, which is
generated from AWS service models and assumes Amazon S3 semantics
(checksum headers, endpoint resolution, signing variants, response
schemas); features that depend on AWS-specific semantics, such as
CRC64NVME checksums or newer S3 API additions, may not work against
non-AWS endpoints. In practice, most s7cmd subcommands are not
supported on S3-compatible storage and are therefore unlikely to
work: the bucket-configuration family drives management APIs that
such services implement partially or not at all, and `rename`,
`restore-object`, and the object-annotation subcommands depend on
Amazon-S3-only APIs and checksum semantics. Bug reports, questions,
and assistance requests regarding S3-compatible storage will not be
addressed.

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

## Non-Goals

The following are explicitly out of scope and will not be added,
regardless of demand:

- Support, testing, or guaranteed compatibility for any
  storage service other than Amazon S3. S3-compatible storage is
  provided strictly as-is, with no support or assistance, as
  described in the Scope section above; adding dedicated code
  paths, provider-specific workarounds, or backends for services
  such as MinIO, Cloudflare R2, Backblaze B2, Wasabi, Ceph RGW,
  DigitalOcean Spaces, IBM COS, Tencent COS, Alibaba OSS, Azure
  Blob Storage, or Google Cloud Storage is out of scope.
- Feature parity with, or porting features from, other S3 clients.
  Feature requests of the form "tool X has feature Y, please add
  it to s7cmd" — including variants such as "feature Y would also
  be useful in s7cmd," "many users expect Y because tool X has it,"
  or "Y is missing compared to tool X" — will be closed without
  further discussion. The existence of a feature, flag, command,
  output format, or behavior in `aws s3`, `aws s3api`, `s3cmd`,
  `s4cmd`, `s5cmd`, `s6cmd`, `rclone`, or any other S3 tool carries no weight
  in s7cmd's design decisions, regardless of how the request is
  framed. Each feature is evaluated solely against s7cmd's own
  scope and the design principles of its underlying libraries. If
  the feature you need exists in another tool, use that tool.
- Outperforming other S3 tools on raw speed or memory usage.
  Performance and resource consumption are addressed only when they
  compromise practical workflows — not for edge cases or benchmark
  wins. Issues of the form "tool X transfers Y MB/s faster",
  "tool X transfers Y objects/second faster", or
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
- Edge cases that are more reasonably addressed by using the AWS
  SDK directly, shell scripting, or other purpose-built tooling.
  s7cmd is not intended to cover every conceivable S3 use case;
  niche or one-off requirements that can be straightforwardly
  handled by combining the AWS SDK, shell pipelines, or existing
  tools fall outside its scope.
- Changes to flag names, subcommand names, default values, output
  formats, log formats, or exit code assignments based on subjective
  preference. Such interfaces are stabilized once shipped; breaking
  changes are made only when required by an underlying library, an
  upstream SDK, or a clear correctness bug.
- Additional platform targets, distribution channels, or package
  manager registrations beyond those listed in Requirements and
  Installation. Community-maintained packages are welcome but will
  not be endorsed or supported.

Issues and pull requests requesting any of the above will be closed.

## Maintenance Model

s7cmd is maintained as a personal project. Dependency updates and
critical bug fixes are applied on a best-effort basis. New features
are not actively solicited. If you need guaranteed enterprise
support, this is not the tool for you.

## Intended Audience and Issue Tracker Scope

s7cmd assumes operational familiarity with Amazon S3 and the AWS
SDK. It is aimed at engineers who already run S3 workloads — not
at learners or general AWS users.

The issue tracker accepts:

- Reproducible defects in s7cmd's own behavior (with version,
  exact command, and observed vs. expected output).
- Scope-aligned feature discussion, subject to the Non-Goals
  section above.

The issue tracker does **not** accept:

- General questions about S3, IAM, AWS credentials, or AWS
  account configuration. See the [AWS documentation](https://docs.aws.amazon.com/s3/).
- Usage questions about other S3 clients.
- Help with user shell scripts, pipelines, or CI configurations
  that do not isolate an s7cmd-specific defect.
- Tutorials or design consulting.
- Diagnosing or fixing performance degradation, resource exhaustion,
  or errors caused by raising concurrency settings.
- Questions and issues that belong with AWS, with the operator
  of an S3-compatible storage service, or with the operating
  system vendor rather than with s7cmd — including general S3,
  IAM, KMS, networking, and account-configuration questions;
  S3 (or S3-compatible) service behavior such as request rate
  limits, 503 SlowDown, consistency semantics, or regional
  availability; operating-system configuration and behavior such
  as `ulimit` and file-descriptor limits, kernel networking
  parameters, filesystem quirks, shell quoting, path-length
  limits, code signing, or antivirus interference; and anything
  that reproduces with the AWS CLI, the AWS SDK, or the vendor's
  own client directly. Refer to the [AWS documentation](https://docs.aws.amazon.com/s3/),
  AWS Support, your storage vendor's documentation, or your OS
  vendor's documentation. If the issue is not specific to s7cmd's
  own code, it belongs there, not here.

Out-of-scope issues will be closed without further discussion.

## Contributing

- Bug reports are welcome, but responses are not guaranteed.
- Since this project is considered functionally complete, I will not accept any feature requests.
- If you find this project useful, feel free to fork and modify it as you wish.

🔒 I consider this project “complete” and will maintain it only minimally going forward.
However, I intend to keep the AWS SDK for Rust and other dependencies up to date monthly.

**Issue and PR lifecycle**

To keep the tracker focused, an issue or PR with no activity for 30 days is labeled `stale` and closed 7 days later unless a new comment (or, for PRs, a new commit) is added. Items labeled `pinned` or `security` are exempt; PRs are also exempt from `pinned`. Closed items can always be reopened.

## License

Apache-2.0
