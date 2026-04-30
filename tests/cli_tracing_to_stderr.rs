//! End-to-end CLI test: launches the real `s7cmd` binary as a child process and
//! verifies that tracing output is written to stderr (and not leaked to stdout).
//!
//! This intentionally doesn't require AWS — the assertion only depends on the
//! `trace_config_summary` call in `src/main.rs` that fires
//! immediately after `init_tracing`, before any S3 work. The cp invocation is
//! expected to fail on the missing source, but the trace line has already been
//! emitted by then. The trace deliberately logs only non-sensitive summary
//! fields (no credentials or SSE-C key material).

use std::process::{Command, Stdio};

#[test]
fn tracing_output_goes_to_stderr_not_stdout() {
    let bin = env!("CARGO_BIN_EXE_s7cmd");

    // -vvv → trace level so the "config = ..." trace fires.
    // --disable-color-tracing → no ANSI escapes muddying string matches.
    // The source path is deliberately nonexistent: the cp will fail, but only
    // *after* tracing init has already emitted the trace line we're asserting on.
    let output = Command::new(bin)
        .args([
            "cp",
            "-vvv",
            "--disable-color-tracing",
            "/nonexistent/source/file/for/tracing/test.bin",
            "s3://nonexistent-bucket-for-tracing-test-12345/key",
        ])
        // Empty environment for tracing-related vars so user shell config (RUST_LOG,
        // NO_COLOR, etc.) can't perturb the test.
        .env_remove("RUST_LOG")
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn s7cmd binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The trace line emitted by `trace_config_summary` in main.rs must appear
    // on stderr.
    assert!(
        stderr.contains("config ="),
        "expected the 'config = ...' trace on stderr.\n--- stderr ---\n{stderr}\n--- stdout ---\n{stdout}"
    );

    // tracing-subscriber's compact formatter emits a level marker per line.
    // At -vvv we should see at least one of TRACE/DEBUG/INFO/WARN/ERROR on stderr.
    assert!(
        stderr.contains("TRACE")
            || stderr.contains("DEBUG")
            || stderr.contains("INFO")
            || stderr.contains("WARN")
            || stderr.contains("ERROR"),
        "expected a tracing level marker on stderr.\n--- stderr ---\n{stderr}"
    );

    // And tracing output must NOT have leaked to stdout. Use the same anchors
    // we asserted on stderr — anything else on stdout (e.g. nothing) is fine.
    assert!(
        !stdout.contains("config ="),
        "tracing 'config = ...' line leaked to stdout.\n--- stdout ---\n{stdout}"
    );
    assert!(
        !(stdout.contains(" TRACE ")
            || stdout.contains(" DEBUG ")
            || stdout.contains(" INFO ")
            || stdout.contains(" WARN ")
            || stdout.contains(" ERROR ")),
        "tracing level marker leaked to stdout.\n--- stdout ---\n{stdout}"
    );
}
