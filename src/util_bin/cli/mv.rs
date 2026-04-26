// Vendored from s3util-rs@0.2.0
//   src/bin/s3util/cli/mv.rs
// Adjustments: stripped #[cfg(test)] mod tests; rewrote crate::cli → super

use anyhow::{Result, anyhow};
use tracing::{error, info};

use s3util_rs::Config;

use super::{CopyPhase, ExitStatus, run_copy_phase};

pub async fn run_mv(config: Config) -> Result<ExitStatus> {
    let phase = run_copy_phase(config.clone()).await?;
    apply_mv_decision_tree(config, phase).await
}

async fn apply_mv_decision_tree(config: Config, phase: CopyPhase) -> Result<ExitStatus> {
    // Gate 1: cancellation observed during/after transfer
    if phase.cancelled {
        return Ok(ExitStatus::Cancelled);
    }

    // Gate 2: transfer error
    let outcome = match phase.transfer_result {
        Ok(o) => o,
        Err(e) => {
            error!(error = format!("{e:#}"), "copy failed; source not deleted.");
            return Err(e);
        }
    };

    // Gate 3: verification warning
    if phase.has_warning && !config.no_fail_on_verify_error {
        let msg =
            "verification failed; source not deleted (use --no-fail-on-verify-error to override)";
        error!("{msg}");
        return Err(anyhow!(msg));
    }

    // Gate 4: defensive cancellation re-check (token may have flipped between
    // gate 1 and now if a SIGINT arrived while gate 2/3 were evaluating).
    if phase.cancellation_token.is_cancelled() {
        return Ok(ExitStatus::Cancelled);
    }

    // Resolve version-id: explicit user-supplied --source-version-id wins;
    // otherwise fall back to the value captured by the transfer.
    let version_id = config.version_id.clone().or(outcome.source_version_id);

    let version_id_for_log = version_id.clone().unwrap_or_default();
    match phase
        .source_storage
        .delete_object(&phase.source_key, version_id)
        .await
    {
        Ok(_) => {
            info!(
                key = %phase.source_key,
                version_id = %version_id_for_log,
                "Source delete completed."
            );
            Ok(ExitStatus::Success)
        }
        Err(e) => {
            error!(error = format!("{e:#}"), "source delete failed.");
            Err(e)
        }
    }
}
