// Vendored from s3util-rs@0.2.0
//   src/bin/s3util/cli/mod.rs
// Adjustments: stripped #[cfg(test)] mod tests; commented out per-subcommand
//              pub mod declarations (re-enabled per task as files land);
//              kept ExitStatus, EXIT_CODE_*, run_copy_phase, build_rate_limiter.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::{Result, anyhow};
use aws_sdk_s3::types::RequestPayer;
use leaky_bucket::RateLimiter;
use tracing::trace;

use s3util_rs::Config;
use s3util_rs::storage::Storage;
use s3util_rs::storage::StorageFactory;
use s3util_rs::storage::local::LocalStorageFactory;
use s3util_rs::storage::s3::S3StorageFactory;
use s3util_rs::transfer::{TransferDirection, TransferOutcome, detect_direction};
use s3util_rs::types::StoragePath;
use s3util_rs::types::token::{PipelineCancellationToken, create_pipeline_cancellation_token};

pub mod ctrl_c_handler;
pub mod indicator;
pub mod tagging;
pub mod ui_config;

// Per-subcommand modules — uncomment as each is vendored in later tasks.
pub mod cp;
pub mod mv;
pub mod rm;
pub mod create_bucket;
pub mod delete_bucket;
pub mod head_bucket;
pub mod head_object;
// pub mod get_object_tagging;
// pub mod put_object_tagging;
// pub mod delete_object_tagging;
// pub mod get_bucket_tagging;
// pub mod put_bucket_tagging;
// pub mod delete_bucket_tagging;
// pub mod get_bucket_policy;
// pub mod put_bucket_policy;
// pub mod delete_bucket_policy;
// pub mod get_bucket_versioning;
// pub mod put_bucket_versioning;

pub use cp::run_cp;
pub use create_bucket::run_create_bucket;
pub use delete_bucket::run_delete_bucket;
// pub use delete_bucket_policy::run_delete_bucket_policy;
// pub use delete_bucket_tagging::run_delete_bucket_tagging;
// pub use delete_object_tagging::run_delete_object_tagging;
// pub use get_bucket_policy::run_get_bucket_policy;
// pub use get_bucket_tagging::run_get_bucket_tagging;
// pub use get_bucket_versioning::run_get_bucket_versioning;
// pub use get_object_tagging::run_get_object_tagging;
pub use head_bucket::run_head_bucket;
pub use head_object::run_head_object;
pub use mv::run_mv;
// pub use put_bucket_policy::run_put_bucket_policy;
// pub use put_bucket_tagging::run_put_bucket_tagging;
// pub use put_bucket_versioning::run_put_bucket_versioning;
// pub use put_object_tagging::run_put_object_tagging;
pub use rm::run_rm;

// Default refill interval is 100ms (= 10 refills per second).
const REFILL_PER_INTERVAL_DIVIDER: usize = 10;

fn build_rate_limiter(config: &Config) -> Option<Arc<RateLimiter>> {
    config.rate_limit_bandwidth.map(|bandwidth| {
        let refill = bandwidth as usize / REFILL_PER_INTERVAL_DIVIDER;
        Arc::new(
            RateLimiter::builder()
                .max(bandwidth as usize)
                .initial(bandwidth as usize)
                .refill(refill)
                .fair(true)
                .build(),
        )
    })
}

#[derive(Debug)]
pub enum ExitStatus {
    Success,
    Warning,
    NotFound,
    Cancelled,
}

impl ExitStatus {
    pub fn code(&self) -> i32 {
        match self {
            ExitStatus::Success => EXIT_CODE_SUCCESS,
            ExitStatus::Warning => EXIT_CODE_WARNING,
            ExitStatus::NotFound => EXIT_CODE_NOT_FOUND,
            ExitStatus::Cancelled => EXIT_CODE_CANCELLED,
        }
    }
}

pub const EXIT_CODE_SUCCESS: i32 = 0;
pub const EXIT_CODE_ERROR: i32 = 1;
pub const EXIT_CODE_WARNING: i32 = 3;
// Returned by the head-* subcommands when the target does not exist
// (HeadBucket / HeadObject service error reports `is_not_found()`).
pub const EXIT_CODE_NOT_FOUND: i32 = 4;
// Standard Unix convention for processes terminated by SIGINT (128 + 2).
pub const EXIT_CODE_CANCELLED: i32 = 130;

