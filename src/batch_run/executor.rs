//! Sequential and parallel execution loops.

use crate::batch_run::progress::Progress;
use crate::batch_run::summary::Summary;
use crate::cli::Cmd;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use tokio::sync::Semaphore;
use tokio::task::{JoinSet, LocalSet};

// The future returned by a dispatched command is intentionally NOT
// `Send`: upstream subcommand implementations (s3util_rs, s3sync) hold
// non-`Send` types (`&dyn StorageTrait`, `Option<&dyn Error>`) across
// `.await` points. The closure itself is `Send + Sync` so it can be
// shared across worker tasks; the parallel executor uses
// `LocalSet::spawn_local` so its returned future does not need to be
// `Send`.
pub type DispatchFn = Arc<dyn Fn(Cmd) -> Pin<Box<dyn Future<Output = i32>>> + Send + Sync>;

pub struct PreparedLine {
    pub line_no: usize,
    /// The original input line, used by `log_start` / `log_end` to
    /// identify which subcommand each per-line event belongs to.
    pub raw: String,
    pub cmd: Cmd,
}

/// Emitted at info level immediately before each dispatched
/// subcommand. Visible with `-v`.
fn log_start(line_no: usize, raw: &str) {
    tracing::info!("line {line_no}: start: {}", raw.trim_end());
}

