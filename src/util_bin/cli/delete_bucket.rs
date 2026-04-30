// Vendored from s3util-rs@1.1.0
//   src/bin/s3util/cli/delete_bucket.rs
// Adjustments: no tests stripped; rewrote crate::cli → super

use anyhow::Result;
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::delete_bucket::DeleteBucketArgs;
use s3util_rs::storage::s3::api;

/// Runtime entry for `s3util delete-bucket s3://<BUCKET>`.
///
/// Builds the SDK client from `client_config`, issues `DeleteBucket`, and
/// returns silently on success. The bucket must be empty; S3 returns
/// `409 BucketNotEmpty` otherwise, which is surfaced as an error (exit 1).
pub async fn run_delete_bucket(args: DeleteBucketArgs, client_config: ClientConfig) -> Result<()> {
    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;
    let client = client_config.create_client().await;
    if args.dry_run {
        info!(bucket = %bucket, "[dry-run] would delete bucket.");
        return Ok(());
    }
    api::delete_bucket(&client, &bucket).await?;
    info!(bucket = %bucket, "Bucket deleted.");
    Ok(())
}