/// Intermediate state produced by [`run_copy_phase`].
///
/// [`run_cp`] and [`run_mv`] translate this into an [`ExitStatus`] / cleanup
/// decision. The clone of the source `Storage` is kept so callers can reuse
/// the same factory-built instance for follow-up operations (e.g. `mv`'s
/// post-transfer delete) without rebuilding it.
pub struct CopyPhase {
    pub transfer_result: Result<TransferOutcome>,
    pub source_storage: Storage,
    pub source_key: String,
    pub cancellation_token: PipelineCancellationToken,
    pub cancelled: bool,
    pub has_warning: bool,
}

/// Run the copy pipeline (cancellation token, indicator, transfer dispatch,
/// teardown). Returns enough state for callers (`run_cp`, `run_mv`) to decide
/// what exit code to produce.
pub async fn run_copy_phase(config: Config) -> Result<CopyPhase> {
    let cancellation_token = create_pipeline_cancellation_token();
    ctrl_c_handler::spawn_ctrl_c_handler(cancellation_token.clone());

    let (stats_sender, stats_receiver) = async_channel::unbounded();

    // Determine transfer direction
    let (source_str, target_str) = get_path_strings(&config.source, &config.target);
    let direction = detect_direction(&source_str, &target_str)?;

    trace!(direction = ?direction, "detected transfer direction");

    check_local_source_not_directory(&config.source, &direction)?;

    // For cp, the full path is always passed as the key to get_object/put_object.
    // Storage instances are created with an empty base path so that key = full path.
    let (source_key, target_key) = extract_keys(&config)?;

    // When `extract_keys` resolved the target to a different path than the user
    // typed (bare `s3://bucket`, S3 prefix ending in `/`, or a directory-style
    // local target), surface the resolved path in the indicator.
    let resolved_target_display = resolve_target_display(&config.target, &target_str, &target_key);

    let show_progress = ui_config::is_progress_indicator_needed(&config);
    let show_result = ui_config::is_show_result_needed(&config);
    let log_sync_summary = config.tracing_config.is_some();

    // Start indicator
    let indicator_handle = indicator::show_indicator(
        stats_receiver,
        show_progress,
        show_result,
        log_sync_summary,
        resolved_target_display,
        source_key.clone(),
        target_key.clone(),
    );

    let has_warning = Arc::new(AtomicBool::new(false));
    let rate_limit_bandwidth = build_rate_limiter(&config);

    // Each direction builds the source `Storage` it consumes (for transfer)
    // plus a sibling clone (`source_for_caller`) that survives so callers
    // such as `run_mv` can reuse it for follow-up operations. Stdio sources
    // never reach mv, but the type still requires *some* storage — a no-op
    // LocalStorage placeholder fills that slot.
    let (transfer_result, source_for_caller) = match direction {
        TransferDirection::LocalToS3 => {
            let target_request_payer = if config.target_request_payer {
                Some(RequestPayer::Requester)
            } else {
                None
            };

            let source = LocalStorageFactory::create(
                config.clone(),
                empty_local_storage_path(),
                cancellation_token.clone(),
                stats_sender.clone(),
                None,
                None,
                rate_limit_bandwidth.clone(),
                has_warning.clone(),
                None,
            )
            .await;
            let source_for_caller = dyn_clone::clone_box(&*source);

            let target = S3StorageFactory::create(
                config.clone(),
                empty_s3_storage_path(&config.target),
                cancellation_token.clone(),
                stats_sender.clone(),
                config.target_client_config.clone(),
                target_request_payer,
                rate_limit_bandwidth.clone(),
                has_warning.clone(),
                None,
            )
            .await;

            let result = s3util_rs::transfer::local_to_s3::transfer(
                &config,
                source,
                target,
                &source_key,
                &target_key,
                cancellation_token.clone(),
                stats_sender.clone(),
            )
            .await;
            (result, source_for_caller)
        }
        TransferDirection::S3ToLocal => {
            let source_request_payer = if config.source_request_payer {
                Some(RequestPayer::Requester)
            } else {
                None
            };

            let source = S3StorageFactory::create(
                config.clone(),
                empty_s3_storage_path(&config.source),
                cancellation_token.clone(),
                stats_sender.clone(),
                config.source_client_config.clone(),
                source_request_payer,
                rate_limit_bandwidth.clone(),
                has_warning.clone(),
                None,
            )
            .await;
            let source_for_caller = dyn_clone::clone_box(&*source);

            let target = LocalStorageFactory::create(
                config.clone(),
                empty_local_storage_path(),
                cancellation_token.clone(),
                stats_sender.clone(),
                None,
                None,
                rate_limit_bandwidth.clone(),
                has_warning.clone(),
                None,
            )
            .await;

            let result = s3util_rs::transfer::s3_to_local::transfer(
                &config,
                source,
                target,
                &source_key,
                &target_key,
                cancellation_token.clone(),
                stats_sender.clone(),
            )
            .await;
            (result, source_for_caller)
        }
        TransferDirection::S3ToS3 => {
            let source_request_payer = if config.source_request_payer {
                Some(RequestPayer::Requester)
            } else {
                None
            };
            let target_request_payer = if config.target_request_payer {
                Some(RequestPayer::Requester)
            } else {
                None
            };

            let source = S3StorageFactory::create(
                config.clone(),
                empty_s3_storage_path(&config.source),
                cancellation_token.clone(),
                stats_sender.clone(),
                config.source_client_config.clone(),
                source_request_payer,
                rate_limit_bandwidth.clone(),
                has_warning.clone(),
                None,
            )
            .await;
            let source_for_caller = dyn_clone::clone_box(&*source);

            let target = S3StorageFactory::create(
                config.clone(),
                empty_s3_storage_path(&config.target),
                cancellation_token.clone(),
                stats_sender.clone(),
                config.target_client_config.clone(),
                target_request_payer,
                rate_limit_bandwidth.clone(),
                has_warning.clone(),
                None,
            )
            .await;

            let result = s3util_rs::transfer::s3_to_s3::transfer(
                &config,
                source,
                target,
                &source_key,
                &target_key,
                cancellation_token.clone(),
                stats_sender.clone(),
            )
            .await;
            (result, source_for_caller)
        }
        TransferDirection::StdioToS3 => {
            let target_request_payer = if config.target_request_payer {
                Some(RequestPayer::Requester)
            } else {
                None
            };

            let target = S3StorageFactory::create(
                config.clone(),
                empty_s3_storage_path(&config.target),
                cancellation_token.clone(),
                stats_sender.clone(),
                config.target_client_config.clone(),
                target_request_payer,
                rate_limit_bandwidth.clone(),
                has_warning.clone(),
                None,
            )
            .await;

            // Stdio sources never reach run_mv (mv rejects stdio at config
            // validation). Use a placeholder LocalStorage so CopyPhase always
            // owns a valid Storage instance.
            let source_for_caller = LocalStorageFactory::create(
                config.clone(),
                empty_local_storage_path(),
                cancellation_token.clone(),
                stats_sender.clone(),
                None,
                None,
                rate_limit_bandwidth.clone(),
                has_warning.clone(),
                None,
            )
            .await;

            let result = s3util_rs::transfer::stdio_to_s3::transfer(
                &config,
                target,
                &target_key,
                tokio::io::stdin(),
                cancellation_token.clone(),
                stats_sender.clone(),
            )
            .await;
            (result, source_for_caller)
        }
        TransferDirection::S3ToStdio => {
            let source_request_payer = if config.source_request_payer {
                Some(RequestPayer::Requester)
            } else {
                None
            };

            let source = S3StorageFactory::create(
                config.clone(),
                empty_s3_storage_path(&config.source),
                cancellation_token.clone(),
                stats_sender.clone(),
                config.source_client_config.clone(),
                source_request_payer,
                rate_limit_bandwidth.clone(),
                has_warning.clone(),
                None,
            )
            .await;
            let source_for_caller = dyn_clone::clone_box(&*source);

            let result = s3util_rs::transfer::s3_to_stdio::transfer(
                &config,
                source,
                &source_key,
                tokio::io::stdout(),
                cancellation_token.clone(),
                stats_sender.clone(),
            )
            .await;
            (result, source_for_caller)
        }
    };

    // Send error stat if transfer failed, so indicator can suppress summary
    if transfer_result.is_err() {
        let _ = stats_sender
            .send(s3util_rs::types::SyncStatistics::SyncError { key: String::new() })
            .await;
    }

    // Close stats channel to signal indicator to finish
    stats_sender.close();

    // Wait for indicator to finish
    let _ = indicator_handle.await;

    // ctrl_c_handler is the only code path that cancels this token, so an
    // observed cancellation means SIGINT was received. Snapshot the token
    // here so callers (run_cp's wrapper, run_mv) see the same bool the
    // pipeline did.
    let cancelled = cancellation_token.is_cancelled();
    let has_warning = has_warning.load(std::sync::atomic::Ordering::SeqCst);

    Ok(CopyPhase {
        transfer_result,
        source_storage: source_for_caller,
        source_key,
        cancellation_token,
        cancelled,
        has_warning,
    })
}

