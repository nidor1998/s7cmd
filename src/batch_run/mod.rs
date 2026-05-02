//! `batch-run` subcommand.

use anyhow::Result;
use clap::FromArgMatches;
use std::io::BufRead;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub mod args;
pub mod executor;
pub mod parser;
pub mod progress;
pub mod summary;
pub mod validate;

use crate::cli::{BatchRunArgs, Cli, cli_command};
use executor::{DispatchFn, ExecutorOptions, Interrupt, PreparedLine, resolve_workers};

/// Parse one batch-run line's argv into a `Cli`, using the same
/// post-processed `clap::Command` that `main.rs` uses at the top level.
///
/// `cli_command()` clears the long name on every subcommand's
/// `--auto-complete-shell` (the flag is inherited from upstream args
/// structs and is redundant with the top-level form). Going through it
/// here means per-line `cp --auto-complete-shell <SHELL>` etc. is rejected
/// at parse time with a clean "unexpected argument" error — instead of
/// accepting the line and panicking later when `Config::try_from` calls
/// `parse_storage_path("")` on the absent source positional.
///
/// Direct `Cli::try_parse_from` would skip that mutation and reintroduce
/// the panic. All batch-run line parsing should go through this helper.
fn parse_line_argv(argv: &[String]) -> clap::error::Result<Cli> {
    let matches = cli_command().try_get_matches_from(argv)?;
    Cli::from_arg_matches(&matches)
}

pub async fn run(args: BatchRunArgs) -> i32 {
    if args.check_format {
        run_check_format(args)
    } else if args.streaming {
        run_streaming(args).await
    } else {
        run_read_all(args).await
    }
}

/// Drain stdin to EOF without parsing it, so an upstream producer
/// (xargs, a shell loop, etc.) can finish its writes cleanly. If
/// `s7cmd` exits while bytes are still in flight on stdin, the
/// kernel breaks the pipe and the producer gets SIGPIPE on its next
/// write — visible to the user as
/// `xargs: terminated with signal 13`. Reading-and-discarding keeps
/// the pipe open until the producer is done; the data is never
/// parsed because by the time we drain we've already decided to exit
/// with an error.
///
/// Skipped when the script source is a file: stdin isn't the producer
/// in that case, and a real `std::io::copy` would block on a stdin
/// that happens to be connected to something (`< /dev/zero`, etc.).
fn drain_stdin_on_error_exit(args: &BatchRunArgs) {
    if args.script != "-" {
        return;
    }
    let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink());
}

/// `--check-format` mode. Walk the script line by line; the first
/// tokenize / clap-parse / validate failure (or read I/O error) is
/// reported at error level and the process returns 1 immediately —
/// no further lines are inspected and no command is executed. On a
/// clean walk, log a single info-level success message and return 0.
fn run_check_format(args: BatchRunArgs) -> i32 {
    let source = script_source_label(&args.script);
    let has_issue = if args.script == "-" {
        let stdin = std::io::stdin();
        check_format_lines(stdin.lock(), source)
    } else {
        match std::fs::File::open(&args.script) {
            Ok(f) => check_format_lines(std::io::BufReader::new(f), source),
            Err(e) => {
                tracing::error!("{source}: {e}");
                drain_stdin_on_error_exit(&args);
                return 1;
            }
        }
    };
    if has_issue {
        drain_stdin_on_error_exit(&args);
        1
    } else {
        tracing::info!("batch-run: format OK ({source}).");
        0
    }
}

/// Display label for the script source: `"stdin"` for `-`, the file
/// path otherwise. Used to prefix per-line check-format diagnostics so
/// the user can see at a glance which file is being checked — the
/// real-world failure mode is forgetting `-` and accidentally pointing
/// `batch-run` at a non-script file (`/etc/hosts`, etc.).
fn script_source_label(script: &str) -> &str {
    if script == "-" { "stdin" } else { script }
}

