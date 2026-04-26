// Vendored from s3rm-rs@1.3.3
//   src/bin/s3rm/tracing_init.rs
// Adjustments: stripped #[cfg(test)] mod tests; added s7cmd=<level> to
//              filter strings (vendored tracing events emit from
//              s7cmd::clean_bin::* targets)

// Tracing infrastructure adapted from s3sync.
// Initializes the tracing subscriber for the CLI binary.

use std::env;
use std::io::IsTerminal;

use tracing_subscriber::fmt::format::FmtSpan;

use s3rm_rs::config::TracingConfig;

const EVENT_FILTER_ENV_VAR: &str = "RUST_LOG";

pub fn init_tracing(config: &TracingConfig) {
    let fmt_span = if config.span_events_tracing {
        FmtSpan::NEW | FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    let subscriber_builder = tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .compact()
        .with_target(false)
        .with_ansi(!config.disable_color_tracing && std::io::stdout().is_terminal())
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
