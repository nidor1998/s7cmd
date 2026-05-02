use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::shells::Shell;
use clap_verbosity_flag::{Verbosity, WarnLevel};

#[cfg(feature = "version")]
use shadow_rs::shadow;

#[cfg(feature = "version")]
shadow!(build);

#[derive(Parser, Clone, Debug)]
#[cfg_attr(
    feature = "version",
    command(version = format!(
        "{} ({} {}), {}",
        build::PKG_VERSION,
        build::SHORT_COMMIT,
        build::BUILD_TARGET,
        build::RUST_VERSION
    ))
)]
#[cfg_attr(not(feature = "version"), command(version))]
#[command(
    name = "s7cmd",
    about = "Reliable, flexible, and fast command-line tool for Amazon S3",
    arg_required_else_help = true,
    // clap 4.6 has no per-subcommand help_heading in derive, so the grouped
    // listing below is static. The order/names must match the `Cmd` enum
    // variants — `tests/cli_help.rs::top_level_help_*` exercise the basics,
    // but adding/renaming a subcommand requires a manual edit here.
    help_template = "\
{about-with-newline}
{usage-heading} {usage}

Object Operations:
  ls                                    List S3 objects
  cp                                    Copy objects from/to S3 (or S3 to S3)
  mv                                    Move objects from/to S3 (copy then delete source)
  rm                                    Delete a single S3 object
  sync                                  Synchronize files between local and S3 (or S3 to S3)
  clean                                 Bulk-delete S3 objects

Object Metadata:
  head-object                           Head an S3 object
  get-object-tagging                    Get an S3 object's tagging
  put-object-tagging                    Put tagging on an S3 object
  delete-object-tagging                 Delete tagging from an S3 object

Bucket Operations:
  create-bucket                         Create an S3 bucket
  delete-bucket                         Delete an S3 bucket
  head-bucket                           Head an S3 bucket

Bucket Tagging:
  get-bucket-tagging                    Get a bucket's tagging
  put-bucket-tagging                    Put tagging on a bucket
  delete-bucket-tagging                 Delete tagging from a bucket

Bucket Policy:
  get-bucket-policy                     Get a bucket's policy
  put-bucket-policy                     Put a bucket policy
  delete-bucket-policy                  Delete a bucket's policy

Bucket Versioning:
  get-bucket-versioning                 Get a bucket's versioning configuration
  put-bucket-versioning                 Put a bucket versioning configuration

Bucket Lifecycle:
  get-bucket-lifecycle-configuration    Get a bucket's lifecycle configuration
  put-bucket-lifecycle-configuration    Put a bucket lifecycle configuration
  delete-bucket-lifecycle-configuration Delete a bucket's lifecycle configuration

Bucket Encryption:
  get-bucket-encryption                 Get a bucket's encryption configuration
  put-bucket-encryption                 Put a bucket encryption configuration
  delete-bucket-encryption              Delete a bucket's encryption configuration

Bucket CORS:
  get-bucket-cors                       Get a bucket's CORS configuration
  put-bucket-cors                       Put a bucket CORS configuration
  delete-bucket-cors                    Delete a bucket's CORS configuration

Bucket Public Access Block:
  get-public-access-block               Get a bucket's public access block configuration
  put-public-access-block               Put a bucket public access block configuration
  delete-public-access-block            Delete a bucket's public access block configuration

Bucket Website:
  get-bucket-website                    Get a bucket's website configuration
  put-bucket-website                    Put a bucket website configuration
  delete-bucket-website                 Delete a bucket's website configuration

Bucket Logging:
  get-bucket-logging                    Get a bucket's logging configuration
  put-bucket-logging                    Put a bucket logging configuration

Bucket Notification:
  get-bucket-notification-configuration Get a bucket's notification configuration
  put-bucket-notification-configuration Put a bucket notification configuration

Batch:
  batch-run                             Run s7cmd commands from a file (or - for stdin)

