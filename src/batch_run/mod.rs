//! `batch-run` subcommand.

use anyhow::Result;
use clap::Parser;
use std::io::BufRead;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub mod args;
pub mod executor;
pub mod parser;
pub mod progress;
pub mod summary;
pub mod validate;

use crate::cli::{BatchRunArgs, Cli};
use executor::{DispatchFn, ExecutorOptions, Interrupt, PreparedLine, resolve_workers};

pub async fn run(args: BatchRunArgs) -> i32 {
    if args.check_format {
        run_check_format(args)
    } else if args.streaming {
        run_streaming(args).await
    } else {
        run_read_all(args).await
    }
}

/// `--check-format` mode. Walk the script line by line; the first
/// tokenize / clap-parse / validate failure (or read I/O error) is
/// reported at error level and the process returns 1 immediately —
/// no further lines are inspected and no command is executed. On a
/// clean walk, log a single info-level success message and return 0.
fn run_check_format(args: BatchRunArgs) -> i32 {
    let has_issue = if args.script == "-" {
        let stdin = std::io::stdin();
        check_format_lines(stdin.lock())
    } else {
        match std::fs::File::open(&args.script) {
            Ok(f) => check_format_lines(std::io::BufReader::new(f)),
            Err(e) => {
                tracing::error!("{}: {e}", args.script);
                return 1;
            }
        }
    };
    if has_issue {
        1
    } else {
        tracing::info!("batch-run: format OK ({}).", args.script);
        0
    }
}

/// Walk lines and stop at the first problem. Returns `true` if an issue
/// was reported (via `tracing::error!`), `false` if every line passed.
fn check_format_lines<R: BufRead>(mut reader: R) -> bool {
    let mut line_no: usize = 0;
    loop {
        line_no += 1;
        let line = match parser::read_line_capped(&mut reader) {
            Ok(Some(s)) => s,
            Ok(None) => return false,
            Err(e) => {
                tracing::error!("line {line_no}: read error: {e}");
                return true;
            }
        };
        let argv = match parser::tokenize_line(&line) {
            Ok(Some(a)) => a,
            Ok(None) => continue, // blank or comment
            Err(e) => {
                tracing::error!("line {line_no}: parse error: {e}: {}", line.trim_end());
                return true;
            }
        };
        let cli = match Cli::try_parse_from(&argv) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(
                    "line {line_no}: parse error: {}: {}",
                    flatten(&e.to_string()),
                    line.trim_end()
                );
                return true;
            }
        };
        let cmd = match cli.command {
            Some(c) => c,
            None => {
                tracing::error!("line {line_no}: empty command: {}", line.trim_end());
                return true;
            }
        };
        if let Err(e) = validate::validate(line_no, &line, &cmd) {
            // validate already includes the line number and a `> raw`
            // tail; flatten to keep one log entry per line.
            tracing::error!("{}", flatten(&e.to_string()));
            return true;
        }
    }
}