/// Walk lines and stop at the first problem. Returns `true` if an issue
/// was reported (via `tracing::error!`), `false` if every line passed.
/// Each per-line error is prefixed with `<source>: line N:` so the log
/// identifies *which* file the offending line is from.
fn check_format_lines<R: BufRead>(mut reader: R, source: &str) -> bool {
    let mut line_no: usize = 0;
    loop {
        line_no += 1;
        let line = match parser::read_line_capped(&mut reader) {
            Ok(Some(s)) => s,
            Ok(None) => return false,
            Err(e) => {
                tracing::error!("{source}: line {line_no}: read error: {e}");
                return true;
            }
        };
        let argv = match parser::tokenize_line(&line) {
            Ok(Some(a)) => a,
            Ok(None) => continue, // blank or comment
            Err(e) => {
                tracing::error!(
                    "{source}: line {line_no}: parse error: {e}: {}",
                    line.trim_end()
                );
                return true;
            }
        };
        let cli = match parse_line_argv(&argv) {
            Ok(c) => c,
            Err(e) => {
                let s = e.to_string();
                tracing::error!(
                    "{source}: line {line_no}: parse error: {}: {}",
                    clap_error_summary(&s),
                    line.trim_end()
                );
                return true;
            }
        };
        let cmd = match cli.command {
            Some(c) => c,
            None => {
                tracing::error!(
                    "{source}: line {line_no}: empty command: {}",
                    line.trim_end()
                );
                return true;
            }
        };
        if let Err(e) = validate::validate(line_no, &line, &cmd) {
            // validate's message already includes "line N: <description>"
            // plus a `> raw` tail; flatten and prefix with the source.
            tracing::error!("{source}: {}", flatten(&e.to_string()));
            return true;
        }
    }
}

/// Collapse a multi-line error message into a single line so each entry
/// in the error log occupies exactly one line. Used for `validate`
/// errors, whose follow-up lines (`  > <raw>`) carry useful context
/// worth keeping. clap errors take a different path — see
/// `clap_error_summary`.
fn flatten(msg: &str) -> String {
    msg.lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract the first non-empty line of a clap error and strip its
/// leading `error: ` prefix, returning a short single-line summary.
///
/// clap formats parse errors as a description line followed by blank-
/// separated `Usage: ...` and `For more information, try '--help'.`
/// blocks. Those trailers are noise in a per-line check-format report
/// (the user already saw the help when they invoked the binary), and
/// the leading `error: ` would otherwise appear right after our own
/// `parse error:` prefix.
fn clap_error_summary(msg: &str) -> &str {
    let first = msg
        .lines()
        .map(str::trim)
        .find(|s| !s.is_empty())
        .unwrap_or("parse error");
    first.strip_prefix("error: ").unwrap_or(first)
}

/// Resolve the failure-stop policy from the (mutually exclusive)
/// `--continue-on-error` and `--max-errors` flags:
///   - `--continue-on-error` → `None` (run every line, never stop on
///     failures).
///   - `--max-errors N`     → `Some(N)` (stop after `N` failures).
///   - neither flag         → `Some(1)` (stop on the first failure —
///     the historical default).
///
/// clap rejects the conflicting combination at parse time, so this
/// function never has to break the tie.
fn error_threshold(args: &BatchRunArgs) -> Option<u64> {
    if args.continue_on_error {
        None
    } else {
        Some(args.max_errors.unwrap_or(1))
    }
}

/// Read-all (default) mode. Read the whole script first (file or stdin),
/// parse and validate every line, then install the SIGINT listener and
/// execute. Original behavior preserved when the script is `-`.
async fn run_read_all(args: BatchRunArgs) -> i32 {
    // Phase 1 - read the script. Default SIGINT behavior applies here, so
    // Ctrl-C immediately kills the process (matching user intuition for
    // "I haven't started anything yet").
    let parsed = if args.script == "-" {
        let stdin = std::io::stdin();
        match parser::read_all(stdin.lock()) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("{e}");
                return 1;
            }
        }
    } else {
        let file = match std::fs::File::open(&args.script) {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("{}: {e}", args.script);
                return 1;
            }
        };
        match parser::read_all(std::io::BufReader::new(file)) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("{e}");
                return 1;
            }
        }
    };

    // Phase 2 - parse and validate every line. Still under default SIGINT.
    // Per-line failures (tokenize / clap-parse / empty / validate) are
    // turned into `PreparedLineKind::Invalid` entries here and become
    // exit-2 failures inside the executor — they NO LONGER abort the
    // whole run, so `--max-errors` and `--continue-on-error` apply to
    // them like any other failure.
    let prepared = parse_and_validate(parsed);

    // Phase 3 - install the SIGINT listener. From this point on, Ctrl-C
    // sets the shared flag the executors check before each new spawn;
    // in-flight commands cancel via their own per-subcommand cancellation
    // handlers (registered inside each dispatched command).
    let interrupt: Interrupt = Arc::new(AtomicBool::new(false));
    {
        let interrupt = Arc::clone(&interrupt);
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                interrupt.store(true, Ordering::SeqCst);
            }
        });
    }

    let workers = resolve_workers(args.parallel);
    let opts = ExecutorOptions {
        workers,
        error_threshold: error_threshold(&args),
        continue_on_warning: args.continue_on_warning,
        streaming: args.streaming,
        // `--json-tracing` implies machine-readable output — the live
        // progress bar would be visual noise interleaved with JSON, so
        // suppress it just like `--no-progress` does.
        no_progress: args.no_progress || args.json_tracing,
    };

    let dispatch: DispatchFn =
        Arc::new(|cmd| Box::pin(async move { crate::dispatch::dispatch(cmd).await }));

    let (code, summary) = if workers == 1 {
        executor::run_sequential(prepared, opts, dispatch, interrupt).await
    } else {
        executor::run_parallel(prepared, opts, dispatch, interrupt).await
    };

    if !args.no_summary {
        eprintln!("{}", format_summary(&summary, args.json_tracing));
    }

    code
}

