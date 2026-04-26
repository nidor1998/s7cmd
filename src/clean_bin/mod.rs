// Vendored from s3rm-rs@1.3.3
//   src/bin/s3rm/main.rs
// Adjustments: removed #[tokio::main] async fn main() (s7cmd has its own);
//              stripped #[cfg(test)] indicator_properties module declaration
//              and any test blocks; helpers (load_config_exit_if_err,
//              start_tracing_if_necessary, run, EXIT_CODE_*) made pub for
//              dispatch from s7cmd::main; load_config_exit_if_err takes args
//              by value instead of calling parse() (s7cmd parses at top level)

use anyhow::Result;
use tracing::{debug, error};

use s3rm_rs::callback::user_defined_event_callback::UserDefinedEventCallback;
use s3rm_rs::callback::user_defined_filter_callback::UserDefinedFilterCallback;
use s3rm_rs::config::Config;
use s3rm_rs::types::event_callback::EventType;
use s3rm_rs::{
    CLIArgs, DeletionPipeline, create_pipeline_cancellation_token, exit_code_from_error,
    is_cancelled_error,
};

pub mod ctrl_c_handler;
pub mod indicator;
mod tracing_init;
pub mod ui_config;

pub const EXIT_CODE_WARNING: i32 = 3;
pub const EXIT_CODE_ABNORMAL_TERMINATION: i32 = 101;

// Adjusted from upstream: takes args by value instead of calling
// CLIArgs::parse() internally (s7cmd parses at the top level).
pub fn load_config_exit_if_err(args: CLIArgs) -> Config {
    match Config::try_from(args) {
        Ok(config) => config,
        Err(error_message) => {
            clap::Error::raw(clap::error::ErrorKind::ValueValidation, error_message).exit();
        }
    }
}

pub fn start_tracing_if_necessary(config: &Config) -> bool {
    if let Some(tracing_config) = config.tracing_config.as_ref() {
        tracing_init::init_tracing(tracing_config);
        true
    } else {
        false
    }
}

fn register_user_defined_callbacks(config: &mut Config) {
    // Note: Each type of callback is registered only once.
    // The user-defined event callback is disabled by default.
    let mut user_defined_event_callback = UserDefinedEventCallback::new();
    // This is for testing purpose only.
    if config.test_user_defined_callback {
        user_defined_event_callback.enable = true;
    }
    if user_defined_event_callback.is_enabled() {
        // By default, the user-defined event callback notifies all events.
        // You can modify EventType::ALL_EVENTS to filter specific events
        config.event_manager.register_callback(
            EventType::ALL_EVENTS,
            user_defined_event_callback,
            config.dry_run,
        );
    }

    // The user-defined filter callback is disabled by default.
    // But you can modify the `UserDefinedFilterCallback` to enable it.
    // User-defined filter callback allows us to filter objects while listing them.
    let mut user_defined_filter_callback = UserDefinedFilterCallback::new();
    // This is for testing purpose only.
    if config.test_user_defined_callback {
        user_defined_filter_callback.enable = true;
    }
    if user_defined_filter_callback.is_enabled() {
        config
            .filter_manager
            .register_callback(user_defined_filter_callback);
    }
}

pub async fn run(mut config: Config) -> Result<()> {
    register_user_defined_callbacks(&mut config);

    #[allow(unused_assignments)]
    let mut has_warning = false;

    {
        let cancellation_token = create_pipeline_cancellation_token();

        let start_time = tokio::time::Instant::now();
        debug!("deletion pipeline start.");

        let mut pipeline = DeletionPipeline::new(config.clone(), cancellation_token.clone()).await;

        // Check prerequisites (confirmation prompt) before starting the indicator,
        // so the progress bar doesn't interfere with the prompt.
        // The Ctrl+C handler is spawned AFTER this so that the default OS
        // SIGINT handler remains active during the blocking stdin read,
        // allowing Ctrl+C to terminate the process immediately at the prompt.
        if let Err(e) = pipeline.check_prerequisites().await {
            pipeline.close_stats_sender();
            if is_cancelled_error(&e) {
                println!("Deletion cancelled.");
                debug!("deletion cancelled by user.");
                return Ok(());
            }
            let code = exit_code_from_error(&e);
            error!("{}", e);
            std::process::exit(code);
        }

        // Now that the blocking prompt is done, install the async Ctrl+C
        // handler for graceful pipeline shutdown.
        ctrl_c_handler::spawn_ctrl_c_handler(cancellation_token);

        let indicator_join_handle = indicator::show_indicator(
            pipeline.get_stats_receiver(),
            ui_config::is_progress_indicator_needed(&config),
            ui_config::is_show_result_needed(&config),
            config.dry_run,
        );

        pipeline.run().await;
        match indicator_join_handle.await {
            Ok(_summary) => {}
            Err(e) => {
                error!("indicator task panicked: {}", e);
                std::process::exit(EXIT_CODE_ABNORMAL_TERMINATION);
            }
        }

        let duration_sec = format!("{:.3}", start_time.elapsed().as_secs_f32());

        if pipeline.has_error() {
            if pipeline.has_panic() {
                error!(duration_sec = duration_sec, "s3rm abnormal termination.");
                std::process::exit(EXIT_CODE_ABNORMAL_TERMINATION);
            }
            let Some(errors) = pipeline.get_errors_and_consume() else {
                // has_error() was true but no errors found — should not happen.
                error!(duration_sec = duration_sec, "s3rm failed.");
                std::process::exit(1);
            };
            // Use the highest exit code across all errors so that a severe
            // status (e.g. 3 for PartialFailure) is not downgraded by a
            // subsequent generic error (code 1).
            let mut code = 1;
            for err in &errors {
                if is_cancelled_error(err) {
                    debug!("deletion cancelled by user.");
                    return Ok(());
                }
                code = code.max(exit_code_from_error(err));
                error!("{}", err);
            }
            error!(duration_sec = duration_sec, "s3rm failed.");
            std::process::exit(code);
        }

        has_warning = pipeline.has_warning();

        debug!(duration_sec = duration_sec, "s3rm has been completed.");
    }

    if has_warning {
        std::process::exit(EXIT_CODE_WARNING);
    }

    Ok(())
}
