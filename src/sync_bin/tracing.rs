// Vendored from s3sync@1.57.1
//   src/bin/s3sync/tracing/mod.rs
// Adjustments: stripped rusty_fork_test! tests; added s7cmd=<level> to
//              filter strings (vendored tracing events now emit from
//              s7cmd::sync_bin::* targets, not s3sync)
use std::env;
use std::io::Write;

use tracing_subscriber::fmt::format::FmtSpan;

use s3sync::config::TracingConfig;

const EVENT_FILTER_ENV_VAR: &str = "RUST_LOG";

/// A writer that silently ignores BrokenPipe errors, so piping to
/// head/tail does not produce noisy tracing-subscriber diagnostics.
struct PipeSafeWriter;

impl Write for PipeSafeWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match std::io::stdout().write(buf) {
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(buf.len()),
            other => other,
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match std::io::stdout().flush() {
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
        .with_ansi(!config.disable_color_tracing)
        .with_span_events(fmt_span);

    let mut show_target = true;
    let tracing_level = config.tracing_level;
    let event_filter = if config.aws_sdk_tracing {
        format!(
            "s7cmd={tracing_level},s3sync={tracing_level},aws_smithy_runtime={tracing_level},aws_config={tracing_level},aws_sigv4={tracing_level}"
        )
    } else if env::var(EVENT_FILTER_ENV_VAR).is_ok() {
        env::var(EVENT_FILTER_ENV_VAR).unwrap()
    } else {
        show_target = false;
        format!("s7cmd={tracing_level},s3sync={tracing_level}")
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