/// Streaming mode. Read and execute lines concurrently. The SIGINT
/// listener is installed BEFORE script reading because read and execute
/// are interleaved — Ctrl-C must abort both halves of the pipeline.
async fn run_streaming(args: BatchRunArgs) -> i32 {
    // Install SIGINT listener early — it sets the shared flag the reader
    // and executor both check.
    let interrupt: Interrupt = Arc::new(AtomicBool::new(false));
    {
        let interrupt = Arc::clone(&interrupt);
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                interrupt.store(true, Ordering::SeqCst);
            }
        });
    }

    let workers = resolve_workers(args.parallel);
    let opts = ExecutorOptions {
        workers,
        error_threshold: error_threshold(&args),
        continue_on_warning: args.continue_on_warning,
        streaming: args.streaming,
        // See `run_read_all` for the `--json-tracing` rationale.
        no_progress: args.no_progress || args.json_tracing,
    };
    let dispatch: DispatchFn =
        Arc::new(|cmd| Box::pin(async move { crate::dispatch::dispatch(cmd).await }));

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<PreparedLine>();

    // Spawn reader task. It reads the script line-by-line, tokenizes/
    // parses/validates each, and forwards a `PreparedLine` on the
    // channel. On EOF, parse error, validate error, ctrl-c, or receiver-
    // dropped, the task returns. File-open failure is reported here
    // before any executor work and short-circuits the run.
    let interrupt_for_reader = Arc::clone(&interrupt);
    let source = script_source_label(&args.script).to_string();
    let reader_handle: tokio::task::JoinHandle<Result<()>> = if args.script == "-" {
        let reader = tokio::io::BufReader::new(tokio::io::stdin());
        tokio::spawn(
            async move { streaming_reader(reader, tx, interrupt_for_reader, source).await },
        )
    } else {
        match tokio::fs::File::open(&args.script).await {
            Ok(f) => {
                let reader = tokio::io::BufReader::new(f);
                tokio::spawn(async move {
                    streaming_reader(reader, tx, interrupt_for_reader, source).await
                })
            }
            Err(e) => {
                tracing::error!("{}: {e}", args.script);
                return 1;
            }
        }
    };

    // Drive the executor concurrently with the reader. The executor
    // pulls from `rx`; when the reader drops `tx` the channel closes,
    // which the executor uses as the "no more lines" signal.
    let (code, summary) = if workers == 1 {
        executor::run_sequential_streaming(rx, opts, dispatch, interrupt).await
    } else {
        executor::run_parallel_streaming(rx, opts, dispatch, interrupt).await
    };

    // Wait for the reader to finish (it may still be holding on a slow
    // pipe; if the executor finished early, the reader will exit on its
    // next `tx.send` because the receiver is gone).
    let final_code = match reader_handle.await {
        Ok(Ok(())) => code,
        Ok(Err(e)) => {
            tracing::error!("{}", flatten(&e.to_string()));
            code.max(1)
        }
        Err(e) => {
            tracing::error!("reader task panicked: {e}");
            code.max(1)
        }
    };

    if !args.no_summary {
        eprintln!("{}", format_summary(&summary, args.json_tracing));
    }

    final_code
}

