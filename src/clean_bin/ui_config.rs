// Vendored from s3rm-rs@1.3.3
//   src/bin/s3rm/ui_config.rs
// Adjustments: stripped #[cfg(test)] mod tests

// UI configuration helpers adapted from s3sync's `bin/s3sync/cli/ui_config.rs`.
//
// Determines whether to show the progress indicator and result summary
// based on Config settings (quiet mode, verbosity, JSON logging).

use s3rm_rs::config::Config;

/// Whether to show the live-updating progress indicator.
///
/// Returns `false` when:
/// - `show_no_progress` is set (quiet mode)
/// - Verbosity is above Warn (tracing takes over the terminal)
/// - JSON logging is enabled (progress text would corrupt JSON output)
pub fn is_progress_indicator_needed(config: &Config) -> bool {
    if config.show_no_progress {
        return false;
    }

    let Some(tracing_config) = config.tracing_config.as_ref() else {
        return true;
    };

    if log::Level::Warn < tracing_config.tracing_level {
        return false;
    }

    !tracing_config.json_tracing
}

/// Whether to show the final result summary line.
///
/// Returns `false` when:
/// - `show_no_progress` is set (quiet mode)
/// - JSON logging is enabled (result would corrupt JSON output)
pub fn is_show_result_needed(config: &Config) -> bool {
    if config.show_no_progress {
        return false;
    }

    let Some(tracing_config) = config.tracing_config.as_ref() else {
        return true;
    };

    !tracing_config.json_tracing
}
