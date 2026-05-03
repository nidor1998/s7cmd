// Vendored from s3util-rs@1.2.0
//   src/bin/s3util/cli/cp.rs
// Adjustments: stripped #[cfg(test)] mod tests; rewrote crate::cli → super

use anyhow::Result;
use tracing::error;

use s3util_rs::Config;
use s3util_rs::storage::s3::api::{self, HeadError, HeadObjectOpts};
use s3util_rs::types::StoragePath;

use super::{ExitStatus, extract_keys, run_copy_phase};

pub async fn run_cp(config: Config) -> Result<ExitStatus> {
    if config.skip_existing && target_exists(&config).await? {
        let (_, target_key) = extract_keys(&config)?;
        let target_display = match &config.target {
            StoragePath::S3 { bucket, .. } => format!("s3://{bucket}/{target_key}"),
            StoragePath::Local(_) => target_key,
            StoragePath::Stdio => "-".to_string(),
        };
        if config.dry_run {
            tracing::info!(target = %target_display, "[dry-run] would skip: target exists.");
        } else {
            tracing::info!(target = %target_display, "Target exists; skipping copy.");
        }
        return Ok(ExitStatus::Success);
    }

    let phase = run_copy_phase(config).await?;
    if phase.cancelled {
        return Ok(ExitStatus::Cancelled);
    }
    if let Err(e) = phase.transfer_result {
        error!(error = format!("{e:#}"), "copy failed.");
        return Err(e);
    }
    if phase.has_warning {
        return Ok(ExitStatus::Warning);
    }
    Ok(ExitStatus::Success)
}

/// Check whether the target already exists. For S3 targets this issues
/// HeadObject; for local targets this is a filesystem exists check. Stdio
/// targets are rejected by `validate_storage_config`, so they cannot reach
/// here — the match arm is `unreachable!`.
async fn target_exists(config: &Config) -> anyhow::Result<bool> {
    let (_, target_key) = extract_keys(config)?;
    match &config.target {
        StoragePath::S3 { bucket, .. } => {
            let target_client_config = config.target_client_config.as_ref().ok_or_else(|| {
                anyhow::anyhow!("internal error: target_client_config missing for s3 target")
            })?;
            let client = target_client_config.create_client().await;
            let opts = HeadObjectOpts {
                version_id: None,
                sse_c: config.target_sse_c.clone(),
                sse_c_key: config.target_sse_c_key.key.clone(),
                sse_c_key_md5: config.target_sse_c_key_md5.clone(),
                enable_additional_checksum: false,
            };
            match api::head_object(&client, bucket, &target_key, opts).await {
                Ok(_) => Ok(true),
                Err(HeadError::NotFound) | Err(HeadError::BucketNotFound) => Ok(false),
                Err(HeadError::Other(e)) => Err(e),
            }
        }
        StoragePath::Local(_) => Ok(tokio::fs::try_exists(&target_key).await?),
        StoragePath::Stdio => {
            unreachable!("validate_storage_config rejects --skip-existing with stdout target")
        }
    }
}
