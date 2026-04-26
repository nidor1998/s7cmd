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

#[test]
fn cp_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["cp", "--help"])
        .assert().success()
        .stdout(predicate::str::contains("AWS Configuration"));
}

#[test]
fn mv_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["mv", "--help"])
        .assert().success();
}

#[test]
fn rm_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["rm", "--help"])
        .assert().success();
}

#[test]
fn top_level_help_lists_cp_mv_rm() {
    Command::cargo_bin("s7cmd").unwrap()
        .arg("--help")
        .assert().success()
        .stdout(predicate::str::contains("cp"))
        .stdout(predicate::str::contains("mv"))
        .stdout(predicate::str::contains("rm"));
}
