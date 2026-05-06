// Vendored from s3util-rs@1.3.0
//   src/bin/s3util/cli/delete_bucket_replication.rs
// Adjustments: no tests stripped; rewrote crate::cli → super
use anyhow::Result;
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::delete_bucket_replication::DeleteBucketReplicationArgs;
use s3util_rs::storage::s3::api;

/// Runtime entry for `s3util delete-bucket-replication s3://<BUCKET>`.
///
/// Builds the SDK client from `client_config`, issues `DeleteBucketReplication`,
/// and returns silently on success.
pub async fn run_delete_bucket_replication(
    args: DeleteBucketReplicationArgs,
    client_config: ClientConfig,
) -> Result<()> {
    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;
    let client = client_config.create_client().await;
    if args.dry_run {
        info!(bucket = %bucket, "[dry-run] would delete bucket replication.");
        return Ok(());
    }
    api::delete_bucket_replication(&client, &bucket).await?;
    info!(bucket = %bucket, "Bucket replication deleted.");
    Ok(())
}
