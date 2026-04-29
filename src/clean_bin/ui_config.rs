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

#[cfg(test)]
mod tests {
    use s3rm_rs::parse_from_args;

    use super::*;

    fn config_from_args(args: &[&str]) -> Config {
        Config::try_from(parse_from_args(args).unwrap()).unwrap()
    }

    #[test]
    fn progress_indicator_needed_default() {
        let config = config_from_args(&[
            "s3rm",
            "--target-profile",
            "p",
            "--force",
            "s3://test-bucket",
        ]);
        assert!(is_progress_indicator_needed(&config));
    }

    #[test]
    fn progress_indicator_suppressed_by_show_no_progress() {
        let config = config_from_args(&[
            "s3rm",
            "--target-profile",
            "p",
            "--force",
            "--show-no-progress",
            "s3://test-bucket",
        ]);
        assert!(!is_progress_indicator_needed(&config));
    }

    #[test]
    fn progress_indicator_suppressed_by_json_tracing() {
        let config = config_from_args(&[
            "s3rm",
            "--target-profile",
            "p",
            "--force",
            "--json-tracing",
            "s3://test-bucket",
        ]);
        assert!(!is_progress_indicator_needed(&config));
    }

    #[test]
    fn progress_indicator_suppressed_by_high_verbosity() {
        let config = config_from_args(&[
            "s3rm",
            "-v",
            "--target-profile",
            "p",
            "--force",
            "s3://test-bucket",
        ]);
        assert!(!is_progress_indicator_needed(&config));
    }

    #[test]
    fn progress_indicator_shown_when_no_tracing_config() {
        let config = config_from_args(&[
            "s3rm",
            "-qqq",
            "--target-profile",
            "p",
            "--force",
            "s3://test-bucket",
        ]);
        assert!(is_progress_indicator_needed(&config));
    }

    #[test]
    fn show_result_needed_default() {
        let config = config_from_args(&[
            "s3rm",
            "--target-profile",
            "p",
            "--force",
            "s3://test-bucket",
        ]);
        assert!(is_show_result_needed(&config));
    }

    #[test]
    fn show_result_suppressed_by_show_no_progress() {
        let config = config_from_args(&[
            "s3rm",
            "--target-profile",
            "p",
            "--force",
            "--show-no-progress",
            "s3://test-bucket",
        ]);
        assert!(!is_show_result_needed(&config));
    }

    #[test]
    fn show_result_suppressed_by_json_tracing() {
        let config = config_from_args(&[
            "s3rm",
            "--target-profile",
            "p",
            "--force",
            "--json-tracing",
            "s3://test-bucket",
        ]);
        assert!(!is_show_result_needed(&config));
    }

    #[test]
    fn show_result_shown_at_verbose_level() {
        let config = config_from_args(&[
            "s3rm",
            "-v",
            "--target-profile",
            "p",
            "--force",
            "s3://test-bucket",
        ]);
        assert!(is_show_result_needed(&config));
    }

    #[test]
    fn show_result_shown_when_no_tracing_config() {
        let config = config_from_args(&[
            "s3rm",
            "-qqq",
            "--target-profile",
            "p",
            "--force",
            "s3://test-bucket",
        ]);
        assert!(is_show_result_needed(&config));
    }
}
