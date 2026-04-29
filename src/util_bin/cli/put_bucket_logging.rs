// Vendored from s3util-rs@0.2.2
//   src/bin/s3util/cli/put_bucket_logging.rs
// Adjustments: no tests stripped; rewrote crate::cli → super
use anyhow::{Context, Result};
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::put_bucket_logging::PutBucketLoggingArgs;
use s3util_rs::input::json::BucketLoggingStatusJson;
use s3util_rs::storage::s3::api;

/// Runtime entry for
/// `s3util put-bucket-logging s3://<BUCKET> <CONFIG_FILE|->`.
///
/// Reads the configuration JSON from a file path or stdin (`-`), parses it
/// into a `BucketLoggingStatusJson` mirror struct (AWS-CLI input shape),
/// converts to the SDK type, and issues `PutBucketLogging`. Exits silently
/// on success.
///
/// To disable logging on a bucket, supply an empty configuration (`{}`).
/// AWS does not expose a `DeleteBucketLogging` API; replacing the status
/// with one that omits `LoggingEnabled` is the documented way to remove
/// a logging configuration.
pub async fn run_put_bucket_logging(
    args: PutBucketLoggingArgs,
    client_config: ClientConfig,
) -> Result<()> {
    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;

    let config_arg = args
        .bucket_logging_status
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("bucket-logging-status source required"))?;

    let body = if config_arg == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        buf
    } else {
        std::fs::read_to_string(config_arg)
            .with_context(|| format!("reading bucket-logging-status from {config_arg}"))?
    };

    let parsed: BucketLoggingStatusJson =
        serde_json::from_str(&body).with_context(|| format!("parsing JSON from {config_arg}"))?;
    let status = parsed.into_sdk()?;

    let client = client_config.create_client().await;
    api::put_bucket_logging(&client, &bucket, status).await?;
    info!(bucket = %bucket, "Bucket logging set.");
    Ok(())
}
