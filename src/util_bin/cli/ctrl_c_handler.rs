// Vendored from s3util-rs@0.2.0
//   src/bin/s3util/cli/ctrl_c_handler.rs
// Adjustments: stripped #[cfg(test)] mod tests

use tokio::task::JoinHandle;
use tokio::{select, signal};
use tracing::{debug, error, warn};

use s3util_rs::types::token::PipelineCancellationToken;

pub fn spawn_ctrl_c_handler(cancellation_token: PipelineCancellationToken) -> JoinHandle<()> {
    tokio::spawn(async move {
        select! {
            _ = cancellation_token.cancelled() => {
                debug!("cancellation_token canceled.")
            }
            result = signal::ctrl_c() => {
                match result {
                    Ok(()) => {
                        warn!("ctrl-c received, shutting down.");
                        cancellation_token.cancel();
                    }
                    Err(e) => {
                        error!("failed to listen for ctrl-c signal: {e}");
                    }
                }
            }
        }
    })
}
