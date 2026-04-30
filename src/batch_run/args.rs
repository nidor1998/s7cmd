//! Implementation of methods on `BatchRunArgs` (defined in `src/cli.rs`).
//! The struct itself lives in `cli.rs` so the integration test that
//! `#[path]`-includes `cli.rs` does not need to also pull in `batch_run`.

use s3util_rs::config::TracingConfig;

use crate::cli::BatchRunArgs;

impl BatchRunArgs {
    /// Mirror of `CommonClientArgs::build_tracing_config()`.
    /// Returns `None` when verbosity is below the lowest tracing level
    /// (e.g. `-qqq`), matching every other subcommand's behaviour.
    pub fn build_tracing_config(&self) -> Option<TracingConfig> {
        self.verbosity
            .log_level()
            .map(|tracing_level| TracingConfig {
                tracing_level,
                json_tracing: self.json_tracing,
                aws_sdk_tracing: self.aws_sdk_tracing,
                span_events_tracing: self.span_events_tracing,
                disable_color_tracing: self.disable_color_tracing,
            })
    }
}
