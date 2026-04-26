// Vendored from s3rm-rs@1.3.3
//   src/bin/s3rm/ctrl_c_handler/mod.rs
// Adjustments: flattened from ctrl_c_handler/mod.rs to ctrl_c_handler.rs;
//              stripped #[cfg(test)] mod tests

// Ctrl+C signal handler adapted from s3sync's `bin/s3sync/cli/ctrl_c_handler/mod.rs`.
//
// Uses tokio::select! to wait for either pipeline cancellation or Ctrl+C signal.

use s3rm_rs::PipelineCancellationToken;
use tokio::task::JoinHandle;
use tokio::{select, signal};
use tracing::{debug, error};

pub fn spawn_ctrl_c_handler(cancellation_token: PipelineCancellationToken) -> JoinHandle<()> {
    tokio::spawn(async move {
        select! {
            _ = cancellation_token.cancelled() => {
                debug!("cancellation_token canceled.")
            }
            result = signal::ctrl_c() => {
                match result {
                    Ok(()) => {
                        debug!("ctrl-c received, shutting down.");
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