Other:
  help                                  Print this message or the help of the given subcommand(s)

Options:
{options}{after-help}",
)]
pub struct Cli {
    /// Generate shell completions for s7cmd (all subcommands) and exit.
    ///
    /// Equivalent to `<subcommand> --auto-complete-shell <SHELL>`,
    /// but does not require picking a subcommand.
    #[arg(long, value_enum, value_name = "SHELL", global = false)]
    pub auto_complete_shell: Option<Shell>,

    #[command(subcommand)]
    pub command: Option<Cmd>,
}

#[derive(clap::Args, Clone, Debug)]
pub struct BatchRunArgs {
    /// Path to a script file with s7cmd commands, or `-` to read from stdin.
    #[arg(value_name = "FILE")]
    pub script: String,

    /// Number of commands to run concurrently. 1 = sequential (default).
    /// 0 = use all logical CPUs.
    #[arg(long, default_value_t = 1, value_name = "N")]
    pub parallel: usize,

    /// Execute commands as they are read from stdin (no progress bar).
    /// By default, all commands are read first, then executed.
    #[arg(long)]
    pub streaming: bool,

    /// Continue executing remaining commands after any non-zero exit
    /// (failure or warning). By default, the first non-zero exit stops
    /// execution (sequential) or prevents new commands from starting
    /// (parallel). Mutually exclusive with `--max-errors` and
    /// `--continue-on-warning`.
    #[arg(long, conflicts_with_all = ["max_errors", "continue_on_warning"])]
    pub continue_on_error: bool,

    /// Continue executing remaining commands after a per-line warning
    /// (exit codes 3 and 4 — `EXIT_CODE_WARNING` and `EXIT_CODE_NOT_FOUND`).
    /// True failures (any other non-zero exit) still stop the run
    /// according to `--max-errors` (or the default first-failure stop).
    /// Mutually exclusive with `--continue-on-error`.
    #[arg(long, conflicts_with = "continue_on_error")]
    pub continue_on_warning: bool,