/// Collapse a multi-line error message into a single line so each entry
/// in the error log occupies exactly one line. clap errors in
/// particular span several lines (`error: ...` plus usage / help).
fn flatten(msg: &str) -> String {
    msg.lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
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
                eprintln!("batch-run: {e}");
                return 1;
            }
        }
    } else {
        let file = match std::fs::File::open(&args.script) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("batch-run: {}: {e}", args.script);
                return 1;
            }
        };
        match parser::read_all(std::io::BufReader::new(file)) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("batch-run: {e}");
                return 1;
            }
        }
    };

    // Phase 2 - parse and validate every line. Still under default SIGINT.
    let prepared = match parse_and_validate(parsed) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("batch-run: {e}");
            return 1;
        }
    };

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
        no_summary: args.no_summary,
        streaming: args.streaming,
    };

    let dispatch: DispatchFn =
        Arc::new(|cmd| Box::pin(async move { crate::dispatch::dispatch(cmd).await }));

    let (code, summary) = if workers == 1 {
        executor::run_sequential(prepared, opts, dispatch, interrupt).await
    } else {
        executor::run_parallel(prepared, opts, dispatch, interrupt).await
    };

    if !args.no_summary {
        eprintln!("{}", summary.format());
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
        no_summary: args.no_summary,
        streaming: args.streaming,
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
    let reader_handle: tokio::task::JoinHandle<Result<()>> = if args.script == "-" {
        let reader = tokio::io::BufReader::new(tokio::io::stdin());
        tokio::spawn(async move { streaming_reader(reader, tx, interrupt_for_reader).await })
    } else {
        match tokio::fs::File::open(&args.script).await {
            Ok(f) => {
                let reader = tokio::io::BufReader::new(f);
                tokio::spawn(
                    async move { streaming_reader(reader, tx, interrupt_for_reader).await },
                )
            }
            Err(e) => {
                eprintln!("batch-run: {}: {e}", args.script);
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
            eprintln!("batch-run: {e}");
            code.max(1)
        }
        Err(e) => {
            eprintln!("batch-run: reader task panicked: {e}");
            code.max(1)
        }
    };

    if !args.no_summary {
        eprintln!("{}", summary.format());
    }

    final_code
}

/// Async reader loop for streaming mode. Reads the script line by line,
/// tokenizes / parses / validates each line, and pushes a `PreparedLine`
/// onto the channel. Returns:
///   - `Ok(())` on EOF, on Ctrl-C (interrupt set), or when the receiver
///     has been dropped.
///   - `Err(_)` on parse / validate / I/O error (the caller prints it
///     and bumps the exit code to >= 1).
async fn streaming_reader<R>(
    mut reader: R,
    tx: tokio::sync::mpsc::UnboundedSender<PreparedLine>,
    interrupt: Interrupt,
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
                            "stdin read error at line {line_no}: {e}"
                        ));
                    }
                };

                // Tokenize (skips blank/comment).
                let argv = match parser::tokenize_line(&text) {
                    Ok(Some(a)) => a,
                    Ok(None) => continue,
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "parse error at line {line_no}: {e}\n  > {text}"
                        ));
                    }
                };

                // Parse via clap.
                let cli = Cli::try_parse_from(&argv).map_err(|e| {
                    anyhow::anyhow!("line {line_no}: parse error: {e}\n  > {text}")
                })?;
                let cmd = cli.command.ok_or_else(|| {
                    anyhow::anyhow!("line {line_no}: empty command\n  > {text}")
                })?;

                // Validate.
                validate::validate(line_no, &text, &cmd)?;

                let prepared = PreparedLine {
                    line_no,
                    raw: text,
                    cmd,
                };

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

