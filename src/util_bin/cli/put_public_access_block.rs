// Vendored from s3util-rs@1.1.0
//   src/bin/s3util/cli/put_public_access_block.rs
// Adjustments: no tests stripped; rewrote crate::cli → super
use anyhow::{Context, Result};
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::put_public_access_block::PutPublicAccessBlockArgs;
use s3util_rs::input::json::PublicAccessBlockConfigurationJson;
use s3util_rs::storage::s3::api;

/// Runtime entry for
/// `s3util put-public-access-block s3://<BUCKET> <CONFIG_FILE|->`.
///
/// Reads the configuration JSON from a file path or stdin (`-`), parses it
/// into a `PublicAccessBlockConfigurationJson` mirror struct (AWS-CLI input
/// shape), converts to the SDK type, and issues `PutPublicAccessBlock`.
/// Exits silently on success.
pub async fn run_put_public_access_block(
    args: PutPublicAccessBlockArgs,
    client_config: ClientConfig,
) -> Result<()> {
    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;

    let config_arg = args
        .public_access_block_configuration
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("public-access-block-configuration source required"))?;

    let body = if config_arg == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        buf
    } else {
        std::fs::read_to_string(config_arg).with_context(|| {
            format!("reading public-access-block configuration from {config_arg}")
        })?
    };

    let parsed: PublicAccessBlockConfigurationJson =
        serde_json::from_str(&body).with_context(|| format!("parsing JSON from {config_arg}"))?;
    let cfg = parsed.into_sdk()?;

    let client = client_config.create_client().await;
    if args.dry_run {
        info!(bucket = %bucket, "[dry-run] would put public access block.");
        return Ok(());
    }
    api::put_public_access_block(&client, &bucket, cfg).await?;
    info!(bucket = %bucket, "Public access block set.");
    Ok(())
}
