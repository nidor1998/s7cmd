// s7cmd entry point. Dispatch arms for each subcommand are direct
// transcriptions of the corresponding match arms in
//   s3sync@1.57.1/src/bin/s3sync/main.rs   (sync arm)
//   s3util-rs@0.2.0/src/bin/s3util/main.rs (all other arms)
// Adjustments: program name "s3sync"/"s3util" → "s7cmd";
//              Cli::command() refers to s7cmd's Cli, not the upstream's.

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::generate;

mod cli;
mod sync_bin;

use cli::{Cli, Cmd};

#[tokio::main]
async fn main() -> Result<()> {
    let cli_args = Cli::parse();
    match cli_args.command {
        // ── port of s3sync's main.rs ───────────────────────────
        Cmd::Sync(boxed_args) => {
            let mut config = match s3sync::Config::try_from(*boxed_args) {
                Ok(c) => c,
                Err(msg) => clap::Error::raw(
                    clap::error::ErrorKind::ValueValidation, msg).exit(),
            };
            // s3sync's main: when reporting sync status, force dry_run=true.
            if config.report_sync_status {
                config.dry_run = true;
            }
            if let Some(shell) = config.auto_complete_shell {
                generate(shell, &mut Cli::command(), "s7cmd",
                    &mut std::io::stdout());
                return Ok(());
            }
            if let Some(tc) = &config.tracing_config {
                sync_bin::tracing::init_tracing(tc);
            }
            tracing::trace!("config = {:?}", config);
            // sync_bin::cli::run handles ctrl-c, pipeline, indicator, and
            // exits the process with EXIT_CODE_WARNING (3) on warning.
            // Errors propagate up; anyhow → main returns Err → exit 1.
            sync_bin::cli::run(config).await
        }
    }
}