/// Pick the summary representation: human-readable line by default,
/// JSON object when `--json-tracing` is set on `batch-run`.
fn format_summary(summary: &summary::Summary, json_tracing: bool) -> String {
    if json_tracing {
        summary.format_json()
    } else {
        summary.format()
    }
}

/// Async reader loop for streaming mode. Reads the script line by line,
/// tokenizes / parses / validates each line, and pushes a `PreparedLine`
/// onto the channel. Per-line tokenize / clap-parse / empty / validate
/// failures are pushed as `PreparedLineKind::Invalid` so the executor
/// can count them toward `--max-errors` like any other failure — they
/// do NOT short-circuit the reader. Returns:
///   - `Ok(())` on EOF, on Ctrl-C (interrupt set), or when the receiver
///     has been dropped.
///   - `Err(_)` only on a true read I/O error (file unreadable, line
///     cap exceeded, non-UTF-8). The caller prints it and bumps the
///     exit code to >= 1.
async fn streaming_reader<R>(
    mut reader: R,
    tx: tokio::sync::mpsc::UnboundedSender<PreparedLine>,
    interrupt: Interrupt,
    source: String,
) -> Result<()>
where
    R: tokio::io::AsyncBufRead + Unpin,
{
    let mut line_no: usize = 0;

    loop {
        tokio::select! {
            biased;
            _ = tokio::signal::ctrl_c() => {
                interrupt.store(true, Ordering::SeqCst);
                return Ok(());
            }
            result = read_line_capped_async(&mut reader) => {
                line_no += 1;
                let text = match result {
                    Ok(Some(t)) => t,
                    Ok(None) => return Ok(()), // EOF
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "{source} read error at line {line_no}: {e}"
                        ));
                    }
                };

                // Tokenize (skips blank/comment). Tokenize errors flow
                // through as a `TokenizeError` ParsedLine kind so they
                // become exit-2 Invalid lines downstream rather than
                // aborting the reader.
                let parsed_kind = match parser::tokenize_line(&text) {
                    Ok(None) => continue, // blank / comment
                    Ok(Some(argv)) => parser::ParsedLineKind::Ok(argv),
                    Err(e) => parser::ParsedLineKind::TokenizeError(format!(
                        "line {line_no}: parse error: {e}: {}",
                        text.trim_end()
                    )),
                };

                let prepared = try_prepare(parser::ParsedLine {
                    line_no,
                    raw: text,
                    kind: parsed_kind,
                });

                if tx.send(prepared).is_err() {
                    // Receiver dropped; executor finished early.
                    return Ok(());
                }
            }
        }
    }
}

/// Async equivalent of `parser::read_line_capped`. Reads one line from
/// `reader`, capped at `parser::MAX_LINE_LEN` bytes (excluding the `\n`).
/// Returns `Ok(None)` only at EOF before any byte was read; otherwise
/// `Ok(Some(line))` (newline stripped) or `Err` on cap-exceeded /
/// non-UTF-8 / I/O error.
async fn read_line_capped_async<R>(reader: &mut R) -> std::io::Result<Option<String>>
where
    R: tokio::io::AsyncBufRead + Unpin,
{
    use tokio::io::AsyncBufReadExt;

    let mut buf: Vec<u8> = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return if buf.is_empty() {
                Ok(None)
            } else {
                String::from_utf8(buf)
                    .map(Some)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            };
        }
        match available.iter().position(|&b| b == b'\n') {
            Some(idx) => {
                buf.extend_from_slice(&available[..idx]);
                reader.consume(idx + 1);
                if buf.len() > parser::MAX_LINE_LEN {
                    return Err(parser::too_long_err());
                }
                return String::from_utf8(buf)
                    .map(Some)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e));
            }
            None => {
                buf.extend_from_slice(available);
                let n = available.len();
                reader.consume(n);
                if buf.len() > parser::MAX_LINE_LEN {
                    return Err(parser::too_long_err());
                }
            }
        }
    }
}