    /// Stop spawning new commands once `N` failures have been recorded
    /// (graceful: in-flight commands complete). Must be >= 1. When
    /// omitted, the run stops on the first failure — the same behavior
    /// as no flag at all. Mutually exclusive with `--continue-on-error`.
    /// When combined with `--continue-on-warning`, only true failures
    /// (non-warning non-zero exits) count toward `N`.
    ///
    /// In parallel mode this only stops NEW spawns; in-flight commands
    /// run to completion. When `--parallel` is close to or exceeds the
    /// total line count, every line may already be in flight by the
    /// time the threshold trips, so the threshold has no visible effect.
    /// Send SIGINT to cancel in-flight work.
    #[arg(
        long,
        value_name = "N",
        value_parser = clap::value_parser!(u64).range(1..),
    )]
    pub max_errors: Option<u64>,

    /// Suppress the end-of-run summary line on stderr.
    #[arg(long)]
    pub no_summary: bool,

    /// Suppress the live progress bar on stderr (the end-of-run summary
    /// is still printed unless `--no-summary` is also set). Useful when
    /// stderr is a TTY but you want machine-readable log output —
    /// terminal multiplexers, `script(1)`, some CI runners. Has no
    /// effect with `--streaming` or non-TTY stderr, which already
    /// suppress the bar.
    #[arg(long)]
    pub no_progress: bool,

    /// Only validate the script's format. Stops at the first
    /// problematic line, reports it at error level, and exits 1 — no
    /// command is executed. On success an info-level message is logged.
    /// Verbosity is forced to at least info while this flag is set so
    /// the success message is visible at the default warn level.
    #[arg(long)]
    pub check_format: bool,

    // Tracing flags — same names as every other subcommand's tracing
    // block. AWS auth/endpoint flags are intentionally NOT included
    // (each per-line subcommand brings its own).
    #[arg(long, default_value_t = false, help_heading = "Tracing/Logging")]
    pub json_tracing: bool,

    #[arg(long, default_value_t = false, help_heading = "Tracing/Logging")]
    pub aws_sdk_tracing: bool,

    #[arg(long, default_value_t = false, help_heading = "Tracing/Logging")]
    pub span_events_tracing: bool,

    #[arg(long, default_value_t = false, help_heading = "Tracing/Logging")]
    pub disable_color_tracing: bool,

    #[command(flatten)]
    pub verbosity: Verbosity<WarnLevel>,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Cmd {
    // Object Operations
    /// List S3 objects
    Ls(Box<s3ls_rs::CLIArgs>),
    /// Copy objects from/to S3 (or S3 to S3)
    Cp(s3util_rs::config::args::CpArgs),
    /// Move objects from/to S3 (copy then delete source)
    Mv(s3util_rs::config::args::MvArgs),
    /// Delete a single S3 object
    Rm(s3util_rs::config::args::RmArgs),
    /// Synchronize files between local and S3 (or S3 to S3)
    Sync(Box<s3sync::CLIArgs>),
    /// Bulk-delete S3 objects
    Clean(Box<s3rm_rs::CLIArgs>),

    // Object Metadata
    /// Head an S3 object
    HeadObject(s3util_rs::config::args::HeadObjectArgs),
    /// Get an S3 object's tagging
    GetObjectTagging(s3util_rs::config::args::GetObjectTaggingArgs),
    /// Put tagging on an S3 object
    PutObjectTagging(s3util_rs::config::args::PutObjectTaggingArgs),
    /// Delete tagging from an S3 object
    DeleteObjectTagging(s3util_rs::config::args::DeleteObjectTaggingArgs),

    // Bucket Operations
    /// Create an S3 bucket
    CreateBucket(s3util_rs::config::args::CreateBucketArgs),
    /// Delete an S3 bucket
    DeleteBucket(s3util_rs::config::args::DeleteBucketArgs),
    /// Head an S3 bucket
    HeadBucket(s3util_rs::config::args::HeadBucketArgs),

    // Bucket Tagging
    /// Get a bucket's tagging
    GetBucketTagging(s3util_rs::config::args::GetBucketTaggingArgs),
    /// Put tagging on a bucket
    PutBucketTagging(s3util_rs::config::args::PutBucketTaggingArgs),
    /// Delete tagging from a bucket
    DeleteBucketTagging(s3util_rs::config::args::DeleteBucketTaggingArgs),

    // Bucket Policy
    /// Get a bucket's policy
    GetBucketPolicy(s3util_rs::config::args::GetBucketPolicyArgs),
    /// Put a bucket policy
    PutBucketPolicy(s3util_rs::config::args::PutBucketPolicyArgs),
    /// Delete a bucket's policy
    DeleteBucketPolicy(s3util_rs::config::args::DeleteBucketPolicyArgs),

    // Bucket Versioning
    /// Get a bucket's versioning configuration
    GetBucketVersioning(s3util_rs::config::args::GetBucketVersioningArgs),
    /// Put a bucket versioning configuration
    PutBucketVersioning(s3util_rs::config::args::PutBucketVersioningArgs),

    // Bucket Lifecycle
    /// Get a bucket's lifecycle configuration
    GetBucketLifecycleConfiguration(s3util_rs::config::args::GetBucketLifecycleConfigurationArgs),
    /// Put a bucket lifecycle configuration
    PutBucketLifecycleConfiguration(s3util_rs::config::args::PutBucketLifecycleConfigurationArgs),
    /// Delete a bucket's lifecycle configuration
    DeleteBucketLifecycleConfiguration(
        s3util_rs::config::args::DeleteBucketLifecycleConfigurationArgs,
    ),

    // Bucket Encryption
    /// Get a bucket's encryption configuration
    GetBucketEncryption(s3util_rs::config::args::GetBucketEncryptionArgs),
    /// Put a bucket encryption configuration
    PutBucketEncryption(s3util_rs::config::args::PutBucketEncryptionArgs),
    /// Delete a bucket's encryption configuration
    DeleteBucketEncryption(s3util_rs::config::args::DeleteBucketEncryptionArgs),

    // Bucket CORS
    /// Get a bucket's CORS configuration
    GetBucketCors(s3util_rs::config::args::GetBucketCorsArgs),
    /// Put a bucket CORS configuration
    PutBucketCors(s3util_rs::config::args::PutBucketCorsArgs),
    /// Delete a bucket's CORS configuration
    DeleteBucketCors(s3util_rs::config::args::DeleteBucketCorsArgs),

    // Bucket Public Access Block
    /// Get a bucket's public access block configuration
    GetPublicAccessBlock(s3util_rs::config::args::GetPublicAccessBlockArgs),
    /// Put a bucket public access block configuration
    PutPublicAccessBlock(s3util_rs::config::args::PutPublicAccessBlockArgs),
    /// Delete a bucket's public access block configuration
    DeletePublicAccessBlock(s3util_rs::config::args::DeletePublicAccessBlockArgs),

    // Bucket Website
    /// Get a bucket's website configuration
    GetBucketWebsite(s3util_rs::config::args::GetBucketWebsiteArgs),
    /// Put a bucket website configuration
    PutBucketWebsite(s3util_rs::config::args::PutBucketWebsiteArgs),
    /// Delete a bucket's website configuration
    DeleteBucketWebsite(s3util_rs::config::args::DeleteBucketWebsiteArgs),

    // Bucket Logging
    /// Get a bucket's logging configuration
    GetBucketLogging(s3util_rs::config::args::GetBucketLoggingArgs),
    /// Put a bucket logging configuration
    PutBucketLogging(s3util_rs::config::args::PutBucketLoggingArgs),

    // Bucket Notification
    /// Get a bucket's notification configuration
    GetBucketNotificationConfiguration(
        s3util_rs::config::args::GetBucketNotificationConfigurationArgs,
    ),
    /// Put a bucket notification configuration
    PutBucketNotificationConfiguration(
        s3util_rs::config::args::PutBucketNotificationConfigurationArgs,
    ),

    // Batch
    /// Run s7cmd commands from a file (or - for stdin)
    BatchRun(BatchRunArgs),
}