fn get_path_strings(source: &StoragePath, target: &StoragePath) -> (String, String) {
    let source_str = match source {
        StoragePath::S3 { bucket, prefix } => {
            if prefix.is_empty() {
                format!("s3://{}", bucket)
            } else {
                format!("s3://{}/{}", bucket, prefix)
            }
        }
        StoragePath::Local(path) => path.to_string_lossy().to_string(),
        StoragePath::Stdio => "-".to_string(),
    };
    let target_str = match target {
        StoragePath::S3 { bucket, prefix } => {
            if prefix.is_empty() {
                format!("s3://{}", bucket)
            } else {
                format!("s3://{}/{}", bucket, prefix)
            }
        }
        StoragePath::Local(path) => path.to_string_lossy().to_string(),
        StoragePath::Stdio => "-".to_string(),
    };
    (source_str, target_str)
}

/// Extract the full path as the key for each side.
/// For cp, the full path is always passed to get_object/put_object.
/// Storage instances are created with empty base paths.
fn extract_keys(config: &Config) -> Result<(String, String)> {
    let source_key = match &config.source {
        StoragePath::S3 { prefix, .. } => {
            if prefix.is_empty() {
                return Err(anyhow!("source S3 key is required (e.g. s3://bucket/key)"));
            }
            prefix.clone()
        }
        StoragePath::Local(path) => path.to_string_lossy().to_string(),
        StoragePath::Stdio => String::new(),
    };
    let source_basename = std::path::Path::new(&source_key)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or(source_key.clone());

    let target_key = match &config.target {
        StoragePath::S3 { prefix, .. } => {
            // If target is empty or ends with '/', treat as directory prefix — append source basename
            if prefix.is_empty() || prefix.ends_with('/') {
                // With a stdin source there's no basename to derive, so the user must
                // spell the target key explicitly (e.g. `s3://bucket/key`).
                if source_basename.is_empty() {
                    return Err(anyhow!(
                        "target S3 key is required when source is stdin (e.g. s3://bucket/key)"
                    ));
                }
                format!("{prefix}{source_basename}")
            } else {
                prefix.clone()
            }
        }
        StoragePath::Local(path) => {
            let p = path.clone();
            // If target is a directory (existing dir or ends with separator),
            // append the source object's basename — like `aws s3 cp s3://bucket/key .`
            if p.is_dir() || p.to_string_lossy().ends_with(std::path::MAIN_SEPARATOR) {
                p.join(&source_basename).to_string_lossy().to_string()
            } else {
                p.to_string_lossy().to_string()
            }
        }
        StoragePath::Stdio => String::new(),
    };
    Ok((source_key, target_key))
}

