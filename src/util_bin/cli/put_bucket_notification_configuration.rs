// Vendored from s3util-rs@0.2.2
//   src/bin/s3util/cli/put_bucket_notification_configuration.rs
// Adjustments: no tests stripped; rewrote crate::cli → super
use anyhow::{Context, Result};
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::put_bucket_notification_configuration::PutBucketNotificationConfigurationArgs;
use s3util_rs::input::json::NotificationConfigurationJson;
use s3util_rs::storage::s3::api;

/// Runtime entry for
/// `s3util put-bucket-notification-configuration s3://<BUCKET> <CONFIG_FILE|->`.
///
/// Reads the configuration JSON from a file path or stdin (`-`), parses it
/// into a `NotificationConfigurationJson` mirror struct (AWS-CLI input
/// shape), converts to the SDK type, and issues
/// `PutBucketNotificationConfiguration`. Exits silently on success.
///
/// To remove every notification on a bucket, supply an empty configuration
/// (`{}`). AWS does not expose a `DeleteBucketNotificationConfiguration`
/// API; replacing the configuration with one that omits every field is
/// the documented way to clear notifications.
pub async fn run_put_bucket_notification_configuration(
    args: PutBucketNotificationConfigurationArgs,
    client_config: ClientConfig,
) -> Result<()> {
    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;

    let config_arg = args
        .notification_configuration
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("notification-configuration source required"))?;

    let body = if config_arg == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        buf
    } else {
        std::fs::read_to_string(config_arg)
            .with_context(|| format!("reading notification-configuration from {config_arg}"))?
    };

    let parsed: NotificationConfigurationJson =
        serde_json::from_str(&body).with_context(|| format!("parsing JSON from {config_arg}"))?;
    let cfg = parsed.into_sdk()?;

    let client = client_config.create_client().await;
    api::put_bucket_notification_configuration(&client, &bucket, cfg).await?;
    info!(bucket = %bucket, "Bucket notification configuration set.");
    Ok(())
}