/// Convert one `ParsedLine` into a `PreparedLine`. Tokenize / clap-parse
/// / empty-command / validate failures all become `PreparedLineKind::Invalid`
/// carrying a single-line error message; only successful clap parses with
/// a present subcommand and clean validation become `PreparedLineKind::Cmd`.
/// Per-line failures are NEVER fatal here — the executor schedules
/// `Invalid` lines as exit-2 failures so they count toward `--max-errors`
/// like any runtime failure.
fn try_prepare(parsed: parser::ParsedLine) -> PreparedLine {
    let parser::ParsedLine { line_no, raw, kind } = parsed;
    let kind = match build_prepared_kind(line_no, &raw, kind) {
        Ok(k) => k,
        Err(message) => executor::PreparedLineKind::Invalid(message),
    };
    PreparedLine { line_no, raw, kind }
}

/// `Ok(Cmd(_))` on a successfully parsed + validated line; `Err(msg)`
/// otherwise. The error string is single-line, already prefixed with
/// `line N:` and suffixed with the offending raw line, suitable for
/// direct emission via `tracing::error!`.
fn build_prepared_kind(
    line_no: usize,
    raw: &str,
    parsed_kind: parser::ParsedLineKind,
) -> std::result::Result<executor::PreparedLineKind, String> {
    let argv = match parsed_kind {
        parser::ParsedLineKind::TokenizeError(msg) => return Err(msg),
        parser::ParsedLineKind::Ok(argv) => argv,
    };
    let cli = parse_line_argv(&argv).map_err(|e| {
        // Strip clap's `Usage:` / `For more information…` trailers and
        // its leading `error:` prefix so the message stays single-line —
        // same approach as `check_format_lines`.
        let s = e.to_string();
        format!(
            "line {line_no}: parse error: {}: {}",
            clap_error_summary(&s),
            raw.trim_end()
        )
    })?;
    let cmd = cli
        .command
        .ok_or_else(|| format!("line {line_no}: empty command: {}", raw.trim_end()))?;
    validate::validate(line_no, raw, &cmd).map_err(|e| flatten(&e.to_string()))?;
    Ok(executor::PreparedLineKind::Cmd(Box::new(cmd)))
}

