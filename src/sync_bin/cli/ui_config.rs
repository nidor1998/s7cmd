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

#[cfg(test)]
mod tests {
    use s3sync::config::args::parse_from_args;

    use super::*;

    fn config_from_args(args: &[&str]) -> Config {
        Config::try_from(parse_from_args(args).unwrap()).unwrap()
    }

    #[test]
    fn is_progress_indicator_needed_json_tracing() {
        let config = config_from_args(&[
            "s3sync",
            "--source-profile",
            "p",
            "--json-tracing",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(!is_progress_indicator_needed(&config));
    }

    #[test]
    fn is_progress_indicator_needed_no_json_tracing() {
        let config = config_from_args(&[
            "s3sync",
            "--source-profile",
            "p",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(is_progress_indicator_needed(&config));
    }

    #[test]
    fn is_progress_indicator_needed_no_tracing_config() {
        let config = config_from_args(&[
            "s3sync",
            "--source-profile",
            "p",
            "-qqq",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(is_progress_indicator_needed(&config));
    }

    #[test]
    fn is_progress_indicator_needed_show_no_progress() {
        let config = config_from_args(&[
            "s3sync",
            "--source-profile",
            "p",
            "--show-no-progress",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(!is_progress_indicator_needed(&config));
    }

    #[test]
    fn is_progress_indicator_needed_info_verbose() {
        let config = config_from_args(&[
            "s3sync",
            "-v",
            "--source-profile",
            "p",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(!is_progress_indicator_needed(&config));
    }

    #[test]
    fn is_show_result_needed_default() {
        let config = config_from_args(&[
            "s3sync",
            "--source-profile",
            "p",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(is_show_result_needed(&config));
    }

    #[test]
    fn is_show_result_needed_silent() {
        let config = config_from_args(&[
            "s3sync",
            "-qqq",
            "--source-profile",
            "p",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(is_show_result_needed(&config));
    }

    #[test]
    fn is_show_result_needed_show_no_progress() {
        let config = config_from_args(&[
            "s3sync",
            "--source-profile",
            "p",
            "--show-no-progress",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(!is_show_result_needed(&config));
    }

    #[test]
    fn is_show_result_needed_json_tracing() {
        let config = config_from_args(&[
            "s3sync",
            "-v",
            "--source-profile",
            "p",
            "--json-tracing",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(!is_show_result_needed(&config));
    }

    #[test]
    fn is_show_result_needed_sync_report() {
        let config = config_from_args(&[
            "s3sync",
            "-v",
            "--source-profile",
            "p",
            "--report-sync-status",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(!is_show_result_needed(&config));
    }
}
