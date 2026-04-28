// Vendored from s3rm-rs@1.3.3
//   src/bin/s3rm/tracing_init.rs
// Adjustments: stripped #[cfg(test)] mod tests; added s7cmd=<level> to
//              filter strings (vendored tracing events emit from
//              s7cmd::clean_bin::* targets)

// Tracing infrastructure adapted from s3sync.
// Initializes the tracing subscriber for the CLI binary.

use std::env;
use std::io::{IsTerminal, Write};

use tracing_subscriber::fmt::format::FmtSpan;

use s3rm_rs::config::TracingConfig;

const EVENT_FILTER_ENV_VAR: &str = "RUST_LOG";

/// A writer that silently ignores BrokenPipe errors, so piping to
/// head/tail does not produce noisy tracing-subscriber diagnostics.
struct PipeSafeWriter;

impl Write for PipeSafeWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match std::io::stderr().write(buf) {
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(buf.len()),
            other => other,
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match std::io::stderr().flush() {
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

    let subscriber_builder = tracing_subscriber::fmt()
        .with_writer(|| PipeSafeWriter)
        .compact()
        .with_target(false)
        .with_ansi(!config.disable_color_tracing && std::io::stderr().is_terminal())
        .with_span_events(fmt_span);

    let mut show_target = true;
    let tracing_level = config.tracing_level;
    let event_filter = if config.aws_sdk_tracing {
        format!(
            "s7cmd={tracing_level},s3rm_rs={tracing_level},s3rm={tracing_level},aws_smithy_runtime={tracing_level},aws_config={tracing_level},aws_sigv4={tracing_level}"
        )
    } else if let Ok(env_filter) = env::var(EVENT_FILTER_ENV_VAR) {
        env_filter
    } else {
        show_target = false;
        format!("s7cmd={tracing_level},s3rm_rs={tracing_level},s3rm={tracing_level}")
    };

    let subscriber_builder = subscriber_builder
        .with_env_filter(event_filter)
        .with_target(show_target);
    if config.json_tracing {
        subscriber_builder.json().init();
    } else {
        subscriber_builder.init();
    }
}

#[cfg(test)]
mod tests {
    use std::io::{IsTerminal, Write};

    use s3rm_rs::config::TracingConfig;
    use tracing_subscriber::fmt::format::FmtSpan;

    use super::{EVENT_FILTER_ENV_VAR, PipeSafeWriter};

    /// Mirror of `init_tracing` that uses `try_init` so it never panics on
    /// repeated calls during tests where another subscriber may already be set.
    fn try_init_tracing(config: &TracingConfig) {
        let fmt_span = if config.span_events_tracing {
            FmtSpan::NEW | FmtSpan::CLOSE
        } else {
            FmtSpan::NONE
        };

        let subscriber_builder = tracing_subscriber::fmt()
            .with_writer(|| PipeSafeWriter)
            .compact()
            .with_target(false)
            .with_ansi(!config.disable_color_tracing && std::io::stderr().is_terminal())
            .with_span_events(fmt_span);

        let mut show_target = true;
        let tracing_level = config.tracing_level;
        let event_filter = if config.aws_sdk_tracing {
            format!(
                "s7cmd={tracing_level},s3rm_rs={tracing_level},s3rm={tracing_level},aws_smithy_runtime={tracing_level},aws_config={tracing_level},aws_sigv4={tracing_level}"
            )
        } else if let Ok(env_filter) = std::env::var(EVENT_FILTER_ENV_VAR) {
            env_filter
        } else {
            show_target = false;
            format!("s7cmd={tracing_level},s3rm_rs={tracing_level},s3rm={tracing_level}")
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
        try_init_tracing(&TracingConfig {
            tracing_level: log::Level::Info,
            json_tracing: true,
            aws_sdk_tracing: false,
            span_events_tracing: false,
            disable_color_tracing: false,
        });
    }

    #[test]
    fn init_aws_sdk_tracing() {
        try_init_tracing(&TracingConfig {
            tracing_level: log::Level::Info,
            json_tracing: false,
            aws_sdk_tracing: true,
            span_events_tracing: false,
            disable_color_tracing: false,
        });
    }

    #[test]
    fn init_normal_tracing() {
        unsafe { std::env::remove_var(EVENT_FILTER_ENV_VAR) };

        try_init_tracing(&TracingConfig {
            tracing_level: log::Level::Info,
            json_tracing: false,
            aws_sdk_tracing: false,
            span_events_tracing: false,
            disable_color_tracing: false,
        });
    }

    #[test]
    fn init_span_events_tracing() {
        try_init_tracing(&TracingConfig {
            tracing_level: log::Level::Info,
            json_tracing: false,
            aws_sdk_tracing: true,
            span_events_tracing: true,
            disable_color_tracing: false,
        });
    }

    #[test]
    fn init_disable_color_tracing() {
        try_init_tracing(&TracingConfig {
            tracing_level: log::Level::Info,
            json_tracing: false,
            aws_sdk_tracing: false,
            span_events_tracing: false,
            disable_color_tracing: true,
        });
    }

    #[test]
    fn init_with_env() {
        unsafe { std::env::set_var(EVENT_FILTER_ENV_VAR, "trace") };

        try_init_tracing(&TracingConfig {
            tracing_level: log::Level::Info,
            json_tracing: false,
            aws_sdk_tracing: false,
            span_events_tracing: false,
            disable_color_tracing: true,
        });

        unsafe { std::env::remove_var(EVENT_FILTER_ENV_VAR) };
    }

    #[test]
    fn pipe_safe_writer_writes_normally() {
        let mut writer = PipeSafeWriter;
        let n = writer.write(b"").expect("empty write must succeed");
        assert_eq!(n, 0);
    }

    #[test]
    fn pipe_safe_writer_flush_normally() {
        let mut writer = PipeSafeWriter;
        writer.flush().expect("flush must succeed");
    }
}
