// Vendored from s3util-rs@0.2.2
//   src/bin/s3util/cli/delete_bucket_lifecycle_configuration.rs
// Adjustments: no tests stripped; rewrote crate::cli → super
use anyhow::Result;
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::delete_bucket_lifecycle_configuration::DeleteBucketLifecycleConfigurationArgs;
use s3util_rs::storage::s3::api;

/// Runtime entry for `s3util delete-bucket-lifecycle-configuration s3://<BUCKET>`.
///
/// Builds the SDK client from `client_config`, issues `DeleteBucketLifecycle`
/// (the symmetric `delete-bucket-lifecycle-configuration` CLI name wraps
/// the SDK's asymmetric `DeleteBucketLifecycle` operation), and returns
/// silently on success.
pub async fn run_delete_bucket_lifecycle_configuration(
    args: DeleteBucketLifecycleConfigurationArgs,
    client_config: ClientConfig,
) -> Result<()> {
    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;
    let client = client_config.create_client().await;
    api::delete_bucket_lifecycle_configuration(&client, &bucket).await?;
    info!(bucket = %bucket, "Bucket lifecycle configuration deleted.");
    Ok(())
}
