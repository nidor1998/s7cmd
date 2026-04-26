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
}
