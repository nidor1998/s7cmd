// Vendored from s3util-rs@1.1.0
//   src/bin/s3util/cli/put_bucket_versioning.rs
// Adjustments: no tests stripped; rewrote crate::cli → super
use anyhow::Result;
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::put_bucket_versioning::PutBucketVersioningArgs;
use s3util_rs::storage::s3::api;

/// Runtime entry for `s3util put-bucket-versioning s3://<BUCKET> --enabled|--suspended`.
///
/// Builds the SDK client from `client_config`, issues `PutBucketVersioning`
/// with `Status=Enabled` or `Status=Suspended` (determined by the mutually-
/// exclusive `--enabled` / `--suspended` flags), and exits silently on success.
pub async fn run_put_bucket_versioning(
    args: PutBucketVersioningArgs,
    client_config: ClientConfig,
) -> Result<()> {
    // Enforce that exactly one of --enabled / --suspended was given.
    // This exits with code 2 if neither flag is present (matches clap convention).
    args.validate_state_flag();

    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;
    let status = args.versioning_status();
    let client = client_config.create_client().await;
    if args.dry_run {
        info!(bucket = %bucket, status = %status.as_str(), "[dry-run] would put bucket versioning.");
        return Ok(());
    }
    api::put_bucket_versioning(&client, &bucket, status.clone()).await?;
    info!(bucket = %bucket, status = %status.as_str(), "Bucket versioning set.");
    Ok(())
}