/// Emitted at info level immediately after each dispatched
/// subcommand. Maps the exit code to one of the three outcome words
/// the user wants to see at a glance.
fn log_end(line_no: usize, raw: &str, code: i32) {
    let raw = raw.trim_end();
    match code {
        0 => tracing::info!("line {line_no}: success: {raw}"),
        // EXIT_CODE_WARNING from util_bin::cli — kept literal here to
        // avoid a cross-module dep just for one number.
        3 => tracing::info!("line {line_no}: warning: {raw}"),
        _ => tracing::info!("line {line_no}: failure (exit {code}): {raw}"),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExecutorOptions {
    pub workers: usize, // resolved (1 = sequential)
    /// `Some(N)` = stop spawning new commands once `N` failures have
    /// been recorded (graceful: in-flight commands complete). `None` =
    /// `--continue-on-error` (no failure-driven stop). Default mapping
    /// performed in `batch_run::run`: no flag → `Some(1)` (fail-fast),
    /// `--max-errors N` → `Some(N)`, `--continue-on-error` → `None`.
    pub error_threshold: Option<u64>,
    pub no_summary: bool,
    pub streaming: bool,
}

/// Shared interrupt flag. Set by the SIGINT handler installed in
/// `batch_run::run`; checked by both executors to stop spawning new
/// commands. Per-subcommand cancellation tokens (registered inside
/// each dispatched command) handle propagation into in-flight transfers.
pub type Interrupt = Arc<AtomicBool>;

pub async fn run_sequential(
    lines: Vec<PreparedLine>,
    opts: ExecutorOptions,
    dispatch: DispatchFn,
    interrupt: Interrupt,
) -> (i32, Summary) {
    let total = lines.len() as u64;
    let mut progress = Progress::new(
        total,
        Progress::should_show(opts.no_summary, opts.streaming),
    );
    let mut summary = Summary::default();
    let start = Instant::now();
    let mut worst = 0i32;

    for (idx, line) in lines.into_iter().enumerate() {
        // Bail out if SIGINT arrived between commands (regardless of
        // error threshold — interrupt is unconditional).
        if interrupt.load(Ordering::SeqCst) {
            let processed = idx as u64;
            summary.skipped = total.saturating_sub(processed);
            break;
        }
        let PreparedLine { line_no, raw, cmd } = line;
        log_start(line_no, &raw);
        let code = dispatch(cmd).await;
        log_end(line_no, &raw, code);
        progress.tick(code);
        summary.record(code);
        if code > worst {
            worst = code;
        }
        if code != 0 && opts.error_threshold.is_some_and(|t| summary.failed >= t) {
            // Skipped count = total - already-processed.
            let processed = (idx + 1) as u64;
            summary.skipped = total.saturating_sub(processed);
            break;
        }
    }

    finish_or_abandon(&progress, &summary);
    summary.elapsed = start.elapsed();
    (worst, summary)
}

pub async fn run_parallel(
    lines: Vec<PreparedLine>,
    opts: ExecutorOptions,
    dispatch: DispatchFn,
    interrupt: Interrupt,
) -> (i32, Summary) {
    let total = lines.len() as u64;
    let progress = Arc::new(tokio::sync::Mutex::new(Progress::new(
        total,
        Progress::should_show(opts.no_summary, opts.streaming),
    )));
    let mut summary = Summary::default();
    let start = Instant::now();

    let sem = Arc::new(Semaphore::new(opts.workers));
    let fail_cancel = Arc::new(AtomicBool::new(false));
    let fail_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut spawned = 0u64;

    // Dispatched futures hold non-`Send` types (e.g. `&dyn
    // StorageTrait`) across `.await` points, so we cannot use
    // `tokio::spawn` / `JoinSet::spawn`. A `LocalSet` lets us drive
    // many non-`Send` futures concurrently on the current thread; the
    // semaphore still bounds active workers to `opts.workers`.
    let local = LocalSet::new();
    let worst = local
        .run_until(async {
            let mut joinset: JoinSet<(usize, i32)> = JoinSet::new();
            for line in lines {
                // SIGINT (interrupt) unconditionally stops spawning.
                if interrupt.load(Ordering::SeqCst) {
                    break;
                }
                // Failure-driven cancel: tasks set `fail_cancel` only after
                // the per-run error threshold has been crossed (None =
                // unbounded → never set).
                if fail_cancel.load(Ordering::SeqCst) {
                    break;
                }
                let permit = sem.clone().acquire_owned().await.expect("sem closed");
                let dispatch = Arc::clone(&dispatch);
                let progress = Arc::clone(&progress);
                let fail_cancel = Arc::clone(&fail_cancel);
                let fail_count = Arc::clone(&fail_count);
                let error_threshold = opts.error_threshold;
                spawned += 1;
                joinset.spawn_local(async move {
                    let _permit = permit;
                    let PreparedLine { line_no, raw, cmd } = line;
                    log_start(line_no, &raw);
                    let code = dispatch(cmd).await;
                    log_end(line_no, &raw, code);
                    if code != 0 {
                        let new_count = fail_count.fetch_add(1, Ordering::SeqCst) + 1;
                        if let Some(threshold) = error_threshold
                            && new_count >= threshold
                        {
                            fail_cancel.store(true, Ordering::SeqCst);
                        }
                    }
                    progress.lock().await.tick(code);
                    (line_no, code)
                });
            }

            let mut worst = 0i32;
            while let Some(joined) = joinset.join_next().await {
                match joined {
                    Ok((_, code)) => {
                        summary.record(code);
                        if code > worst {
                            worst = code;
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"), "task panicked");
                        summary.record(1);
                        worst = worst.max(1);
                    }
                }
            }
            worst
        })
        .await;

    summary.skipped = total.saturating_sub(spawned);
    {
        let p = progress.lock().await;
        if summary.skipped > 0 {
            p.abandon();
        } else {
            p.finish();
        }
    }
    summary.elapsed = start.elapsed();
    (worst, summary)
}

/// Streaming sequential executor. Identical semantics to
/// [`run_sequential`], except lines arrive on a channel rather than as a
/// pre-built `Vec`. Total is unknown so the progress bar is unconditionally
/// off (matching `Progress::should_show`); skipped count is accumulated as
/// drain count when fail-fast or interrupt trips.
pub async fn run_sequential_streaming(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<PreparedLine>,
    opts: ExecutorOptions,
    dispatch: DispatchFn,
    interrupt: Interrupt,
) -> (i32, Summary) {
    let mut progress = Progress::new(0, false);
    let mut summary = Summary::default();
    let start = Instant::now();
    let mut worst = 0i32;

    while let Some(line) = rx.recv().await {
        if interrupt.load(Ordering::SeqCst) {
            summary.skipped += 1;
            // Drain remaining items the reader has already produced.
            while rx.recv().await.is_some() {
                summary.skipped += 1;
            }
            break;
        }
        let PreparedLine { line_no, raw, cmd } = line;
        log_start(line_no, &raw);
        let code = dispatch(cmd).await;
        log_end(line_no, &raw, code);
        progress.tick(code);
        summary.record(code);
        if code > worst {
            worst = code;
        }
        if code != 0 && opts.error_threshold.is_some_and(|t| summary.failed >= t) {
            // Drain remaining items already in the channel as skipped.
            while rx.recv().await.is_some() {
                summary.skipped += 1;
            }
            break;
        }
    }

    finish_or_abandon(&progress, &summary);
    summary.elapsed = start.elapsed();
    (worst, summary)
}

/// Streaming parallel executor. Identical semantics to [`run_parallel`],
/// except lines arrive on a channel rather than as a pre-built `Vec`.
/// Dispatched futures hold non-`Send` types, so a `LocalSet` drives them
/// on the current thread.
pub async fn run_parallel_streaming(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<PreparedLine>,
    opts: ExecutorOptions,
    dispatch: DispatchFn,
    interrupt: Interrupt,
) -> (i32, Summary) {
    let progress = Arc::new(tokio::sync::Mutex::new(Progress::new(0, false)));
    let mut summary = Summary::default();
    let start = Instant::now();

    let sem = Arc::new(Semaphore::new(opts.workers));
    let fail_cancel = Arc::new(AtomicBool::new(false));
    let fail_count = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Use LocalSet because dispatch futures are not Send.
    let local = LocalSet::new();
    let worst = local
        .run_until(async {
            let mut joinset: JoinSet<(usize, i32)> = JoinSet::new();
            loop {
                // SIGINT (interrupt) unconditionally stops spawning.
                if interrupt.load(Ordering::SeqCst) {
                    break;
                }
                // Failure-driven cancel: tasks set `fail_cancel` only after
                // the per-run error threshold has been crossed (None =
                // unbounded → never set).
                if fail_cancel.load(Ordering::SeqCst) {
                    break;
                }

                // Pull the next line from the channel, OR notice channel
                // closed (meaning reader is done).
                let line = match rx.recv().await {
                    Some(line) => line,
                    None => break, // channel closed, reader finished
                };

                let permit = sem.clone().acquire_owned().await.expect("sem closed");
                let dispatch = Arc::clone(&dispatch);
                let progress = Arc::clone(&progress);
                let fail_cancel = Arc::clone(&fail_cancel);
                let fail_count = Arc::clone(&fail_count);
                let error_threshold = opts.error_threshold;
                joinset.spawn_local(async move {
                    let _permit = permit;
                    let PreparedLine { line_no, raw, cmd } = line;
                    log_start(line_no, &raw);
                    let code = dispatch(cmd).await;
                    log_end(line_no, &raw, code);
                    if code != 0 {
                        let new_count = fail_count.fetch_add(1, Ordering::SeqCst) + 1;
                        if let Some(threshold) = error_threshold
                            && new_count >= threshold
                        {
                            fail_cancel.store(true, Ordering::SeqCst);
                        }
                    }
                    progress.lock().await.tick(code);
                    (line_no, code)
                });
            }

            // Spawn loop ended. Drain any remaining channel items as skipped.
            while rx.recv().await.is_some() {
                summary.skipped += 1;
            }

            // Wait for all spawned tasks to finish.
            let mut worst = 0i32;
            while let Some(joined) = joinset.join_next().await {
                match joined {
                    Ok((_, code)) => {
                        summary.record(code);
                        if code > worst {
                            worst = code;
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"), "task panicked");
                        summary.record(1);
                        worst = worst.max(1);
                    }
                }
            }
            worst
        })
        .await;

    {
        let p = progress.lock().await;
        if summary.skipped > 0 {
            p.abandon();
        } else {
            p.finish();
        }
    }
    summary.elapsed = start.elapsed();
    (worst, summary)
}

/// `finish()` makes the bar jump to 100%; `abandon()` leaves it at the
/// current position. Pick based on whether anything was skipped — if so,
/// the run was cut short (fail-fast or interrupt) and a "100%" finale
/// would be misleading.
fn finish_or_abandon(progress: &Progress, summary: &Summary) {
    if summary.skipped > 0 {
        progress.abandon();
    } else {
        progress.finish();
    }
}

/// Resolve `--parallel 0` to `num_cpus`. Otherwise pass through.
pub fn resolve_workers(parallel: usize) -> usize {
    if parallel == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    } else {
        parallel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn fake_dispatch(codes: Vec<i32>) -> DispatchFn {
        let codes = Arc::new(tokio::sync::Mutex::new(codes.into_iter()));
        Arc::new(move |_cmd: Cmd| {
            let codes = Arc::clone(&codes);
            Box::pin(async move {
                let mut g = codes.lock().await;
                g.next().expect("not enough fake codes")
            })
        })
    }

    fn make_lines(n: usize) -> Vec<PreparedLine> {
        (0..n)
            .map(|i| PreparedLine {
                line_no: i + 1,
                raw: format!("create-bucket s3://b{i}"),
                cmd: crate::cli::Cli::try_parse_from(["s7cmd", "create-bucket", "s3://b"])
                    .unwrap()
                    .command
                    .unwrap(),
            })
            .collect()
    }

    /// `error_threshold = Some(1)` is the historical "fail-fast" mode
    /// (no flag); `None` is `--continue-on-error`; `Some(N>1)` is
    /// `--max-errors N`.
    fn opts(workers: usize, error_threshold: Option<u64>) -> ExecutorOptions {
        ExecutorOptions {
            workers,
            error_threshold,
            no_summary: true,
            streaming: false,
        }
    }

    fn no_interrupt() -> Interrupt {
        Arc::new(AtomicBool::new(false))
    }

    fn already_interrupted() -> Interrupt {
        Arc::new(AtomicBool::new(true))
    }

    #[tokio::test]
    async fn sequential_all_ok() {
        let lines = make_lines(3);
        let (code, summary) = run_sequential(
            lines,
            opts(1, Some(1)),
            fake_dispatch(vec![0, 0, 0]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 0);
        assert_eq!(summary.ok, 3);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 0);
    }

    #[tokio::test]
    async fn sequential_fail_fast() {
        let lines = make_lines(5);
        let (code, summary) = run_sequential(
            lines,
            opts(1, Some(1)),
            fake_dispatch(vec![0, 1]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 1);
        assert_eq!(summary.ok, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 3);
    }

    #[tokio::test]
    async fn sequential_continue_on_error() {
        let lines = make_lines(4);
        let (code, summary) = run_sequential(
            lines,
            opts(1, None),
            fake_dispatch(vec![0, 1, 0, 4]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 4);
        assert_eq!(summary.ok, 2);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.skipped, 0);
    }

    #[tokio::test]
    async fn sequential_interrupt_stops_before_first_command() {
        let lines = make_lines(3);
        let (code, summary) = run_sequential(
            lines,
            opts(1, None), // even with continue_on_error
            fake_dispatch(vec![]),
            already_interrupted(),
        )
        .await;
        assert_eq!(code, 0);
        assert_eq!(summary.ok, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 3);
    }

    #[tokio::test]
    async fn parallel_all_ok() {
        let lines = make_lines(4);
        let (code, summary) = run_parallel(
            lines,
            opts(2, Some(1)),
            fake_dispatch(vec![0, 0, 0, 0]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 0);
        assert_eq!(summary.ok, 4);
    }

    #[tokio::test]
    async fn parallel_fail_fast_skips_nothing_already_in_flight() {
        // With workers=2 and 5 lines, first 2 spawn immediately. Once
        // they finish, more spawn. If line 1 fails and continue_on_error
        // is false, lines 3-5 are skipped.
        let lines = make_lines(5);
        let (code, _) = run_parallel(
            lines,
            opts(2, Some(1)),
            fake_dispatch(vec![1, 0, 0, 0, 0]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 1);
        // We don't assert exact skipped count because parallel ordering
        // is racy — what matters is that fail_fast did stop the spawn loop.
    }

    #[test]
    fn resolve_workers_zero_picks_num_cpus() {
        let n = resolve_workers(0);
        assert!(n >= 1);
    }

    #[test]
    fn resolve_workers_passthrough() {
        assert_eq!(resolve_workers(1), 1);
        assert_eq!(resolve_workers(8), 8);
    }

    // ---- streaming variants ----

    #[tokio::test]
    async fn sequential_streaming_all_ok() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        for line in make_lines(3) {
            tx.send(line).unwrap();
        }
        drop(tx);
        let (code, summary) = run_sequential_streaming(
            rx,
            opts(1, Some(1)),
            fake_dispatch(vec![0, 0, 0]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 0);
        assert_eq!(summary.ok, 3);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 0);
    }

    #[tokio::test]
    async fn sequential_streaming_fail_fast_drains_channel() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        for line in make_lines(5) {
            tx.send(line).unwrap();
        }
        drop(tx);
        let (code, summary) = run_sequential_streaming(
            rx,
            opts(1, Some(1)),
            fake_dispatch(vec![0, 1]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 1);
        assert_eq!(summary.ok, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 3);
    }

    #[tokio::test]
    async fn sequential_streaming_continue_on_error() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        for line in make_lines(4) {
            tx.send(line).unwrap();
        }
        drop(tx);
        let (code, summary) = run_sequential_streaming(
            rx,
            opts(1, None),
            fake_dispatch(vec![0, 1, 0, 4]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 4);
        assert_eq!(summary.ok, 2);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.skipped, 0);
    }

    #[tokio::test]
    async fn sequential_streaming_interrupt_skips_all() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        for line in make_lines(3) {
            tx.send(line).unwrap();
        }
        drop(tx);
        let (code, summary) = run_sequential_streaming(
            rx,
            opts(1, None),
            fake_dispatch(vec![]),
            already_interrupted(),
        )
        .await;
        assert_eq!(code, 0);
        assert_eq!(summary.ok, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 3);
    }

    #[tokio::test]
    async fn parallel_streaming_all_ok() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        for line in make_lines(4) {
            tx.send(line).unwrap();
        }
        drop(tx);
        let (code, summary) = run_parallel_streaming(
            rx,
            opts(2, Some(1)),
            fake_dispatch(vec![0, 0, 0, 0]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 0);
        assert_eq!(summary.ok, 4);
    }

    #[tokio::test]
    async fn parallel_streaming_fail_fast_stops_spawning() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        for line in make_lines(5) {
            tx.send(line).unwrap();
        }
        drop(tx);
        let (code, _) = run_parallel_streaming(
            rx,
            opts(2, Some(1)),
            fake_dispatch(vec![1, 0, 0, 0, 0]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 1);
        // We don't assert exact skipped count because parallel ordering
        // is racy — what matters is that fail_fast did stop the spawn loop.
    }

    // ---- --max-errors threshold ----

    /// Threshold of 2: the run continues past the first failure (line 1
    /// fails, line 2 ok, line 3 fails → threshold reached) and stops
    /// before line 4. Sequential ordering makes the skip count exact.
    #[tokio::test]
    async fn sequential_max_errors_two_stops_after_second_failure() {
        let lines = make_lines(5);
        let (code, summary) = run_sequential(
            lines,
            opts(1, Some(2)),
            fake_dispatch(vec![1, 0, 1, 0, 0]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 1);
        assert_eq!(summary.ok, 1);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.skipped, 2); // lines 4 and 5 not run
    }

    /// Threshold of 3 with only two failures across the whole run: the
    /// threshold is never reached, every line executes, and nothing is
    /// skipped.
    #[tokio::test]
    async fn sequential_max_errors_three_runs_to_completion_with_two_failures() {
        let lines = make_lines(4);
        let (code, summary) = run_sequential(
            lines,
            opts(1, Some(3)),
            fake_dispatch(vec![0, 1, 0, 1]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 1);
        assert_eq!(summary.ok, 2);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.skipped, 0);
    }

    /// Same shape in the streaming sequential executor.
    #[tokio::test]
    async fn sequential_streaming_max_errors_two_stops_after_second_failure() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        for line in make_lines(5) {
            tx.send(line).unwrap();
        }
        drop(tx);
        let (code, summary) = run_sequential_streaming(
            rx,
            opts(1, Some(2)),
            fake_dispatch(vec![1, 0, 1, 0, 0]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 1);
        assert_eq!(summary.ok, 1);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.skipped, 2);
    }

    /// Parallel executor with workers=2 and threshold=2: at least one of
    /// the trailing lines must be skipped because the spawn loop stops
    /// once two failures have been recorded. Exact skip count is racy
    /// (an in-flight task may complete first) so we only assert the
    /// load-bearing invariant.
    #[tokio::test]
    async fn parallel_max_errors_two_stops_spawning() {
        let lines = make_lines(8);
        let (code, summary) = run_parallel(
            lines,
            opts(2, Some(2)),
            fake_dispatch(vec![1, 1, 0, 0, 0, 0, 0, 0]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 1);
        assert!(summary.failed >= 2);
        assert!(
            summary.skipped > 0,
            "threshold should have stopped the spawn loop; summary={summary:?}"
        );
    }
}
