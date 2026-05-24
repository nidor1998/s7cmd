// Vendored from s3util-rs@1.5.0 src/bin/s3util/cli/rename.rs
use anyhow::Result;
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::rename::RenameArgs;
use s3util_rs::storage::s3::api::{self, HeadError, RenameObjectConditions};

use super::ExitStatus;

pub async fn run_rename(args: RenameArgs, client_config: ClientConfig) -> Result<ExitStatus> {
    let (src_bucket, src_key) = args
        .source_bucket_key()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;
    let (_, dst_key) = args
        .target_bucket_key()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;

    let client = client_config.create_client().await;

    if args.dry_run {
        info!(
            source = %format!("s3://{src_bucket}/{src_key}"),
            target = %format!("s3://{src_bucket}/{dst_key}"),
            "[dry-run] would rename object."
        );
        return Ok(ExitStatus::Success);
    }

    let source_if_none_match = if args.source_if_none_match {
        Some("*")
    } else {
        None
    };
    let target_if_none_match = if args.target_if_none_match {
        Some("*")
    } else {
        None
    };

    match api::rename_object(
        &client,
        &src_bucket,
        &src_key,
        &dst_key,
        RenameObjectConditions {
            source_if_match: args.source_if_match.as_deref(),
            source_if_none_match,
            destination_if_match: args.target_if_match.as_deref(),
            destination_if_none_match: target_if_none_match,
        },
    )
    .await
    {
        Ok(_) => {
            info!(
                source = %format!("s3://{src_bucket}/{src_key}"),
                target = %format!("s3://{src_bucket}/{dst_key}"),
                "Object renamed."
            );
            Ok(ExitStatus::Success)
        }
        Err(HeadError::BucketNotFound) => {
            Err(anyhow::anyhow!("bucket s3://{src_bucket} not found"))
        }
        Err(HeadError::NotFound) => Err(anyhow::anyhow!(
            "object s3://{src_bucket}/{src_key} not found"
        )),
        Err(HeadError::Other(e)) => Err(e),
    }
}
