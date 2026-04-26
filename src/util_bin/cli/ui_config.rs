// Vendored from s3util-rs@0.2.0
//   src/bin/s3util/cli/ui_config.rs
// Adjustments: stripped #[cfg(test)] mod tests

use s3util_rs::Config;

pub fn is_progress_indicator_needed(config: &Config) -> bool {
    if !config.show_progress {
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
    if !config.show_progress {
        return false;
    }

    if config.tracing_config.is_none() {
        return true;
    }

    !config.tracing_config.as_ref().unwrap().json_tracing
}
