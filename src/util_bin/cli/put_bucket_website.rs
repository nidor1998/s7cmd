// Vendored from s3util-rs@0.2.2
//   src/bin/s3util/cli/put_bucket_website.rs
// Adjustments: no tests stripped; rewrote crate::cli → super
use anyhow::{Context, Result};
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::put_bucket_website::PutBucketWebsiteArgs;
use s3util_rs::input::json::WebsiteConfigurationJson;
use s3util_rs::storage::s3::api;

/// Runtime entry for
/// `s3util put-bucket-website s3://<BUCKET> <CONFIG_FILE|->`.
///
/// Reads the configuration JSON from a file path or stdin (`-`), parses it
/// into a `WebsiteConfigurationJson` mirror struct (AWS-CLI input shape),
/// converts to the SDK type, and issues `PutBucketWebsite`. Exits silently
/// on success.
pub async fn run_put_bucket_website(
    args: PutBucketWebsiteArgs,
    client_config: ClientConfig,
) -> Result<()> {
    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;

    let config_arg = args
        .website_configuration
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("website-configuration source required"))?;

    let body = if config_arg == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        buf
    } else {
        std::fs::read_to_string(config_arg)
            .with_context(|| format!("reading website configuration from {config_arg}"))?
    };

    let parsed: WebsiteConfigurationJson =
        serde_json::from_str(&body).with_context(|| format!("parsing JSON from {config_arg}"))?;
    let cfg = parsed.into_sdk()?;

    let client = client_config.create_client().await;
    api::put_bucket_website(&client, &bucket, cfg).await?;
    info!(bucket = %bucket, "Bucket website configuration set.");
    Ok(())
}
