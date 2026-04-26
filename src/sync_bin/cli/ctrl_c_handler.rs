// Vendored from s3sync@1.57.1
//   src/bin/s3sync/cli/ctrl_c_handler/mod.rs
// Adjustments: stripped #[cfg(test)] mod tests

use tokio::task::JoinHandle;
use tokio::{select, signal};
use tracing::{debug, error, warn};

use s3sync::types::token::PipelineCancellationToken;

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
