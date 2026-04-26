use clap::Parser;

// Re-import the binary crate's modules into the integration test.
// Integration tests can't access the binary's private modules normally,
// so we rebuild Cli here from the public lib types — equivalent shape.
//
// (If/when we expose Cli as part of a thin lib, replace this with
// `use s7cmd::cli::{Cli, Cmd};`.)

#[path = "../src/cli.rs"]
mod cli;

use cli::{Cli, Cmd};

#[test]
fn parses_sync_with_two_paths() {
    let cli = Cli::try_parse_from([
        "s7cmd", "sync",
        "--allow-both-local-storage",
        "/tmp/src", "/tmp/dst",
    ]).expect("sync should parse");
    match cli.command {
        Cmd::Sync(args) => {
            // s3sync::CLIArgs exposes source/target as String fields after parsing.
            // Just confirm the variant matched; deep assertions belong in s3sync.
            let _ = args;
        }
    }
}
