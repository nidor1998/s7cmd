//! Implementation of methods on `BatchRunArgs` (defined in `src/cli.rs`).
//! The struct itself lives in `cli.rs` so the integration test that
//! `#[path]`-includes `cli.rs` does not need to also pull in `batch_run`.

use s3util_rs::config::TracingConfig;

use crate::cli::BatchRunArgs;

impl BatchRunArgs {
    /// Mirror of `CommonClientArgs::build_tracing_config()`.
    /// Returns `None` when verbosity is below the lowest tracing level
    /// (e.g. `-qqq`), matching every other subcommand's behaviour.
    ///
    /// `--check-format` forces the level to at least `Info` so the
    /// "format OK" success message is visible at the default
    /// `WarnLevel` — the same pattern `--dry-run` uses for s3util
    /// commands.
    pub fn build_tracing_config(&self) -> Option<TracingConfig> {
        let tracing_level = if self.check_format {
            Some(
                self.verbosity
                    .log_level()
                    .map_or(log::Level::Info, |l| l.max(log::Level::Info)),
            )
        } else {
            self.verbosity.log_level()
        }?;
        Some(TracingConfig {
            tracing_level,
            json_tracing: self.json_tracing,
            aws_sdk_tracing: self.aws_sdk_tracing,
            span_events_tracing: self.span_events_tracing,
            disable_color_tracing: self.disable_color_tracing,
        })
    }
}
