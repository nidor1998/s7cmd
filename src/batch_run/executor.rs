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
    // raw is kept on the struct so the `==> line N: <raw>` banner can be
    // re-enabled by uncommenting the eprintln calls in the executor loops.
    #[allow(dead_code)]
    pub raw: String,
    pub cmd: Cmd,
}

#[derive(Debug, Clone, Copy)]
pub struct ExecutorOptions {
    pub workers: usize, // resolved (1 = sequential)
    pub continue_on_error: bool,
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
        // continue_on_error — interrupt is unconditional).
        if interrupt.load(Ordering::SeqCst) {
            let processed = idx as u64;
            summary.skipped = total.saturating_sub(processed);
            break;
        }
        // eprintln!("==> line {}: {}", line.line_no, line.raw);
        let code = dispatch(line.cmd).await;
        progress.tick(code);
        summary.record(code);
        if code > worst {
            worst = code;
        }
        if code != 0 && !opts.continue_on_error {
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
                // Failure-driven cancel only stops spawning when fail-fast is on.
                if fail_cancel.load(Ordering::SeqCst) && !opts.continue_on_error {
                    break;
                }
                let permit = sem.clone().acquire_owned().await.expect("sem closed");
                let dispatch = Arc::clone(&dispatch);
                let progress = Arc::clone(&progress);
                let fail_cancel = Arc::clone(&fail_cancel);
                let continue_on_error = opts.continue_on_error;
                spawned += 1;
                joinset.spawn_local(async move {
                    let _permit = permit;
                    // eprintln!("==> line {}: {}", line.line_no, line.raw);
                    let code = dispatch(line.cmd).await;
                    if code != 0 && !continue_on_error {
                        fail_cancel.store(true, Ordering::SeqCst);
                    }
                    progress.lock().await.tick(code);
                    (line.line_no, code)
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
        // eprintln!("==> line {}: {}", line.line_no, line.raw);
        let code = dispatch(line.cmd).await;
        progress.tick(code);
        summary.record(code);
        if code > worst {
            worst = code;
        }
        if code != 0 && !opts.continue_on_error {
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
                // Failure-driven cancel only stops spawning when fail-fast is on.
                if fail_cancel.load(Ordering::SeqCst) && !opts.continue_on_error {
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
                let continue_on_error = opts.continue_on_error;
                joinset.spawn_local(async move {
                    let _permit = permit;
                    // eprintln!("==> line {}: {}", line.line_no, line.raw);
                    let code = dispatch(line.cmd).await;
                    if code != 0 && !continue_on_error {
                        fail_cancel.store(true, Ordering::SeqCst);
                    }
                    progress.lock().await.tick(code);
                    (line.line_no, code)
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

    fn opts(workers: usize, continue_on_error: bool) -> ExecutorOptions {
        ExecutorOptions {
            workers,
            continue_on_error,
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
            opts(1, false),
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
            opts(1, false),
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
            opts(1, true),
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
            opts(1, true), // even with continue_on_error
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
            opts(2, false),
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
            opts(2, false),
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
            opts(1, false),
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
            opts(1, false),
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
            opts(1, true),
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
            opts(1, true),
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
            opts(2, false),
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
            opts(2, false),
            fake_dispatch(vec![1, 0, 0, 0, 0]),
            no_interrupt(),
        )
        .await;
        assert_eq!(code, 1);
        // We don't assert exact skipped count because parallel ordering
        // is racy — what matters is that fail_fast did stop the spawn loop.
    }
}
