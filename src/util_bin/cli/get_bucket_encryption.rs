// Vendored from s3util-rs@0.2.2
//   src/bin/s3util/cli/get_bucket_encryption.rs
// Adjustments: no tests stripped; rewrote crate::cli → super
use anyhow::Result;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::get_bucket_encryption::GetBucketEncryptionArgs;
use s3util_rs::output::json::get_bucket_encryption_to_json;
use s3util_rs::storage::s3::api::{self, HeadError};

use super::ExitStatus;

/// Runtime entry for `s3util get-bucket-encryption s3://<BUCKET>`.
///
/// Builds the SDK client from `client_config`, issues `GetBucketEncryption`,
/// and prints the response as AWS-CLI-shape pretty-printed JSON followed by
/// a newline. Returns `ExitStatus::NotFound` (exit code 4) when S3 reports
/// `NoSuchBucket` (logged as "bucket … not found") or
/// `ServerSideEncryptionConfigurationNotFoundError` (logged as
/// "encryption configuration for … not found").
pub async fn run_get_bucket_encryption(
    args: GetBucketEncryptionArgs,
    client_config: ClientConfig,
) -> Result<ExitStatus> {
    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;
    let client = client_config.create_client().await;
    match api::get_bucket_encryption(&client, &bucket).await {
        Ok(out) => {
            let json = get_bucket_encryption_to_json(&out);
            let pretty = serde_json::to_string_pretty(&json)?;
            println!("{pretty}");
            Ok(ExitStatus::Success)
        }
        Err(HeadError::BucketNotFound) => {
            tracing::error!("bucket s3://{bucket} not found");
            Ok(ExitStatus::NotFound)
        }
        Err(HeadError::NotFound) => {
            tracing::error!("encryption configuration for s3://{bucket} not found");
            Ok(ExitStatus::NotFound)
        }
        Err(HeadError::Other(e)) => Err(e),
    }
}
