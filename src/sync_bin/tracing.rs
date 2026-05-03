// Vendored from s3sync@1.58.6
//   src/bin/s3sync/tracing/mod.rs
// Adjustments: stripped rusty_fork_test! tests; added s7cmd=<level> to
//              filter strings (vendored tracing events now emit from
//              s7cmd::sync_bin::* targets, not s3sync)
use std::env;
use std::io::{IsTerminal, Write};

use tracing_subscriber::fmt::format::FmtSpan;

use s3sync::config::TracingConfig;

const EVENT_FILTER_ENV_VAR: &str = "RUST_LOG";

/// A writer that silently ignores `BrokenPipe` errors when forwarding to
/// the chosen standard stream, so piping to `head`/`wc`/etc. does not
/// produce noisy `tracing-subscriber` diagnostics or panics on broken
/// pipes (e.g. `s3sync ... | wc -l` followed by Ctrl-C).
enum PipeSafeWriter {
    Stdout,
    Stderr,
}

impl PipeSafeWriter {
    fn write_inner(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            PipeSafeWriter::Stdout => std::io::stdout().write(buf),
            PipeSafeWriter::Stderr => std::io::stderr().write(buf),
        }
    }

    fn flush_inner(&mut self) -> std::io::Result<()> {
        match self {
            PipeSafeWriter::Stdout => std::io::stdout().flush(),
            PipeSafeWriter::Stderr => std::io::stderr().flush(),
        }
    }
}

impl Write for PipeSafeWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.write_inner(buf) {
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(buf.len()),
            other => other,
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self.flush_inner() {
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
            other => other,
        }
    }
}

