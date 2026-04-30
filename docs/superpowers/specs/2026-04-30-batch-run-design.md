# `batch-run` subcommand — design

Status: draft (awaiting human review)
Date: 2026-04-30
Author: AI-generated (Claude Code), human review pending

## 1. Goal

Add a new `batch-run` subcommand that reads s7cmd commands from standard
input and executes them within the same process. The motivating use cases
are: scripted workloads that issue many bucket-administration calls,
fan-out of object operations derived from `s7cmd ls | awk | s7cmd batch-run`
pipelines, and any context where launching one s7cmd subprocess per
operation is unacceptably slow.

The command runs subcommands in-process — no `fork`/`exec` per line — so
overhead per command is the cost of clap parsing plus an async function
call, not a process boundary. AWS SDK clients are still constructed
per-line (each subcommand brings its own auth/endpoint flags).

## 2. Non-goals

- Reading commands from a file argument. Users redirect stdin
  (`s7cmd batch-run < cmds.txt`).
- Compatibility with the input syntax of any other tool's `run`
  command, including `s5cmd run`. Lines are parsed as
  `s7cmd <subcommand> <args>` only — no implicit `s7cmd ` prefix
  shortcut, no s5cmd-specific flag aliases.
- Workflow features beyond "run these N commands": no DAGs, no
  retries, no per-line timeouts, no cross-line variable interpolation.
