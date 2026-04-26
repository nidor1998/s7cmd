use clap::{Parser, Subcommand};

#[derive(Parser, Clone, Debug)]
#[command(
    name = "s7cmd",
    about = "Unified S3 CLI: s3sync + s3util",
    version,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Cmd,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Cmd {
    /// Synchronize files between local and S3 (or S3 to S3)
    Sync(Box<s3sync::CLIArgs>),

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
