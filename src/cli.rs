use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::shells::Shell;

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
  ls                     List S3 objects
  cp                     Copy objects from/to S3
  mv                     Move objects from/to S3 (copy then delete source)
  rm                     Delete a single S3 object
  sync                   Synchronize files between local and S3 (or S3 to S3)
  clean                  Bulk-delete S3 objects

Object Metadata:
  head-object            Head an S3 object
  get-object-tagging     Get an S3 object's tagging
  put-object-tagging     Put tagging on an S3 object
  delete-object-tagging  Delete tagging from an S3 object

Bucket Operations:
  create-bucket          Create an S3 bucket
  delete-bucket          Delete an S3 bucket
  head-bucket            Head an S3 bucket

Bucket Tagging:
  get-bucket-tagging     Get a bucket's tagging
  put-bucket-tagging     Put tagging on a bucket
  delete-bucket-tagging  Delete tagging from a bucket

Bucket Policy:
  get-bucket-policy      Get a bucket's policy
  put-bucket-policy      Put a bucket policy
  delete-bucket-policy   Delete a bucket's policy

Bucket Versioning:
  get-bucket-versioning  Get a bucket's versioning configuration
  put-bucket-versioning  Put a bucket versioning configuration

Other:
  help                   Print this message or the help of the given subcommand(s)

Options:
{options}{after-help}",
)]
pub struct Cli {
    /// Generate shell completions for s7cmd (all subcommands) and exit.
    /// Equivalent to passing `--auto-complete-shell <SHELL>` to any subcommand,
    /// but works without picking one.
    #[arg(long, value_enum, value_name = "SHELL", global = false)]
    pub auto_complete_shell: Option<Shell>,

    #[command(subcommand)]
    pub command: Option<Cmd>,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Cmd {
    // Object Operations
    /// List S3 objects
    Ls(Box<s3ls_rs::CLIArgs>),
    /// Copy objects from/to S3
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
#[allow(dead_code)] // used from main.rs; cli_dispatch integration test includes this file directly
pub fn cli_command() -> clap::Command {
    let mut cmd = Cli::command();
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