fn parse_and_validate(parsed: Vec<parser::ParsedLine>) -> Vec<PreparedLine> {
    parsed.into_iter().map(try_prepare).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::ParsedLine;

    fn pl(line_no: usize, raw: &str, argv: &[&str]) -> ParsedLine {
        ParsedLine {
            line_no,
            raw: raw.to_string(),
            kind: parser::ParsedLineKind::Ok(argv.iter().map(|s| s.to_string()).collect()),
        }
    }

    fn pl_tokenize_err(line_no: usize, raw: &str, message: &str) -> ParsedLine {
        ParsedLine {
            line_no,
            raw: raw.to_string(),
            kind: parser::ParsedLineKind::TokenizeError(message.to_string()),
        }
    }

    // ---- parse_and_validate ----
    //
    // Per-line failures are now `PreparedLineKind::Invalid` entries
    // rather than an early `Err` return — the function never bails
    // mid-Vec. Helpers that classify the result by variant.

    fn assert_invalid(line: &PreparedLine, must_contain: &[&str]) {
        match &line.kind {
            executor::PreparedLineKind::Invalid(msg) => {
                for needle in must_contain {
                    assert!(msg.contains(needle), "expected {needle:?} in {msg:?}");
                }
            }
            executor::PreparedLineKind::Cmd(_) => {
                panic!("expected Invalid, got Cmd for line {}", line.line_no)
            }
        }
    }

    fn assert_cmd(line: &PreparedLine) {
        assert!(
            matches!(line.kind, executor::PreparedLineKind::Cmd(_)),
            "expected Cmd for line {}",
            line.line_no
        );
    }

    #[test]
    fn parse_and_validate_empty_input_returns_empty() {
        let out = parse_and_validate(Vec::new());
        assert!(out.is_empty());
    }

    #[test]
    fn parse_and_validate_success_for_one_line() {
        let parsed = vec![pl(
            1,
            "head-bucket s3://bucket",
            &["s7cmd", "head-bucket", "s3://bucket"],
        )];
        let out = parse_and_validate(parsed);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].line_no, 1);
        assert_eq!(out[0].raw, "head-bucket s3://bucket");
        assert_cmd(&out[0]);
    }

    #[test]
    fn parse_and_validate_success_for_multiple_lines() {
        let parsed = vec![
            pl(
                1,
                "head-bucket s3://b1",
                &["s7cmd", "head-bucket", "s3://b1"],
            ),
            pl(
                3,
                "head-bucket s3://b2",
                &["s7cmd", "head-bucket", "s3://b2"],
            ),
        ];
        let out = parse_and_validate(parsed);
        assert_eq!(out.len(), 2);
        assert_cmd(&out[0]);
        assert_cmd(&out[1]);
    }

    #[test]
    fn parse_and_validate_parse_error_becomes_invalid_with_line_number_and_raw() {
        // Unknown subcommand → clap parse error → `Invalid` PreparedLine.
        let parsed = vec![pl(7, "no-such-command", &["s7cmd", "no-such-command"])];
        let out = parse_and_validate(parsed);
        assert_eq!(out.len(), 1);
        assert_invalid(&out[0], &["line 7", "parse error", "no-such-command"]);
    }

    /// Per-line `cp --auto-complete-shell <SHELL>` previously panicked
    /// inside `s3util_rs::Config::try_from` because clap accepted the
    /// flag (inherited from upstream `CommonTransferArgs`) with `source` /
    /// `target` left as `None`, and downstream `parse_storage_path("")`
    /// did `unwrap()` on an `Err`. Going through `parse_line_argv` (which
    /// uses `cli_command()`) clears the per-subcommand long name so clap
    /// rejects the flag at parse time. Same behavior the top-level binary
    /// already had — batch-run was the inconsistency.
    #[test]
    fn parse_and_validate_rejects_per_line_auto_complete_shell_on_cp() {
        let parsed = vec![pl(
            4,
            "cp --auto-complete-shell fish",
            &["s7cmd", "cp", "--auto-complete-shell", "fish"],
        )];
        let out = parse_and_validate(parsed);
        assert_invalid(&out[0], &["line 4", "parse error", "auto-complete-shell"]);
    }

    #[test]
    fn parse_and_validate_empty_command_becomes_invalid() {
        // `--auto-complete-shell` is a top-level flag that doesn't require
        // a subcommand, so clap parses successfully but `cli.command` is
        // `None` — this exercises the `ok_or_else` branch.
        let parsed = vec![pl(
            2,
            "--auto-complete-shell bash",
            &["s7cmd", "--auto-complete-shell", "bash"],
        )];
        let out = parse_and_validate(parsed);
        assert_invalid(&out[0], &["line 2", "empty command"]);
    }

    #[test]
    fn parse_and_validate_validate_error_becomes_invalid() {
        // Nested batch-run is rejected by validate::validate. The script
        // positional is required at parse time, so include `-` to reach
        // the validate step.
        let parsed = vec![pl(5, "batch-run -", &["s7cmd", "batch-run", "-"])];
        let out = parse_and_validate(parsed);
        assert_invalid(&out[0], &["line 5", "nested batch-run"]);
    }

    #[test]
    fn parse_and_validate_continues_past_per_line_errors() {
        // Two lines: first valid, second invalid. Both must appear in
        // the output (Cmd then Invalid) — proving the loop no longer
        // bails at the first per-line failure. This is what makes
        // `--max-errors` apply to parse failures.
        let parsed = vec![
            pl(
                1,
                "head-bucket s3://b1",
                &["s7cmd", "head-bucket", "s3://b1"],
            ),
            pl(2, "batch-run -", &["s7cmd", "batch-run", "-"]),
        ];
        let out = parse_and_validate(parsed);
        assert_eq!(out.len(), 2);
        assert_cmd(&out[0]);
        assert_invalid(&out[1], &["line 2", "nested batch-run"]);
    }

    #[test]
    fn parse_and_validate_propagates_tokenize_error_kind_as_invalid() {
        // `parser::read_all` now surfaces tokenize failures as
        // `TokenizeError` ParsedLineKind. Verify `parse_and_validate`
        // converts that 1:1 into an `Invalid` PreparedLine carrying
        // the same message — no re-formatting, just a passthrough.
        let parsed = vec![pl_tokenize_err(
            9,
            "cp \"unterminated",
            "line 9: parse error: malformed quoting: cp \"unterminated",
        )];
        let out = parse_and_validate(parsed);
        assert_invalid(&out[0], &["line 9", "parse error", "malformed"]);
    }

    // ---- run / run_read_all / run_streaming with empty stdin ----
    // NOTE: tests that drive `run(...)` or `streaming_reader(...)` directly
    // are intentionally omitted. Both read from `tokio::io::stdin()`, which
    // cannot be redirected from inside the test process — with an interactive
    // terminal (the default for `cargo test`), they hang indefinitely instead
    // of receiving EOF. Coverage of those paths is provided by the
    // process-level integration tests in `tests/batch_run.rs`, which can
    // pipe a closed stdin via `Stdio::piped()`.

    // ---- read_line_capped_async ----
    //
    // Drive the async helper directly with an in-memory `&[u8]` (which
    // implements `tokio::io::AsyncBufRead` via `BufReader`) — this avoids
    // the stdin-hang problem above and exercises the same cap logic that
    // streaming_reader relies on.

    use parser::MAX_LINE_LEN;

    #[tokio::test]
    async fn capped_async_returns_none_at_eof() {
        let buf: &[u8] = b"";
        let mut r = tokio::io::BufReader::new(buf);
        assert!(read_line_capped_async(&mut r).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn capped_async_handles_final_line_without_newline() {
        let buf: &[u8] = b"hello";
        let mut r = tokio::io::BufReader::new(buf);
        assert_eq!(
            read_line_capped_async(&mut r).await.unwrap().unwrap(),
            "hello"
        );
        assert!(read_line_capped_async(&mut r).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn capped_async_accepts_line_at_max_len() {
        let mut s = String::with_capacity(MAX_LINE_LEN + 1);
        s.extend(std::iter::repeat_n('x', MAX_LINE_LEN));
        s.push('\n');
        let bytes = s.as_bytes();
        let mut r = tokio::io::BufReader::new(bytes);
        let line = read_line_capped_async(&mut r).await.unwrap().unwrap();
        assert_eq!(line.len(), MAX_LINE_LEN);
    }

    #[tokio::test]
    async fn capped_async_rejects_line_over_max_len() {
        let mut s = String::with_capacity(MAX_LINE_LEN + 2);
        s.extend(std::iter::repeat_n('x', MAX_LINE_LEN + 1));
        s.push('\n');
        let bytes = s.as_bytes();
        let mut r = tokio::io::BufReader::new(bytes);
        let err = read_line_capped_async(&mut r).await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("exceeds"));
    }

    #[tokio::test]
    async fn capped_async_rejects_long_line_without_terminator() {
        let s: String = std::iter::repeat_n('x', MAX_LINE_LEN + 1).collect();
        let bytes = s.as_bytes();
        let mut r = tokio::io::BufReader::new(bytes);
        let err = read_line_capped_async(&mut r).await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    /// `streaming_reader` is generic over `AsyncBufRead`; in production
    /// it's called with both `tokio::io::stdin()` and `tokio::fs::File`.
    /// On a read I/O error it must name the *actual* source (file path or
    /// "stdin"), not a hardcoded label. Drive it with an in-memory reader
    /// big enough to trip the line-length cap and assert the message
    /// carries the supplied source string.
    #[tokio::test]
    async fn streaming_reader_io_error_uses_source_label() {
        // MAX_LINE_LEN + 1 bytes, no newline → cap-exceeded I/O error.
        let s: String = std::iter::repeat_n('x', MAX_LINE_LEN + 1).collect();
        let reader = tokio::io::BufReader::new(s.as_bytes());
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<PreparedLine>();
        let interrupt: Interrupt = Arc::new(AtomicBool::new(false));
        let err = streaming_reader(reader, tx, interrupt, "/tmp/script.txt".to_string())
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("/tmp/script.txt read error"),
            "expected source-prefixed read-error message; got: {msg}"
        );
        assert!(msg.contains("line 1"), "msg: {msg}");
    }

    #[tokio::test]
    async fn capped_async_rejects_invalid_utf8() {
        let bytes: &[u8] = &[0xff, 0xfe, b'\n'];
        let mut r = tokio::io::BufReader::new(bytes);
        let err = read_line_capped_async(&mut r).await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    // ---- check_format_lines ----
    //
    // The function emits the first problem via `tracing::error!` and
    // returns a bool. We can verify the bool without a tracing
    // subscriber. Fail-fast behaviour and visible log output are
    // exercised by the process-level integration tests in
    // `tests/batch_run.rs`.

    #[test]
    fn check_format_lines_empty_input_is_clean() {
        let input: &[u8] = b"";
        assert!(!check_format_lines(input, "test"));
    }

    #[test]
    fn check_format_lines_only_blank_and_comment_is_clean() {
        let input: &[u8] = b"\n# comment\n   \n";
        assert!(!check_format_lines(input, "test"));
    }

    #[test]
    fn check_format_lines_valid_commands_is_clean() {
        let input: &[u8] = b"head-bucket s3://b1\ncreate-bucket --dry-run s3://b2\n";
        assert!(!check_format_lines(input, "test"));
    }

    #[test]
    fn check_format_lines_tokenize_error_returns_true() {
        // Unbalanced quote → shlex rejects.
        let input: &[u8] = b"cp \"oops\n";
        assert!(check_format_lines(input, "test"));
    }

    #[test]
    fn check_format_lines_clap_parse_error_returns_true() {
        let input: &[u8] = b"no-such-command\n";
        assert!(check_format_lines(input, "test"));
    }

    #[test]
    fn script_source_label_maps_dash_to_stdin() {
        assert_eq!(script_source_label("-"), "stdin");
    }

    #[test]
    fn script_source_label_passes_through_paths() {
        assert_eq!(script_source_label("/etc/hosts"), "/etc/hosts");
        assert_eq!(script_source_label("./script.txt"), "./script.txt");
    }

    #[test]
    fn clap_error_summary_strips_error_prefix_and_trailers() {
        let msg = "error: unrecognized subcommand '127.0.0.1'\n\
                   \n\
                   Usage: s7cmd [OPTIONS] [COMMAND]\n\
                   \n\
                   For more information, try '--help'.\n";
        assert_eq!(
            clap_error_summary(msg),
            "unrecognized subcommand '127.0.0.1'"
        );
    }

    #[test]
    fn clap_error_summary_handles_message_without_error_prefix() {
        let msg = "missing required argument <FILE>\n";
        assert_eq!(clap_error_summary(msg), "missing required argument <FILE>");
    }

    #[test]
    fn clap_error_summary_handles_empty_input() {
        assert_eq!(clap_error_summary(""), "parse error");
    }

    #[test]
    fn clap_error_summary_skips_leading_blank_lines() {
        let msg = "\n\nerror: bad thing\n\nUsage: ...";
        assert_eq!(clap_error_summary(msg), "bad thing");
    }

    #[test]
    fn check_format_lines_validate_error_returns_true() {
        // Nested batch-run rejected by validate.
        let input: &[u8] = b"batch-run -\n";
        assert!(check_format_lines(input, "test"));
    }

    #[test]
    fn check_format_lines_stops_at_first_error() {
        // First line is bad. The function must return true after that
        // line; later bad lines are not consumed. `Cursor<&[u8]>`
        // implements `BufRead` directly, so its position tracks exactly
        // what `check_format_lines` consumed (no read-ahead from a
        // wrapping `BufReader`).
        use std::io::{Cursor, Read};
        let payload = b"no-such-command\nbatch-run -\ncp s3://b/k -\n";
        let mut cursor = Cursor::new(&payload[..]);
        assert!(
            check_format_lines(&mut cursor, "test"),
            "first bad line must be reported"
        );
        let mut rest = String::new();
        cursor.read_to_string(&mut rest).unwrap();
        assert!(
            rest.contains("batch-run -"),
            "later lines must remain unread; rest={rest:?}"
        );
    }

    #[test]
    fn check_format_lines_read_error_returns_true() {
        // A line over the 16 KiB cap triggers a read I/O error.
        let mut s = String::with_capacity(parser::MAX_LINE_LEN + 3);
        s.push('#');
        s.extend(std::iter::repeat_n('x', parser::MAX_LINE_LEN));
        s.push('\n');
        assert!(check_format_lines(s.as_bytes(), "test"));
    }
}
