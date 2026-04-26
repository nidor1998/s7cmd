// Vendored from s3sync@1.57.1
//   src/bin/s3sync/cli/ui_config.rs
// Adjustments: stripped #[cfg(test)] mod tests

use s3sync::Config;

pub fn is_progress_indicator_needed(config: &Config) -> bool {
    if config.show_no_progress {
        return false;
    }

    if config.tracing_config.is_none() {
        return true;
    }

    if log::Level::Warn < config.tracing_config.as_ref().unwrap().tracing_level {
        return false;
    }

    !config.tracing_config.as_ref().unwrap().json_tracing
}

pub fn is_show_result_needed(config: &Config) -> bool {
    if config.show_no_progress {
        return false;
    }

    if config.tracing_config.is_none() {
        return true;
    }

    if config.report_sync_status {
        return false;
    }

    !config.tracing_config.as_ref().unwrap().json_tracing
}
