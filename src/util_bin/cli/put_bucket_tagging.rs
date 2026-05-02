// Vendored from s3util-rs@1.1.0
//   src/bin/s3util/cli/put_bucket_tagging.rs
// Adjustments: no tests stripped; rewrote crate::cli → super

use anyhow::Result;
use tracing::info;

use aws_sdk_s3::types::Tagging;
use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::put_bucket_tagging::PutBucketTaggingArgs;
use s3util_rs::storage::s3::api;

use super::tagging::parse_tagging_to_tags;

/// Runtime entry for `s3util put-bucket-tagging s3://<BUCKET>`.
///
/// Builds the SDK client from `client_config`, issues `PutBucketTagging`,
/// replacing all existing tags with the supplied ones. Silent on success.
pub async fn run_put_bucket_tagging(
    args: PutBucketTaggingArgs,
    client_config: ClientConfig,
) -> Result<()> {
    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;

    let tagging_str = args.tagging.as_deref().unwrap_or("");
    let tags = parse_tagging_to_tags(tagging_str)?;
    let tagging = Tagging::builder().set_tag_set(Some(tags)).build()?;

    let client = client_config.create_client().await;

    if args.dry_run {
        info!(bucket = %bucket, "[dry-run] would put bucket tagging.");
        return Ok(());
    }

    api::put_bucket_tagging(&client, &bucket, tagging).await?;
    info!(bucket = %bucket, "Bucket tagging set.");
    Ok(())
}
