//! SIGINT handling for `batch-run`, which needs no AWS access — so unlike
//! the transfer commands (covered in `tests/e2e_ctrl_c.rs`) these run under
//! a plain `cargo test`.
//!
//! Unix-only: SIGINT delivery via the `nix` crate.

#![cfg(unix)]

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const STARTUP_DELAY: Duration = Duration::from_millis(1500);
const WAIT_TIMEOUT: Duration = Duration::from_secs(30);

fn s7cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_s7cmd"))
}

/// Poll for child exit up to `WAIT_TIMEOUT`. Returns `None` if it never
/// exits, so callers can assert "did not hang" without blocking a CI run
/// forever.
fn wait_bounded(child: &mut std::process::Child) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while Instant::now() < deadline {
        match child.try_wait().expect("try_wait failed") {
            Some(status) => return Some(status),
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    None
}

/// Non-streaming `batch-run -` spends phase 1 in a blocking read of stdin
/// under the *default* SIGINT disposition (the handler is installed only
/// after the script has been read), so Ctrl-C there terminates the process
/// by signal — exactly the "I haven't started anything yet" intuition the
/// phase-1 comment describes. `code()` is `None` for a signal death;
/// the signal number is 2 (SIGINT).
#[test]
fn batch_run_sigint_while_reading_stdin_dies_by_signal() {
    use std::os::unix::process::ExitStatusExt;

    let mut child = s7cmd()
        .args(["batch-run", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn s7cmd");

    std::thread::sleep(STARTUP_DELAY);
    let _ = kill(Pid::from_raw(child.id() as i32), Signal::SIGINT);

    let status = wait_bounded(&mut child).unwrap_or_else(|| {
        let _ = child.kill();
        panic!("batch-run did not exit within {WAIT_TIMEOUT:?} of SIGINT during phase-1 read");
    });
    assert_eq!(
        status.signal(),
        Some(2),
        "phase-1 SIGINT must terminate by signal; got {status:?}"
    );
}

/// Streaming `batch-run --streaming -` sitting idle on stdin: SIGINT sets
/// the shared interrupt flag and wakes the reader, so the run finishes
/// cleanly with exit 0 (no line ever executed, so nothing is "failed" or
/// "skipped"). Closing stdin afterwards must not change that; the
/// pipe-stays-open case is covered by
/// `batch_run_streaming_sigint_exits_with_stdin_left_open`.
#[test]
fn batch_run_streaming_sigint_then_eof_exits_0() {
    let mut child = s7cmd()
        .args(["batch-run", "--streaming", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn s7cmd");

    std::thread::sleep(STARTUP_DELAY);
    let _ = kill(Pid::from_raw(child.id() as i32), Signal::SIGINT);

    // Let the signal be observed, then release the blocking stdin read.
    std::thread::sleep(Duration::from_millis(300));
    drop(child.stdin.take());

    let status = wait_bounded(&mut child).unwrap_or_else(|| {
        let _ = child.kill();
        panic!("streaming batch-run did not exit within {WAIT_TIMEOUT:?} of SIGINT + EOF");
    });
    assert_eq!(
        status.code(),
        Some(0),
        "interrupted idle streaming batch must exit 0; got {status:?}"
    );
}

/// The same streaming path with real work queued: lines are written before
/// the signal, stdin is closed after it, and the run still exits cleanly.
/// Every line is a `--dry-run` so no S3 endpoint is contacted.
#[test]
fn batch_run_streaming_sigint_with_queued_lines_exits_cleanly() {
    let mut child = s7cmd()
        .args(["batch-run", "--streaming", "--disable-color-tracing", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn s7cmd");

    {
        let stdin = child.stdin.as_mut().expect("stdin piped");
        for i in 0..3 {
            writeln!(stdin, "create-bucket --dry-run s3://sigint-stream-{i}").expect("write line");
        }
        stdin.flush().expect("flush");
    }

    std::thread::sleep(STARTUP_DELAY);
    let _ = kill(Pid::from_raw(child.id() as i32), Signal::SIGINT);
    std::thread::sleep(Duration::from_millis(300));
    drop(child.stdin.take());

    let status = wait_bounded(&mut child).unwrap_or_else(|| {
        let _ = child.kill();
        panic!("streaming batch-run did not exit within {WAIT_TIMEOUT:?} of SIGINT + EOF");
    });
    assert_eq!(
        status.code(),
        Some(0),
        "dry-run lines all succeed, so an interrupt after them exits 0; got {status:?}"
    );
}

/// SIGINT alone must end an idle streaming run even though its stdin pipe
/// never closes. Through `tokio::io::stdin()` the uncancellable blocking
/// read would sit on tokio's blocking pool, which tokio documents "can make
/// shutdown of the runtime hang until the user presses enter"; streaming
/// mode therefore reads stdin on its own thread (`ThreadReader`).
#[test]
fn batch_run_streaming_sigint_exits_with_stdin_left_open() {
    let mut child = s7cmd()
        .args(["batch-run", "--streaming", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn s7cmd");

    std::thread::sleep(STARTUP_DELAY);
    let _ = kill(Pid::from_raw(child.id() as i32), Signal::SIGINT);

    // stdin stays open (held by `child`) until after the wait.
    let status = wait_bounded(&mut child).unwrap_or_else(|| {
        let _ = child.kill();
        panic!(
            "streaming batch-run with open stdin did not exit within {WAIT_TIMEOUT:?} of SIGINT"
        );
    });
    assert_eq!(
        status.code(),
        Some(0),
        "interrupted idle streaming batch must exit 0; got {status:?}"
    );
}

/// A `--max-errors` stop must end a streaming run whose stdin stays open
/// (`tail -f cmds | s7cmd batch-run --streaming -`): the executor stops
/// taking lines, the reader stops reading, and the process exits with the
/// failing line's code instead of waiting for an EOF that never comes.
/// No signal involved; it lives here for the bounded-wait helpers.
#[test]
fn batch_run_streaming_max_errors_stop_exits_with_stdin_left_open() {
    let mut child = s7cmd()
        .args([
            "batch-run",
            "--streaming",
            "--max-errors",
            "1",
            "--no-summary",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn s7cmd");

    {
        let stdin = child.stdin.as_mut().expect("stdin piped");
        // Not a subcommand: an invalid line, exit 2, trips --max-errors 1.
        writeln!(stdin, "bogus-cmd x").expect("write line");
        stdin.flush().expect("flush");
    }

    let status = wait_bounded(&mut child).unwrap_or_else(|| {
        let _ = child.kill();
        panic!("streaming batch-run with open stdin did not exit within {WAIT_TIMEOUT:?} of a --max-errors stop");
    });
    assert_eq!(
        status.code(),
        Some(2),
        "the invalid line's exit code; got {status:?}"
    );
}

/// The same `--max-errors` stop with the script given as a FIFO path
/// instead of `-`: a path that is not a regular file may never end, so it
/// must not be read to EOF after the stop either.
#[test]
fn batch_run_streaming_max_errors_stop_exits_with_open_fifo_script() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fifo = dir.path().join("cmds.fifo");
    let made = Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("run mkfifo");
    assert!(made.success(), "mkfifo failed: {made:?}");

    let mut child = s7cmd()
        .args([
            "batch-run",
            "--streaming",
            "--max-errors",
            "1",
            "--no-summary",
        ])
        .arg(&fifo)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn s7cmd");

    // Opening the write end blocks until s7cmd opens the read end; do it
    // on a helper thread so a run that never opens the FIFO fails the test
    // instead of hanging it.
    let (opened_tx, opened_rx) = std::sync::mpsc::channel();
    {
        let fifo = fifo.clone();
        std::thread::spawn(move || {
            let _ = opened_tx.send(std::fs::OpenOptions::new().write(true).open(&fifo));
        });
    }
    let mut writer = match opened_rx.recv_timeout(WAIT_TIMEOUT) {
        Ok(w) => w.expect("open fifo for writing"),
        Err(_) => {
            let _ = child.kill();
            // Unblock the helper thread's open so it can finish.
            let _ = std::fs::File::open(&fifo);
            panic!("s7cmd never opened the FIFO script within {WAIT_TIMEOUT:?}");
        }
    };
    writeln!(writer, "bogus-cmd x").expect("write line");
    writer.flush().expect("flush");

    // `writer` stays open until after the wait: the FIFO never hits EOF.
    let status = wait_bounded(&mut child).unwrap_or_else(|| {
        let _ = child.kill();
        panic!(
            "streaming batch-run with an open FIFO script did not exit within {WAIT_TIMEOUT:?} of a --max-errors stop"
        );
    });
    drop(writer);
    assert_eq!(
        status.code(),
        Some(2),
        "the invalid line's exit code; got {status:?}"
    );
}