/// Format the resolved target path for display.
fn format_target_path(target: &StoragePath, target_key: &str) -> String {
    match target {
        StoragePath::S3 { bucket, .. } => format!("s3://{bucket}/{target_key}"),
        StoragePath::Local(_) => target_key.to_string(),
        StoragePath::Stdio => "-".to_string(),
    }
}

/// Build the display string for the resolved target path, if and only if
/// it differs from what the user typed. When `extract_keys` appends a
/// source basename (directory-style local targets, bare S3 buckets, S3
/// prefixes ending in `/`), the resolved path is surfaced so the user
/// sees where the data actually lands.
fn resolve_target_display(
    target: &StoragePath,
    target_str: &str,
    target_key: &str,
) -> Option<String> {
    let resolved = format_target_path(target, target_key);
    if resolved != target_str {
        Some(resolved)
    } else {
        None
    }
}

/// Reject local source directories for `cp`.
///
/// LocalStorage::head_object returns a 0-byte success for directories (inherited
/// from s3sync's recursive-sync semantics). `s3util cp` is single-file only, so
/// without this guard a command like `s3util cp /tmp/ s3://bucket/` would silently
/// upload an empty object.
fn check_local_source_not_directory(
    source: &StoragePath,
    direction: &TransferDirection,
) -> Result<()> {
    if !matches!(direction, TransferDirection::LocalToS3) {
        return Ok(());
    }
    if let StoragePath::Local(path) = source
        && path.is_dir()
    {
        return Err(anyhow!(
            "source is a directory: {}. cp command copies a single file.",
            path.display()
        ));
    }
    Ok(())
}

/// Create a LocalStorage base path (empty — full path is passed as the key).
fn empty_local_storage_path() -> StoragePath {
    StoragePath::Local(".".into())
}

/// Create an S3Storage base path with empty prefix (full key is passed to operations).
fn empty_s3_storage_path(original: &StoragePath) -> StoragePath {
    match original {
        StoragePath::S3 { bucket, .. } => StoragePath::S3 {
            bucket: bucket.clone(),
            prefix: String::new(),
        },
        _ => unreachable!("expected S3 storage path"),
    }
}
