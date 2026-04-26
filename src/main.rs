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
mod util_bin;

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
        // ── ports of s3util-rs's main.rs match arms ─────────────
        Cmd::Cp(args) => {
            if let Some(shell) = args.auto_complete_shell() {
                generate(shell, &mut Cli::command(), "s7cmd",
                    &mut std::io::stdout());
                return Ok(());
            }
            let config = match s3util_rs::Config::try_from(args) {
                Ok(c) => c,
                Err(msg) => clap::Error::raw(
                    clap::error::ErrorKind::ValueValidation, msg).exit(),
            };
            start_tracing_if_necessary(&config);
            trace_config_summary(&config);
            let exit_code = match util_bin::cli::run_cp(config).await {
                Ok(status) => status.code(),
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::Mv(args) => {
            if let Some(shell) = args.auto_complete_shell() {
                generate(shell, &mut Cli::command(), "s7cmd",
                    &mut std::io::stdout());
                return Ok(());
            }
            let config = match s3util_rs::Config::try_from(args) {
                Ok(c) => c,
                Err(msg) => clap::Error::raw(
                    clap::error::ErrorKind::ValueValidation, msg).exit(),
            };
            start_tracing_if_necessary(&config);
            trace_config_summary(&config);
            let exit_code = match util_bin::cli::run_mv(config).await {
                Ok(status) => status.code(),
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::Rm(args) => {
            if let Some(shell) = args.auto_complete_shell() {
                generate(shell, &mut Cli::command(), "s7cmd",
                    &mut std::io::stdout());
                return Ok(());
            }
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_rm(args, client_config).await {
                Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::CreateBucket(args) => {
            if let Some(shell) = args.auto_complete_shell() {
                generate(shell, &mut Cli::command(), "s7cmd",
                    &mut std::io::stdout());
                return Ok(());
            }
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_create_bucket(args, client_config).await {
                Ok(status) => status.code(),
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::DeleteBucket(args) => {
            if let Some(shell) = args.auto_complete_shell() {
                generate(shell, &mut Cli::command(), "s7cmd",
                    &mut std::io::stdout());
                return Ok(());
            }
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_delete_bucket(args, client_config).await {
                Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::HeadBucket(args) => {
            if let Some(shell) = args.auto_complete_shell() {
                generate(shell, &mut Cli::command(), "s7cmd",
                    &mut std::io::stdout());
                return Ok(());
            }
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_head_bucket(args, client_config).await {
                Ok(status) => status.code(),
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::HeadObject(args) => {
            if let Some(shell) = args.auto_complete_shell() {
                generate(shell, &mut Cli::command(), "s7cmd",
                    &mut std::io::stdout());
                return Ok(());
            }
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_head_object(args, client_config).await {
                Ok(status) => status.code(),
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::GetObjectTagging(args) => {
            if let Some(shell) = args.auto_complete_shell() {
                generate(shell, &mut Cli::command(), "s7cmd",
                    &mut std::io::stdout());
                return Ok(());
            }
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_get_object_tagging(args, client_config).await {
                Ok(status) => status.code(),
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::PutObjectTagging(args) => {
            if let Some(shell) = args.auto_complete_shell() {
                generate(shell, &mut Cli::command(), "s7cmd",
                    &mut std::io::stdout());
                return Ok(());
            }
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_put_object_tagging(args, client_config).await {
                Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::DeleteObjectTagging(args) => {
            if let Some(shell) = args.auto_complete_shell() {
                generate(shell, &mut Cli::command(), "s7cmd",
                    &mut std::io::stdout());
                return Ok(());
            }
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_delete_object_tagging(args, client_config).await {
                Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
    }
}

// ── Vendored from s3util-rs@0.2.0/src/bin/s3util/main.rs ─────────────
fn start_tracing_if_necessary(config: &s3util_rs::Config) -> bool {
    if config.tracing_config.is_none() {
        return false;
    }
    util_bin::tracing_init::init_tracing(config.tracing_config.as_ref().unwrap());
    true
}

// Trace only non-sensitive summary fields. Avoids `{:?}` on the full Config,
// which would risk leaking credentials or SSE-C key material if a future field
// is added without a redacting Debug impl.
fn trace_config_summary(config: &s3util_rs::Config) {
    tracing::trace!(
        "config = {{ source: {:?}, target: {:?}, transfer_config: {:?}, server_side_copy: {}, version_id: {:?} }}",
        config.source,
        config.target,
        config.transfer_config,
        config.server_side_copy,
        config.version_id,
    );
}
