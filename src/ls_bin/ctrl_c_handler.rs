// Vendored from s3ls-rs@0.4.1
//   src/bin/s3ls/ctrl_c_handler/mod.rs
// Adjustments: flattened from ctrl_c_handler/mod.rs to ctrl_c_handler.rs;
//              stripped #[cfg(test)] mod tests

// Ctrl+C signal handler adapted from s3sync's `bin/s3sync/cli/ctrl_c_handler/mod.rs`.
//
// Uses tokio::select! to wait for either pipeline cancellation or Ctrl+C signal.

use s3ls_rs::PipelineCancellationToken;
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, OnceLock};

    use s3ls_rs::create_pipeline_cancellation_token;
    use tokio::sync::Semaphore;

    use super::*;

    fn semaphore() -> Arc<Semaphore> {
        static SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();
        SEMAPHORE
            .get_or_init(|| Arc::new(Semaphore::new(1)))
            .clone()
    }

    #[tokio::test]
    async fn ctrl_c_handler_handles_cancellation_token() {
        let _permit = semaphore().acquire_owned().await.unwrap();

        let cancellation_token = create_pipeline_cancellation_token();
        let join_handle = spawn_ctrl_c_handler(cancellation_token.clone());
        cancellation_token.cancel();

        join_handle.await.unwrap();
        assert!(cancellation_token.is_cancelled());
    }

    #[tokio::test]
    #[cfg(target_family = "unix")]
    async fn ctrl_c_handler_handles_sigint() {
        const STARTUP_MS: u64 = 100;

        let _permit = semaphore().acquire_owned().await.unwrap();

        let cancellation_token = create_pipeline_cancellation_token();
        let join_handle = spawn_ctrl_c_handler(cancellation_token.clone());
        tokio::time::sleep(std::time::Duration::from_millis(STARTUP_MS)).await;

        nix::sys::signal::kill(nix::unistd::Pid::this(), nix::sys::signal::Signal::SIGINT).unwrap();

        join_handle.await.unwrap();
        assert!(cancellation_token.is_cancelled());
    }
}
