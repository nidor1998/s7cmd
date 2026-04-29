// Vendored from s3ls-rs@0.4.1
//   src/bin/s3ls/main.rs
// Adjustments: removed #[tokio::main] async fn main() (s7cmd has its own);
//              helpers made pub for dispatch from s7cmd::main; stripped
//              any #[cfg(test)] blocks; load_config_exit_if_err takes args
//              by value instead of calling parse() (s7cmd parses at top level)

use anyhow::Result;
use tracing::{debug, error};

use s3ls_rs::bucket_lister;
use s3ls_rs::config::Config;
use s3ls_rs::{
    CLIArgs, ListingPipeline, create_pipeline_cancellation_token, exit_code_from_error,
    is_cancelled_error,
};

mod ctrl_c_handler;
mod tracing_init;

// Adjusted from upstream: takes args by value instead of calling
// CLIArgs::parse() internally (s7cmd parses at the top level).
pub fn load_config_exit_if_err(args: CLIArgs) -> Config {
    match Config::try_from(args) {
        Ok(config) => config,
        Err(error_message) => {
            clap::Error::raw(clap::error::ErrorKind::ValueValidation, error_message).exit();
        }
    }
}

pub fn start_tracing_if_necessary(config: &Config) -> bool {
    if let Some(tracing_config) = config.tracing_config.as_ref() {
        tracing_init::init_tracing(tracing_config);
        true
    } else {
        false
    }
}

pub async fn run(config: Config) -> Result<()> {
    // Bucket listing mode: no target specified
    if config.target.bucket.is_empty() {
        return match bucket_lister::list_buckets(&config).await {
            Ok(()) => Ok(()),
            Err(e) => {
                if let Some(io_err) = e.downcast_ref::<std::io::Error>()
                    && io_err.kind() == std::io::ErrorKind::BrokenPipe
                {
                    return Ok(());
                }
                error!("{}", e);
                std::process::exit(1);
            }
        };
    }

    let cancellation_token = create_pipeline_cancellation_token();

    ctrl_c_handler::spawn_ctrl_c_handler(cancellation_token.clone());

    let start_time = tokio::time::Instant::now();
    debug!("listing pipeline start.");

    let pipeline = ListingPipeline::new(config, cancellation_token);

    match pipeline.run().await {
        Ok(()) => {
            let duration_sec = format!("{:.3}", start_time.elapsed().as_secs_f32());
            debug!(duration_sec = duration_sec, "s7cmd ls has been completed.");
            Ok(())
        }
        Err(e) => {
            // Broken pipe is expected when piped to head/tail — exit silently.
            if let Some(io_err) = e.downcast_ref::<std::io::Error>()
                && io_err.kind() == std::io::ErrorKind::BrokenPipe
            {
                return Ok(());
            }
            let duration_sec = format!("{:.3}", start_time.elapsed().as_secs_f32());
            if is_cancelled_error(&e) {
                debug!("listing cancelled by user.");
                return Ok(());
            }
            let code = exit_code_from_error(&e);
            error!(duration_sec = duration_sec, "s7cmd ls failed.");
            error!("{}", e);
            std::process::exit(code);
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn parse(args: &[&str]) -> CLIArgs {
        CLIArgs::try_parse_from(args).unwrap()
    }

    #[test]
    fn load_config_exit_if_err_returns_config_for_valid_args() {
        let cli_args = parse(&["s3ls", "--target-profile", "p", "s3://test-bucket/prefix/"]);
        let config = load_config_exit_if_err(cli_args);
        assert_eq!(config.target.bucket, "test-bucket");
        assert_eq!(config.target.prefix.as_deref(), Some("prefix/"));
    }

    #[test]
    fn load_config_exit_if_err_returns_config_for_bucket_listing() {
        let cli_args = parse(&["s3ls", "--target-profile", "p"]);
        let config = load_config_exit_if_err(cli_args);
        // Bucket listing mode — target.bucket is empty.
        assert!(config.target.bucket.is_empty());
    }

    #[test]
    fn start_tracing_if_necessary_returns_false_when_no_tracing_config() {
        // -qqq drops below all tracing levels → tracing_config is None.
        let cli_args = parse(&["s3ls", "-qqq", "--target-profile", "p", "s3://test-bucket"]);
        let config = load_config_exit_if_err(cli_args);
        assert!(config.tracing_config.is_none());
        assert!(!start_tracing_if_necessary(&config));
    }
}
