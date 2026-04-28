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

#[cfg(test)]
mod tests {
    use clap::Parser;
    use s3util_rs::config::args::{Cli, Commands, CpArgs};

    use super::*;

    fn cp_args_from(cli: Cli) -> CpArgs {
        match cli.command {
            Commands::Cp(cp_args) => cp_args,
            _ => panic!("expected Cp variant"),
        }
    }

    fn config_from_args(args: &[&str]) -> Config {
        let cli = Cli::try_parse_from(args).unwrap();
        Config::try_from(cp_args_from(cli)).unwrap()
    }

    #[test]
    fn is_progress_indicator_needed_json_tracing() {
        let config = config_from_args(&[
            "s3util",
            "cp",
            "--source-profile",
            "p",
            "--json-tracing",
            "--show-progress",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(!is_progress_indicator_needed(&config));
    }

    #[test]
    fn is_progress_indicator_needed_no_json_tracing() {
        let config = config_from_args(&[
            "s3util",
            "cp",
            "--source-profile",
            "p",
            "--show-progress",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(is_progress_indicator_needed(&config));
    }

    #[test]
    fn is_progress_indicator_needed_no_tracing_config() {
        let config = config_from_args(&[
            "s3util",
            "cp",
            "--source-profile",
            "p",
            "-qqq",
            "--show-progress",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(is_progress_indicator_needed(&config));
    }

    #[test]
    fn is_progress_indicator_needed_default_no_show_progress() {
        let config = config_from_args(&[
            "s3util",
            "cp",
            "--source-profile",
            "p",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(!is_progress_indicator_needed(&config));
    }

    #[test]
    fn is_progress_indicator_needed_info_verbose() {
        let config = config_from_args(&[
            "s3util",
            "cp",
            "-v",
            "--source-profile",
            "p",
            "--show-progress",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(!is_progress_indicator_needed(&config));
    }

    #[test]
    fn is_show_result_needed_default_no_show_progress() {
        let config = config_from_args(&[
            "s3util",
            "cp",
            "--source-profile",
            "p",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(!is_show_result_needed(&config));
    }

    #[test]
    fn is_show_result_needed_silent() {
        let config = config_from_args(&[
            "s3util",
            "cp",
            "-qqq",
            "--source-profile",
            "p",
            "--show-progress",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(is_show_result_needed(&config));
    }

    #[test]
    fn is_show_result_needed_json_tracing() {
        let config = config_from_args(&[
            "s3util",
            "cp",
            "-v",
            "--source-profile",
            "p",
            "--json-tracing",
            "--show-progress",
            "s3://source-bucket",
            "/target-dir",
        ]);
        assert!(!is_show_result_needed(&config));
    }
}
