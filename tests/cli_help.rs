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

#[test]
fn create_bucket_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["create-bucket", "--help"])
        .assert().success();
}

#[test]
fn delete_bucket_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["delete-bucket", "--help"])
        .assert().success();
}

#[test]
fn head_bucket_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["head-bucket", "--help"])
        .assert().success();
}

#[test]
fn head_object_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["head-object", "--help"])
        .assert().success();
}

#[test]
fn get_object_tagging_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["get-object-tagging", "--help"])
        .assert().success();
}

#[test]
fn put_object_tagging_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["put-object-tagging", "--help"])
        .assert().success();
}

#[test]
fn delete_object_tagging_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["delete-object-tagging", "--help"])
        .assert().success();
}
#[test]
fn get_bucket_tagging_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["get-bucket-tagging", "--help"]).assert().success();
}
#[test]
fn put_bucket_tagging_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["put-bucket-tagging", "--help"]).assert().success();
}
#[test]
fn delete_bucket_tagging_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["delete-bucket-tagging", "--help"]).assert().success();
}
#[test]
fn get_bucket_policy_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["get-bucket-policy", "--help"]).assert().success();
}
#[test]
fn put_bucket_policy_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["put-bucket-policy", "--help"]).assert().success();
}
#[test]
fn delete_bucket_policy_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["delete-bucket-policy", "--help"]).assert().success();
}
#[test]
fn get_bucket_versioning_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["get-bucket-versioning", "--help"]).assert().success();
}
#[test]
fn put_bucket_versioning_help_works() {
    Command::cargo_bin("s7cmd").unwrap()
        .args(["put-bucket-versioning", "--help"]).assert().success();
}
