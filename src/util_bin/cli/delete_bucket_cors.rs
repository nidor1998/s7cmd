// Vendored from s3util-rs@0.2.2
//   src/bin/s3util/cli/delete_bucket_cors.rs
// Adjustments: no tests stripped; rewrote crate::cli → super
use anyhow::Result;
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::delete_bucket_cors::DeleteBucketCorsArgs;
use s3util_rs::storage::s3::api;

/// Runtime entry for `s3util delete-bucket-cors s3://<BUCKET>`.
///
/// Builds the SDK client from `client_config`, issues `DeleteBucketCors`,
/// and returns silently on success.
pub async fn run_delete_bucket_cors(
    args: DeleteBucketCorsArgs,
    client_config: ClientConfig,
) -> Result<()> {
    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;
    let client = client_config.create_client().await;
    api::delete_bucket_cors(&client, &bucket).await?;
    info!(bucket = %bucket, "Bucket CORS configuration deleted.");
    Ok(())
}
