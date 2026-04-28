// Vendored from s3util-rs@0.2.2
//   src/bin/s3util/cli/delete_bucket_encryption.rs
// Adjustments: no tests stripped; rewrote crate::cli → super
use anyhow::Result;
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::delete_bucket_encryption::DeleteBucketEncryptionArgs;
use s3util_rs::storage::s3::api;

/// Runtime entry for `s3util delete-bucket-encryption s3://<BUCKET>`.
///
/// Builds the SDK client from `client_config`, issues `DeleteBucketEncryption`,
/// and returns silently on success.
pub async fn run_delete_bucket_encryption(
    args: DeleteBucketEncryptionArgs,
    client_config: ClientConfig,
) -> Result<()> {
    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;
    let client = client_config.create_client().await;
    api::delete_bucket_encryption(&client, &bucket).await?;
    info!(bucket = %bucket, "Bucket encryption deleted.");
    Ok(())
}
