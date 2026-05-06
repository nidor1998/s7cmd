// Vendored from s3util-rs@1.3.0
//   src/bin/s3util/cli/put_bucket_accelerate_configuration.rs
// Adjustments: no tests stripped; rewrote crate::cli → super
use anyhow::Result;
use aws_sdk_s3::types::AccelerateConfiguration;
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::put_bucket_accelerate_configuration::PutBucketAccelerateConfigurationArgs;
use s3util_rs::storage::s3::api;

/// Runtime entry for
/// `s3util put-bucket-accelerate-configuration s3://<BUCKET> --enabled|--suspended`.
///
/// Builds the SDK client from `client_config`, issues
/// `PutBucketAccelerateConfiguration` with `Status=Enabled` or
/// `Status=Suspended` (determined by the mutually-exclusive `--enabled` /
/// `--suspended` flags), and exits silently on success.
pub async fn run_put_bucket_accelerate_configuration(
    args: PutBucketAccelerateConfigurationArgs,
    client_config: ClientConfig,
) -> Result<()> {
    args.validate_state_flag();

    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;
    let status = args.accelerate_status();
    let cfg = AccelerateConfiguration::builder()
        .status(status.clone())
        .build();
    let client = client_config.create_client().await;
    if args.dry_run {
        info!(
            bucket = %bucket,
            status = %status.as_str(),
            "[dry-run] would put bucket accelerate configuration."
        );
        return Ok(());
    }
    api::put_bucket_accelerate_configuration(&client, &bucket, cfg).await?;
    info!(
        bucket = %bucket,
        status = %status.as_str(),
        "Bucket Transfer Acceleration set."
    );
    Ok(())
}