fn parse_and_validate(parsed: Vec<parser::ParsedLine>) -> Result<Vec<PreparedLine>> {
    let mut prepared = Vec::with_capacity(parsed.len());
    for line in parsed {
        let cli = Cli::try_parse_from(&line.argv).map_err(|e| {
            anyhow::anyhow!(
                "line {}: parse error: {}\n  > {}",
                line.line_no,
                e,
                line.raw
            )
        })?;
        let cmd = cli.command.ok_or_else(|| {
            anyhow::anyhow!("line {}: empty command\n  > {}", line.line_no, line.raw)
        })?;
        validate::validate(line.line_no, &line.raw, &cmd)?;
        prepared.push(PreparedLine {
            line_no: line.line_no,
            raw: line.raw,
            cmd,
        });
    }
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::ParsedLine;

    fn pl(line_no: usize, raw: &str, argv: &[&str]) -> ParsedLine {
        ParsedLine {
            line_no,
            raw: raw.to_string(),
            argv: argv.iter().map(|s| s.to_string()).collect(),
        }
    }

    // ---- parse_and_validate ----

    fn ok(r: Result<Vec<PreparedLine>>) -> Vec<PreparedLine> {
        match r {
            Ok(v) => v,
            Err(e) => panic!("expected Ok, got Err: {e:#}"),
        }
    }

    fn err(r: Result<Vec<PreparedLine>>) -> anyhow::Error {
        match r {
            Ok(_) => panic!("expected Err, got Ok"),
            Err(e) => e,
        }
    }

    #[test]
    fn parse_and_validate_empty_input_returns_empty() {
        let out = ok(parse_and_validate(Vec::new()));
        assert!(out.is_empty());
    }

    #[test]
    fn parse_and_validate_success_for_one_line() {
        let parsed = vec![pl(
            1,
            "head-bucket s3://bucket",
            &["s7cmd", "head-bucket", "s3://bucket"],
        )];
        let out = ok(parse_and_validate(parsed));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].line_no, 1);
        assert_eq!(out[0].raw, "head-bucket s3://bucket");
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
        let out = ok(parse_and_validate(parsed));
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].line_no, 1);
        assert_eq!(out[1].line_no, 3);
    }

    #[test]
    fn parse_and_validate_parse_error_includes_line_number_and_raw() {
        // Unknown subcommand → clap parse error.
        let parsed = vec![pl(7, "no-such-command", &["s7cmd", "no-such-command"])];
        let e = err(parse_and_validate(parsed));
        let msg = e.to_string();
        assert!(msg.contains("line 7"), "msg: {msg}");
        assert!(msg.contains("parse error"), "msg: {msg}");
        assert!(msg.contains("no-such-command"), "msg: {msg}");
    }

    #[test]
    fn parse_and_validate_empty_command_errors_with_line_number() {
        // `--auto-complete-shell` is a top-level flag that doesn't require
        // a subcommand, so clap parses successfully but `cli.command` is
        // `None` — this exercises the `ok_or_else` branch.
        let parsed = vec![pl(
            2,
            "--auto-complete-shell bash",
            &["s7cmd", "--auto-complete-shell", "bash"],
        )];
        let e = err(parse_and_validate(parsed));
        let msg = e.to_string();
        assert!(msg.contains("line 2"), "msg: {msg}");
        assert!(msg.contains("empty command"), "msg: {msg}");
    }

    #[test]
    fn parse_and_validate_validate_error_propagates() {
        // Nested batch-run is rejected by validate::validate. The script
        // positional is required at parse time, so include `-` to reach
        // the validate step.
        let parsed = vec![pl(5, "batch-run -", &["s7cmd", "batch-run", "-"])];
        let e = err(parse_and_validate(parsed));
        let msg = e.to_string();
        assert!(msg.contains("line 5"), "msg: {msg}");
        assert!(msg.contains("nested batch-run"), "msg: {msg}");
    }

    #[test]
    fn parse_and_validate_stops_at_first_error() {
        // Two lines: first valid, second invalid → must error on the
        // second one (indicating loop continues past the first).
        let parsed = vec![
            pl(
                1,
                "head-bucket s3://b1",
                &["s7cmd", "head-bucket", "s3://b1"],
            ),
            pl(2, "batch-run -", &["s7cmd", "batch-run", "-"]),
        ];
        let e = err(parse_and_validate(parsed));
        assert!(e.to_string().contains("line 2"));
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
        assert!(!check_format_lines(input));
    }

    #[test]
    fn check_format_lines_only_blank_and_comment_is_clean() {
        let input: &[u8] = b"\n# comment\n   \n";
        assert!(!check_format_lines(input));
    }

    #[test]
    fn check_format_lines_valid_commands_is_clean() {
        let input: &[u8] = b"head-bucket s3://b1\ncreate-bucket --dry-run s3://b2\n";
        assert!(!check_format_lines(input));
    }

    #[test]
    fn check_format_lines_tokenize_error_returns_true() {
        // Unbalanced quote → shlex rejects.
        let input: &[u8] = b"cp \"oops\n";
        assert!(check_format_lines(input));
    }

    #[test]
    fn check_format_lines_clap_parse_error_returns_true() {
        let input: &[u8] = b"no-such-command\n";
        assert!(check_format_lines(input));
    }

    #[test]
    fn check_format_lines_validate_error_returns_true() {
        // Nested batch-run rejected by validate.
        let input: &[u8] = b"batch-run -\n";
        assert!(check_format_lines(input));
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
            check_format_lines(&mut cursor),
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
        assert!(check_format_lines(s.as_bytes()));
    }
}