- Killing in-flight commands when fail-fast trips. In parallel mode,
  fail-fast prevents new commands from starting; running ones complete
  on their own (matching the user's stated semantics).

## 3. CLI surface

```
s7cmd batch-run [OPTIONS]

OPTIONS
  --parallel <N>          Number of commands to run concurrently.
                          Default: 1 (sequential). 0 = use all logical CPUs.
  --streaming             Execute commands as they are read from stdin
                          (no progress bar). Default: read all of stdin
                          first, then execute (with progress bar).
  --continue-on-error     Continue executing remaining commands after a
                          failure. Default: fail-fast (sequential stops;
                          parallel prevents new spawns).
  --quiet                 Suppress the end-of-run summary line on stderr.

  Tracing flags (same shape as every other subcommand):
  --aws-sdk-tracing
  --json-tracing
  --filter-trace-directive <DIRECTIVE>
  --tracing-directory <PATH>
  ...
```

Input is read from stdin only. There is no positional argument.

The subcommand is listed in the top-level help under a new "Batch"
heading group.

## 4. Behavior

### 4.1 Line parsing

- Each non-empty, non-comment line is tokenized with `shlex::split`.
- Blank lines (whitespace only) are skipped.
- Lines whose first non-whitespace character is `#` are skipped
  (comments).
- Line continuation with trailing `\` is **not** supported.
- Tokens are prepended with `"s7cmd"` (clap's argv[0]) and parsed via
  `Cli::try_parse_from`. The result must be `Some(Cmd)` other than
  `Cmd::BatchRun`; nested `batch-run` is rejected during validation.

### 4.2 Validation (before any command runs)

After parsing all lines (read-all mode) or as each line is parsed
(streaming mode), the following rules are checked. Any violation exits
`batch-run` with code 1 *before* any command runs (or, in streaming
mode, before the violating line runs and any further line is read):

1. **Parse failure.** Reported with line number and offending raw line.
2. **Nested batch-run.** `Cmd::BatchRun(_)` is rejected.
3. **Stdio cp/mv.** `Cmd::Cp` / `Cmd::Mv` whose source or target is
   `StoragePath::Stdio` is rejected with a clear "stdin/stdout
   transfers are not allowed inside batch-run" message. (`Mv` already
   rejects stdio at config validation upstream; this pre-check just
   produces a friendlier error before the line is dispatched.) One
   shared stdin cannot be split across N parallel commands, and the
   input stream is already consumed by `batch-run` itself.
4. **Per-line tracing flags.** Any subcommand whose
   `common.tracing.*` is set is rejected with a clear "pass it to
   batch-run instead" message. Detection runs on the parsed `Cmd`,
   not on raw strings, so all clap-accepted spellings are caught.

In read-all mode, all lines are validated up front. In streaming mode,
each line is validated at the moment it is parsed; failure stops the
read loop immediately.

### 4.3 Execution

**Sequential** (`--parallel 1`, default):

```
for line in lines:
    eprintln!("==> line {N}: {raw}");
    code = dispatch(line.cmd).await;
    record(code);
    if code != 0 and not continue_on_error: break;
```

**Parallel** (`--parallel N>1`, or `--parallel 0` → `num_cpus::get()`):

```
sem = Semaphore::new(workers);
cancel = CancellationToken::new();
joinset = JoinSet::new();

for line in lines:
    if cancel.is_cancelled() and not continue_on_error: break;
    permit = sem.acquire_owned().await;
    joinset.spawn({
        eprintln!("==> line {N}: {raw}");
        code = dispatch(line.cmd).await;
        if code != 0 and not continue_on_error: cancel.cancel();
        (N, code)
    });

while let Some(joined) = joinset.join_next().await:
    record(joined);
```

The cancel signal *prevents new spawns*; running tasks complete
naturally. We use the existing tokio runtime — no nested runtime per
line.

### 4.4 Output

Per-command output is **not** prefixed line-by-line. Instead, a banner
is printed to stderr immediately before each command's output begins:

```
==> line 17: cp --server-side-copy s3://src/file s3://dst/file
```

In parallel mode, banners and command output from different commands
may interleave. This matches how `xargs -P` and `make -jN` behave;
users who need per-command separation already have `tee`/`logger`.

### 4.5 Progress bar

In read-all mode, an `indicatif::ProgressBar` is shown on stderr with
total = `lines.len()`. Template:

```
[{bar:40.cyan/blue}] {pos}/{len} ({msg}) {elapsed_precise}
```

`msg` is updated to `"X ok, Y failed"` after each completion.

The bar is suppressed when:

- `--quiet` is set, **or**
- `--streaming` is set, **or**
- stderr is not a terminal (`is_terminal::is_terminal(stderr) == false`).

It is drawn via `indicatif::MultiProgress` so command stdout/stderr
output appears above the sticky bar without corruption.

### 4.6 Summary

After all commands finish (or fail-fast halts execution), one line is
printed to stderr unless `--quiet`:

```
batch-run: 47 ok, 2 failed, 1 skipped, elapsed 12.4s
```

`skipped` counts lines that did not run because fail-fast tripped.
With `--continue-on-error`, `skipped` is always 0.

### 4.7 Exit code

The final exit code is the **maximum (worst) exit code** observed
across all commands that ran:

```rust
results.iter().map(|(_, code)| *code).max().unwrap_or(0)
```

`0` on success across the board. Non-zero if any command failed,
warned, was not-found, or was cancelled. This is intentionally
stricter than "last command's exit code" — it gives the user a single
boolean answer to "did anything go wrong?"

If `batch-run` itself fails before any command runs (parse error,
validation error), the exit code is `1`.

If interrupted by SIGINT, the worst-seen rule naturally produces
`130` from any cancelled in-flight command.

### 4.8 SIGINT (Ctrl-C)

A single `tokio::signal::ctrl_c()` task is registered at the top of
`batch_run::run`. On signal: cancel the spawn loop. The
process-wide signal handler that cp/mv/sync/clean register inside
their own `dispatch` arms also fires, propagating cancellation into
the in-flight transfers. No double-handling — the SDK's cancellation
token is shared per-task, not per-process.

## 5. Refactor: in-process dispatch

`batch-run` requires that subcommand handlers return an exit code
instead of calling `std::process::exit`. Today every match arm in
`src/main.rs` ends in `std::process::exit(exit_code)`. The refactor:

1. **New `src/dispatch.rs`** containing one function:
   ```rust
   pub async fn dispatch(cmd: Cmd) -> i32
   ```
   It contains all 38 existing match arms, each returning `i32`
   instead of `process::exit`-ing.

2. **`src/main.rs` shrinks** to ~30 lines:
   ```rust
   let cli = Cli::from_arg_matches(...)?;
   if let Some(shell) = cli.auto_complete_shell { ... }
   let cmd = cli.command.expect(...);
   init_tracing_once(&cmd);     // see 5.1
   let code = dispatch::dispatch(cmd).await;
   std::process::exit(code);
   ```

3. **Existing single-subcommand behavior is preserved.** No flag
   names change. No exit codes change. No output changes.

### 5.1 One-shot tracing init

`tracing-subscriber::set_global_default` errors on second call.
Today, each handler initializes tracing from its own args. After the
refactor:

- Tracing init moves out of the per-handler arms into a single
  `init_tracing_once(&cmd)` call in `main.rs`.
- For single-subcommand invocations: the function reads
  `cmd.<subcommand>.common.tracing.*` and initializes accordingly.
- For `Cmd::BatchRun(args)`: the function reads `args.tracing.*`
  instead.
- Per-line tracing flags inside `batch-run` are rejected at
  validation (rule 4.2.4), so `dispatch` never sees a line that
  would attempt to re-init.
- A `std::sync::Once` belt-and-suspenders guard inside the wrapper
  protects against future code paths that accidentally call init
  twice (and against tests that drive `dispatch` repeatedly in
  one process).

### 5.2 Vendored bin patches (sync / clean / ls)

The vendored `sync_bin::cli::run`, `clean_bin::run`, and `ls_bin::run`
each call `std::process::exit(...)` from multiple internal paths
(warnings, errors, panic recovery). Inside `batch-run` this would
kill the whole process mid-batch, so all three are patched to return
their exit code instead:

```rust
// before
pub async fn run(config: Config) -> Result<()> { ...; std::process::exit(N); }
// after
pub async fn run(config: Config) -> Result<i32> { ...; return Ok(N); }
```

Affected `process::exit` sites (counts current as of `main` at the
time of writing):
- `src/sync_bin/cli/mod.rs`: 1 (warning path)
- `src/clean_bin/mod.rs`: 6 (warning + multiple error paths)
- `src/ls_bin/mod.rs`: 2 (bucket-list error + listing error)

Each site becomes `return Ok(N)` (or `return Err(...)` for the
abnormal-termination path, where the caller maps to
`EXIT_CODE_ABNORMAL_TERMINATION`). The new return type is `Result<i32>`.
`dispatch.rs` translates `Ok(n)` into the integer exit code and `Err(e)`
into `EXIT_CODE_ERROR` (matching today's `main.rs` error handling).

This pattern matches the existing "Adjustments:" header convention in
each vendored file. The patches are one-time and mechanical.

### 5.3 BatchRunArgs tracing fields

`s3util_rs::config::args::common_client::CommonClientArgs` bundles the
tracing flags with all of the AWS auth/endpoint flags. We don't want
the AWS flags on `batch-run` (each per-line subcommand brings its own),
so `BatchRunArgs` defines its tracing flags directly:

```rust
#[arg(long, default_value_t = false, help_heading = "Tracing/Logging")]
pub json_tracing: bool,
#[arg(long, default_value_t = false, help_heading = "Tracing/Logging")]
pub aws_sdk_tracing: bool,
#[arg(long, default_value_t = false, help_heading = "Tracing/Logging")]
pub span_events_tracing: bool,
#[arg(long, default_value_t = false, help_heading = "Tracing/Logging")]
pub disable_color_tracing: bool,
#[command(flatten)]
pub verbosity: clap_verbosity_flag::Verbosity<clap_verbosity_flag::WarnLevel>,
```

The `build_tracing_config()` method mirrors
`CommonClientArgs::build_tracing_config()` — same shape, same
`Option<TracingConfig>` return.

## 6. Module layout

```
src/
├── cli.rs                       (modified: add Cmd::BatchRun; help template)
├── main.rs                      (gutted: ~640 → ~30 lines)
├── dispatch.rs                  (new)
├── batch_run/
│   ├── mod.rs                   (new: pub async fn run(args) -> i32)
│   ├── args.rs                  (new: BatchRunArgs)
│   ├── parser.rs                (new: stdin → ParsedLine stream / Vec)
│   ├── validate.rs              (new: per-line rules from §4.2)
│   ├── executor.rs              (new: sequential + parallel loops)
│   ├── progress.rs              (new: indicatif wrapper, TTY-gated)
│   └── summary.rs               (new: end-of-run line)
├── clean_bin/                   (unchanged)
├── ls_bin/                      (unchanged)
├── sync_bin/                    (unchanged)
└── util_bin/                    (unchanged)
```

`batch_run/` lives at the top level of `src/` — it is new code, not
vendored from any upstream library, so it sits alongside the
`*_bin/` vendored trees rather than inside one of them.

## 7. Dependencies

New, all small, no significant transitive deps:

- `shlex = "1"` — line tokenization
- `is-terminal = "0.4"` — TTY detection for progress bar gating
- `num_cpus = "1"` — `--parallel 0` resolution

Already present and reused: `indicatif`, `tokio`, `clap`,
`tracing`, `anyhow`, `async-channel`.

## 8. Testing

### 8.1 Unit tests (in-module, `#[cfg(test)]`)

- `parser.rs`: shlex tokenization (quotes, escapes, unicode, blanks,
  `#` comments, empty stdin, malformed quotes).
- `validate.rs`: each rejection rule (nested batch-run, stdio cp
  source, stdio cp target, every tracing flag variant). Table-driven.
- `executor.rs`: sequential happy path, sequential fail-fast (with
  and without `--continue-on-error`), parallel ordering invariants,
  parallel cancel-doesn't-kill-in-flight (use a fake dispatch that
  sleeps then completes), parallel `--parallel 0` resolves to
  `num_cpus::get()`.