pub fn init_tracing(config: &TracingConfig) {
    let fmt_span = if config.span_events_tracing {
        FmtSpan::NEW | FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    let stderr_tracing = config.stderr_tracing;
    let ansi_enabled = !config.disable_color_tracing
        && if stderr_tracing {
            std::io::stderr().is_terminal()
        } else {
            std::io::stdout().is_terminal()
        };

    let subscriber_builder = tracing_subscriber::fmt()
        .with_writer(move || {
            if stderr_tracing {
                PipeSafeWriter::Stderr
            } else {
                PipeSafeWriter::Stdout
            }
        })
        .compact()
        .with_target(false)
        .with_ansi(ansi_enabled)
        .with_span_events(fmt_span);

    let mut show_target = true;
    let tracing_level = config.tracing_level;
    let event_filter = if config.aws_sdk_tracing {
        format!(
            "s7cmd={tracing_level},s3sync={tracing_level},aws_smithy_runtime={tracing_level},aws_config={tracing_level},aws_sigv4={tracing_level}"
        )
    } else if let Ok(env_filter) = env::var(EVENT_FILTER_ENV_VAR) {
        env_filter
    } else {
        show_target = false;
        format!("s7cmd={tracing_level},s3sync={tracing_level}")
    };

    let subscriber_builder = subscriber_builder
        .with_env_filter(event_filter)
        .with_target(show_target);
    // try_init keeps init_tracing idempotent: batch-run installs the
    // subscriber once at the top, then per-line subcommand dispatch
    // arms call this again (harmlessly).
    let _ = if config.json_tracing {
        subscriber_builder.json().try_init()
    } else {
        subscriber_builder.try_init()
    };
}

#[cfg(test)]
mod tests {
    use std::io::{IsTerminal, Write};

    use s3sync::config::TracingConfig;
    use tracing_subscriber::fmt::format::FmtSpan;

    use super::{EVENT_FILTER_ENV_VAR, PipeSafeWriter};

    fn make_tracing_config(
        tracing_level: log::Level,
        json_tracing: bool,
        aws_sdk_tracing: bool,
        span_events_tracing: bool,
        disable_color_tracing: bool,
        stderr_tracing: bool,
    ) -> TracingConfig {
        TracingConfig {
            tracing_level,
            json_tracing,
            aws_sdk_tracing,
            span_events_tracing,
            disable_color_tracing,
            stderr_tracing,
        }
    }

    fn try_init_tracing(config: &TracingConfig) {
        let fmt_span = if config.span_events_tracing {
            FmtSpan::NEW | FmtSpan::CLOSE
        } else {
            FmtSpan::NONE
        };

        let stderr_tracing = config.stderr_tracing;
        let ansi_enabled = !config.disable_color_tracing
            && if stderr_tracing {
                std::io::stderr().is_terminal()
            } else {
                std::io::stdout().is_terminal()
            };

        let subscriber_builder = tracing_subscriber::fmt()
            .with_writer(move || {
                if stderr_tracing {
                    PipeSafeWriter::Stderr
                } else {
                    PipeSafeWriter::Stdout
                }
            })
            .compact()
            .with_target(false)
            .with_ansi(ansi_enabled)
            .with_span_events(fmt_span);

        let mut show_target = true;
        let tracing_level = config.tracing_level;
        let event_filter = if config.aws_sdk_tracing {
            format!(
                "s7cmd={tracing_level},s3sync={tracing_level},aws_smithy_runtime={tracing_level},aws_config={tracing_level},aws_sigv4={tracing_level}"
            )
        } else if let Ok(env_filter) = std::env::var(EVENT_FILTER_ENV_VAR) {
            env_filter
        } else {
            show_target = false;
            format!("s7cmd={tracing_level},s3sync={tracing_level}")
        };

        let subscriber_builder = subscriber_builder
            .with_env_filter(event_filter)
            .with_target(show_target);
        if config.json_tracing {
            let _ = subscriber_builder.json().try_init();
        } else {
            let _ = subscriber_builder.try_init();
        }
    }

    #[test]
    fn init_json_tracing() {
        try_init_tracing(&make_tracing_config(
            log::Level::Info,
            true,
            false,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn init_aws_sdk_tracing() {
        try_init_tracing(&make_tracing_config(
            log::Level::Info,
            false,
            true,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn init_normal_tracing() {
        unsafe { std::env::remove_var(EVENT_FILTER_ENV_VAR) };

        try_init_tracing(&make_tracing_config(
            log::Level::Info,
            false,
            false,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn init_span_events_tracing() {
        try_init_tracing(&make_tracing_config(
            log::Level::Info,
            false,
            true,
            true,
            false,
            false,
        ));
    }

    #[test]
    fn init_disable_color_tracing() {
        try_init_tracing(&make_tracing_config(
            log::Level::Info,
            false,
            false,
            false,
            true,
            false,
        ));
    }

    #[test]
    fn init_stderr_tracing() {
        try_init_tracing(&make_tracing_config(
            log::Level::Info,
            false,
            false,
            false,
            false,
            true,
        ));
    }

    #[test]
    fn init_with_env() {
        unsafe { std::env::set_var(EVENT_FILTER_ENV_VAR, "trace") };

        try_init_tracing(&make_tracing_config(
            log::Level::Info,
            false,
            false,
            false,
            true,
            false,
        ));

        unsafe { std::env::remove_var(EVENT_FILTER_ENV_VAR) };
    }

    #[test]
    fn pipe_safe_writer_stdout_writes_normally() {
        let mut writer = PipeSafeWriter::Stdout;
        let n = writer.write(b"").expect("empty write must succeed");
        assert_eq!(n, 0);
    }

    #[test]
    fn pipe_safe_writer_stderr_writes_normally() {
        let mut writer = PipeSafeWriter::Stderr;
        let n = writer.write(b"").expect("empty write must succeed");
        assert_eq!(n, 0);
    }

    #[test]
    fn pipe_safe_writer_stdout_flush_normally() {
        let mut writer = PipeSafeWriter::Stdout;
        writer.flush().expect("flush must succeed");
    }

    #[test]
    fn pipe_safe_writer_stderr_flush_normally() {
        let mut writer = PipeSafeWriter::Stderr;
        writer.flush().expect("flush must succeed");
    }

    /// Tests that exercise the *production* `init_tracing` body. See the
    /// matching block in `util_bin::tracing_init` for the full rationale —
    /// in short, the mirror's existence prevents production-side branches
    /// (the AWS-SDK filter format, the span-events arm, the
    /// stderr-vs-stdout `is_terminal` arm) from being recorded in coverage.
    /// These calls run the full body and no-op only at the final
    /// `try_init` step.

    #[test]
    fn production_init_span_events_tracing() {
        super::init_tracing(&make_tracing_config(
            log::Level::Info,
            false,
            false,
            true,
            false,
            false,
        ));
    }

    #[test]
    fn production_init_aws_sdk_tracing() {
        super::init_tracing(&make_tracing_config(
            log::Level::Info,
            false,
            true,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn production_init_stderr_tracing() {
        super::init_tracing(&make_tracing_config(
            log::Level::Info,
            false,
            false,
            false,
            false,
            true,
        ));
    }

    #[test]
    fn production_init_disable_color_tracing() {
        super::init_tracing(&make_tracing_config(
            log::Level::Info,
            false,
            false,
            false,
            true,
            false,
        ));
    }

    #[test]
    fn production_init_json_tracing() {
        super::init_tracing(&make_tracing_config(
            log::Level::Info,
            true,
            false,
            false,
            true,
            false,
        ));
    }
}
