// Vendored from s3util-rs@0.2.0
//   src/bin/s3util/cli/delete_object_tagging.rs
// Adjustments: no tests stripped; rewrote crate::cli → super

use anyhow::Result;
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::delete_object_tagging::DeleteObjectTaggingArgs;
use s3util_rs::storage::s3::api;

/// Runtime entry for `s3util delete-object-tagging s3://<BUCKET>/<KEY>`.
///
/// Builds the SDK client from `client_config`, issues `DeleteObjectTagging`,
/// removing all tags from the object. Silent on success.
pub async fn run_delete_object_tagging(
    args: DeleteObjectTaggingArgs,
    client_config: ClientConfig,
) -> Result<()> {
    let (bucket, key) = args
        .bucket_key()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;

    let client = client_config.create_client().await;

    api::delete_object_tagging(&client, &bucket, &key, args.source_version_id.as_deref()).await?;
    info!(
        bucket = %bucket,
        key = %key,
        version_id = %args.source_version_id.as_deref().unwrap_or_default(),
        "Object tagging deleted."
    );
    Ok(())
}