- `progress.rs`: bar disabled when `is_terminal=false`, bar disabled
  when `--quiet`, tick math.
- `summary.rs`: counter formatting for ok/failed/skipped/elapsed.

### 8.2 Integration tests

Add to existing files:
- `tests/cli_routing.rs` — `parses_batch_run_with_no_flags`,
  `parses_batch_run_with_parallel_streaming_continue`.
- `tests/cli_help.rs` — `batch_run_help_works`,
  top-level help lists `batch-run`.
- `tests/cli_arg_validation.rs` — `--parallel must be >= 0`, etc.

New `tests/batch_run.rs` — end-to-end via `assert_cmd::Command`:
feed stdin, assert exit code, assert summary line, assert banner
format. No real S3 calls; uses commands that fail at
config-validation time or against a locally-resolvable failure.
**Not gated on `cfg(e2e_test)`** — runs in normal `cargo test`.

### 8.3 Coverage target

Same bar as the rest of s7cmd (96%+). Per-subcommand `dispatch`
arms inherit existing coverage from the vendored library tests; the
new `batch_run/` module is the focus for new coverage.

### 8.4 e2e

No e2e tests are added by this design. Per `CLAUDE.md`, e2e tests
are for the user to write and run manually against real AWS S3.

## 9. Documentation

- `README.md`: usage block — add `batch-run` under a new "Batch"
  heading group, matching the listing order in `cli.rs`.
- `CHANGELOG.md`: entry for the next release describing the new
  subcommand.
- `src/cli.rs`: help template — add the "Batch" heading group.

No new top-level documents.

## 10. Risks and open items

- **Tracing struct shape (5.2).** Resolved during implementation;
  doesn't affect the design.
- **MultiProgress + tracing log lines.** If tracing is enabled and
  tracing logs go to stderr, the progress bar may interleave with
  log lines. `indicatif`'s `LogWrapper` solves this; we'll wire it up
  in implementation if tracing is configured.
- **Streaming + parallel.** Streaming mode with `--parallel N>1` is
  legal and useful (long-running pipelines). The executor handles it
  via the same JoinSet+semaphore loop, fed by the streaming channel
  instead of a Vec. No design difference.
- **Lines-from-stdin and SIGINT.** If the user pipes a slow producer
  into `batch-run --streaming` and hits Ctrl-C, the read loop must
  drop the channel cleanly so the producer's write side errors out
  rather than hanging. Use `select!` on the channel and the cancel
  token in the read loop.
