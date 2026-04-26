use clap::{Parser, Subcommand};
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
    arg_required_else_help = true
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
    /// Synchronize files between local and S3 (or S3 to S3)
    Sync(Box<s3sync::CLIArgs>),

    /// List S3 objects
    Ls(Box<s3ls_rs::CLIArgs>),
    /// Bulk-delete S3 objects
    Clean(Box<s3rm_rs::CLIArgs>),

    /// Copy objects from/to S3
    Cp(s3util_rs::config::args::CpArgs),
    /// Move objects from/to S3 (copy then delete source)
    Mv(s3util_rs::config::args::MvArgs),
    /// Delete a single S3 object
    Rm(s3util_rs::config::args::RmArgs),

    /// Create an S3 bucket
    CreateBucket(s3util_rs::config::args::CreateBucketArgs),
    /// Delete an S3 bucket
    DeleteBucket(s3util_rs::config::args::DeleteBucketArgs),
    /// Head an S3 bucket
    HeadBucket(s3util_rs::config::args::HeadBucketArgs),
    /// Head an S3 object
    HeadObject(s3util_rs::config::args::HeadObjectArgs),

    /// Get an S3 object's tagging
    GetObjectTagging(s3util_rs::config::args::GetObjectTaggingArgs),
    /// Put tagging on an S3 object
    PutObjectTagging(s3util_rs::config::args::PutObjectTaggingArgs),
    /// Delete tagging from an S3 object
    DeleteObjectTagging(s3util_rs::config::args::DeleteObjectTaggingArgs),

    /// Get a bucket's tagging
    GetBucketTagging(s3util_rs::config::args::GetBucketTaggingArgs),
    /// Put tagging on a bucket
    PutBucketTagging(s3util_rs::config::args::PutBucketTaggingArgs),
    /// Delete tagging from a bucket
    DeleteBucketTagging(s3util_rs::config::args::DeleteBucketTaggingArgs),

    /// Get a bucket's policy
    GetBucketPolicy(s3util_rs::config::args::GetBucketPolicyArgs),
    /// Put a bucket policy
    PutBucketPolicy(s3util_rs::config::args::PutBucketPolicyArgs),
    /// Delete a bucket's policy
    DeleteBucketPolicy(s3util_rs::config::args::DeleteBucketPolicyArgs),

    /// Get a bucket's versioning configuration
    GetBucketVersioning(s3util_rs::config::args::GetBucketVersioningArgs),
    /// Put a bucket versioning configuration
    PutBucketVersioning(s3util_rs::config::args::PutBucketVersioningArgs),
}