/// Build the s7cmd `Command` with `--auto-complete-shell` hidden on every
/// subcommand. The flag is inherited from each upstream args struct (we
/// cannot remove it), but it's redundant with the top-level
/// `--auto-complete-shell` — both produce the same full-tree completion.
///
/// `hide(true)` alone only suppresses the flag from `--help`. clap_complete
/// does not honor `hide(true)` for args, so we additionally clear the long
/// name to keep it out of generated completion scripts. The arg id stays
/// so `FromArgMatches` still derives `auto_complete_shell: None` cleanly;
/// users who want completions use the top-level `--auto-complete-shell`.
#[allow(dead_code)] // used from main.rs; cli_routing integration test includes this file directly
pub fn cli_command() -> clap::Command {
    // Cap help-text wrap width so long flag descriptions stay readable on
    // wide terminals (otherwise clap wraps at the detected terminal width
    // and produces 200+ char lines). Requires the clap `wrap_help`
    // feature for terminal-size detection. Propagates to subcommands.
    let mut cmd = Cli::command().max_term_width(100);
    let names: Vec<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    for name in names {
        cmd = cmd.mut_subcommand(name, |sub| {
            let has_flag = sub
                .get_arguments()
                .any(|a| a.get_id().as_str() == "auto_complete_shell");
            if has_flag {
                sub.mut_arg("auto_complete_shell", |a| {
                    a.hide(true).long(None::<&'static str>)
                })
            } else {
                sub
            }
        });
    }
    cmd
}
