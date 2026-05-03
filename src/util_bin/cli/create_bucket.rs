// Vendored from s3util-rs@1.2.0
//   src/bin/s3util/cli/create_bucket.rs
// Adjustments: no tests stripped; rewrote crate::cli → super

use anyhow::Result;
use tracing::info;

use aws_sdk_s3::types::Tagging;
use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::create_bucket::CreateBucketArgs;
use s3util_rs::storage::s3::api::{self, HeadError};

use super::ExitStatus;
use super::tagging::parse_tagging_to_tags;

/// Runtime entry for `s3util create-bucket s3://<BUCKET>`.
///
/// Issues `CreateBucket` using the region from `--target-region`. With
/// `--tagging`, follows up with `PutBucketTagging`. If the tagging step fails
/// after the bucket has been created, exits with [`ExitStatus::Warning`]
/// (exit code 3) and logs a warning explaining the partial state.
/// No automatic rollback is performed.
pub async fn run_create_bucket(
    args: CreateBucketArgs,
    client_config: ClientConfig,
) -> Result<ExitStatus> {
    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;

    // Pre-parse tagging so dry-run validates user input before short-circuit.
    let tagging = if let Some(raw_tagging) = args.tagging.as_deref() {
        let tags = parse_tagging_to_tags(raw_tagging)?;
        Some(Tagging::builder().set_tag_set(Some(tags)).build()?)
    } else {
        None
    };

    let client = client_config.create_client().await;

    if args.if_not_exists {
        match api::head_bucket(&client, &bucket).await {
            Ok(_) => {
                if args.dry_run {
                    info!(bucket = %bucket, "[dry-run] would skip: bucket exists.");
                } else {
                    info!(bucket = %bucket, "Bucket exists; skipping create.");
                }
                return Ok(ExitStatus::Success);
            }
            Err(HeadError::BucketNotFound) | Err(HeadError::NotFound) => {
                // bucket doesn't exist — fall through to the normal create flow
            }
            Err(HeadError::Other(e)) => return Err(e),
        }
    }

    if args.dry_run {
        info!(bucket = %bucket, "[dry-run] would create bucket.");
        if tagging.is_some() {
            info!(bucket = %bucket, "[dry-run] would put bucket tagging.");
        }
        return Ok(ExitStatus::Success);
    }

    api::create_bucket(&client, &bucket).await?;
    info!(bucket = %bucket, "Bucket created.");

    if let Some(tagging) = tagging {
        if let Err(e) = api::put_bucket_tagging(&client, &bucket, tagging).await {
            tracing::warn!(
                error = format!("{e:#}"),
                "bucket s3://{bucket} was created but PutBucketTagging failed; \
                 the bucket exists untagged. Retry tagging or delete the bucket manually."
            );
            return Ok(ExitStatus::Warning);
        }
        info!(bucket = %bucket, "Bucket tagging set.");
    }

    Ok(ExitStatus::Success)
}
