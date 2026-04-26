use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn top_level_help_lists_sync() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("sync"));
}

#[test]
fn sync_help_includes_source_options() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["sync", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Source Options"));
}

#[test]
fn sync_help_includes_lua_passthrough() {
    // Smoke: confirms s3sync's lua_support default feature reaches the user.
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["sync", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--filter-callback-lua-script"));
}
