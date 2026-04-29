// s7cmd entry point. Dispatch arms for each subcommand are direct
// transcriptions of the corresponding match arms in
//   s3sync@1.57.1/src/bin/s3sync/main.rs   (sync arm)
//   s3util-rs@0.2.0/src/bin/s3util/main.rs (all other arms)
// Adjustments: program name "s3sync"/"s3util" → "s7cmd";
//              Cli::command() refers to s7cmd's Cli, not the upstream's.

use anyhow::Result;
use clap::FromArgMatches;
use clap_complete::generate;

mod clean_bin;
mod cli;
mod ls_bin;
mod sync_bin;
mod util_bin;

use cli::{Cli, Cmd, cli_command};

#[tokio::main]
async fn main() -> Result<()> {
    let cli_args = match Cli::from_arg_matches(&cli_command().get_matches()) {
        Ok(c) => c,
        Err(e) => e.exit(),
    };

    // --auto-complete-shell only works at the top level: generate completions
    // for the whole s7cmd CLI (all subcommands) and exit. The per-subcommand
    // form is stripped in `cli_command()` (each upstream args struct still
    // declares the field, but we clear the long name so the parser rejects
    // `s7cmd <sub> --auto-complete-shell ...`).
    if let Some(shell) = cli_args.auto_complete_shell {
        generate(shell, &mut cli_command(), "s7cmd", &mut std::io::stdout());
        return Ok(());
    }

    // arg_required_else_help on the Cli ensures we always have a command
    // by this point — but the type is Option<Cmd>, so unwrap explicitly.
    let command = cli_args
        .command
        .expect("clap's arg_required_else_help should have prevented this");

    match command {
        // ── port of s3sync's main.rs ───────────────────────────
        Cmd::Sync(boxed_args) => {
            let mut config = match s3sync::Config::try_from(*boxed_args) {
                Ok(c) => c,
                Err(msg) => clap::Error::raw(clap::error::ErrorKind::ValueValidation, msg).exit(),
            };
            // s3sync's main: when reporting sync status, force dry_run=true.
            if config.report_sync_status {
                config.dry_run = true;
            }
            if let Some(tc) = &config.tracing_config {
                sync_bin::tracing::init_tracing(tc);
            }
            tracing::trace!(target: "s3sync", "config = {:?}", config);
            // sync_bin::cli::run handles ctrl-c, pipeline, indicator, and
            // exits the process with EXIT_CODE_WARNING (3) on warning.
            // Errors propagate up; anyhow → main returns Err → exit 1.
            sync_bin::cli::run(config).await
        }
        // ── port of s3ls-rs's main.rs ──────────────────────────
        Cmd::Ls(boxed_args) => {
            let config = ls_bin::load_config_exit_if_err(*boxed_args);
            ls_bin::start_tracing_if_necessary(&config);
            tracing::trace!(target: "s3ls", "config = {:?}", config);
            ls_bin::run(config).await
        }
        // ── port of s3rm-rs's main.rs ──────────────────────────
        Cmd::Clean(boxed_args) => {
            let config = clean_bin::load_config_exit_if_err(*boxed_args);
            clean_bin::start_tracing_if_necessary(&config);
            tracing::trace!(target: "s3rm", "config = {:?}", config);
            clean_bin::run(config).await
        }
        // ── ports of s3util-rs's main.rs match arms ─────────────
        Cmd::Cp(args) => {
            let config = match s3util_rs::Config::try_from(args) {
                Ok(c) => c,
                Err(msg) => clap::Error::raw(clap::error::ErrorKind::ValueValidation, msg).exit(),
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
            let config = match s3util_rs::Config::try_from(args) {
                Ok(c) => c,
                Err(msg) => clap::Error::raw(clap::error::ErrorKind::ValueValidation, msg).exit(),
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
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_delete_object_tagging(args, client_config).await {
                    Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::GetBucketTagging(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_get_bucket_tagging(args, client_config).await {
                Ok(status) => status.code(),
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::PutBucketTagging(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_put_bucket_tagging(args, client_config).await {
                Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::DeleteBucketTagging(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_delete_bucket_tagging(args, client_config).await {
                    Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::GetBucketPolicy(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_get_bucket_policy(args, client_config).await {
                Ok(status) => status.code(),
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::PutBucketPolicy(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_put_bucket_policy(args, client_config).await {
                Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::DeleteBucketPolicy(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_delete_bucket_policy(args, client_config).await
            {
                Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::GetBucketVersioning(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_get_bucket_versioning(args, client_config).await {
                    Ok(status) => status.code(),
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::PutBucketVersioning(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_put_bucket_versioning(args, client_config).await {
                    Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::GetBucketLifecycleConfiguration(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_get_bucket_lifecycle_configuration(args, client_config)
                    .await
                {
                    Ok(status) => status.code(),
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::PutBucketLifecycleConfiguration(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_put_bucket_lifecycle_configuration(args, client_config)
                    .await
                {
                    Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::DeleteBucketLifecycleConfiguration(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_delete_bucket_lifecycle_configuration(args, client_config)
                    .await
                {
                    Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::GetBucketEncryption(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_get_bucket_encryption(args, client_config).await {
                    Ok(status) => status.code(),
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::PutBucketEncryption(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_put_bucket_encryption(args, client_config).await {
                    Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::DeleteBucketEncryption(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_delete_bucket_encryption(args, client_config).await {
                    Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::GetBucketCors(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_get_bucket_cors(args, client_config).await {
                Ok(status) => status.code(),
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::PutBucketCors(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_put_bucket_cors(args, client_config).await {
                Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::DeleteBucketCors(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_delete_bucket_cors(args, client_config).await {
                Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::GetPublicAccessBlock(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_get_public_access_block(args, client_config).await {
                    Ok(status) => status.code(),
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::PutPublicAccessBlock(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_put_public_access_block(args, client_config).await {
                    Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::DeletePublicAccessBlock(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_delete_public_access_block(args, client_config).await {
                    Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::GetBucketWebsite(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_get_bucket_website(args, client_config).await {
                Ok(status) => status.code(),
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::PutBucketWebsite(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_put_bucket_website(args, client_config).await {
                Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::DeleteBucketWebsite(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_delete_bucket_website(args, client_config).await {
                    Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::GetBucketLogging(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_get_bucket_logging(args, client_config).await {
                Ok(status) => status.code(),
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::PutBucketLogging(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code = match util_bin::cli::run_put_bucket_logging(args, client_config).await {
                Ok(()) => util_bin::cli::EXIT_CODE_SUCCESS,
                Err(e) => {
                    tracing::error!(error = format!("{e:#}"));
                    util_bin::cli::EXIT_CODE_ERROR
                }
            };
            std::process::exit(exit_code);
        }
        Cmd::GetBucketNotificationConfiguration(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_get_bucket_notification_configuration(args, client_config)
                    .await
                {
                    Ok(status) => status.code(),
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"));
                        util_bin::cli::EXIT_CODE_ERROR
                    }
                };
            std::process::exit(exit_code);
        }
        Cmd::PutBucketNotificationConfiguration(args) => {
            let tracing_config = args.common.build_tracing_config();
            if let Some(tc) = &tracing_config {
                util_bin::tracing_init::init_tracing(tc);
            }
            let client_config = args.common.build_client_config();
            let exit_code =
                match util_bin::cli::run_put_bucket_notification_configuration(args, client_config)
                    .await
                {
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
        target: "s3util",
        "config = {{ source: {:?}, target: {:?}, transfer_config: {:?}, server_side_copy: {}, version_id: {:?} }}",
        config.source,
        config.target,
        config.transfer_config,
        config.server_side_copy,
        config.version_id,
    );
}
