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
    assert!(matches!(cli.command, Cmd::Sync(_)));
}

#[test]
fn parses_cp_with_two_paths() {
    let cli = Cli::try_parse_from([
        "s7cmd", "cp", "/tmp/file", "s3://bucket/key",
    ]).expect("cp should parse");
    assert!(matches!(cli.command, Cmd::Cp(_)));
}

#[test]
fn parses_mv_with_two_paths() {
    let cli = Cli::try_parse_from([
        "s7cmd", "mv", "s3://b1/k1", "s3://b2/k2",
    ]).expect("mv should parse");
    assert!(matches!(cli.command, Cmd::Mv(_)));
}

#[test]
fn parses_rm_with_one_path() {
    let cli = Cli::try_parse_from([
        "s7cmd", "rm", "s3://bucket/key",
    ]).expect("rm should parse");
    assert!(matches!(cli.command, Cmd::Rm(_)));
}
