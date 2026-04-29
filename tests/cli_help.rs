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
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["cp", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("AWS Configuration"));
}

#[test]
fn mv_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["mv", "--help"])
        .assert()
        .success();
}

#[test]
fn rm_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["rm", "--help"])
        .assert()
        .success();
}

#[test]
fn top_level_help_lists_cp_mv_rm() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("cp"))
        .stdout(predicate::str::contains("mv"))
        .stdout(predicate::str::contains("rm"));
}

#[test]
fn create_bucket_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["create-bucket", "--help"])
        .assert()
        .success();
}

#[test]
fn delete_bucket_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["delete-bucket", "--help"])
        .assert()
        .success();
}

#[test]
fn head_bucket_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["head-bucket", "--help"])
        .assert()
        .success();
}

#[test]
fn head_object_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["head-object", "--help"])
        .assert()
        .success();
}

#[test]
fn get_object_tagging_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["get-object-tagging", "--help"])
        .assert()
        .success();
}

#[test]
fn put_object_tagging_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["put-object-tagging", "--help"])
        .assert()
        .success();
}

#[test]
fn delete_object_tagging_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["delete-object-tagging", "--help"])
        .assert()
        .success();
}
#[test]
fn get_bucket_tagging_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["get-bucket-tagging", "--help"])
        .assert()
        .success();
}
#[test]
fn put_bucket_tagging_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["put-bucket-tagging", "--help"])
        .assert()
        .success();
}
#[test]
fn delete_bucket_tagging_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["delete-bucket-tagging", "--help"])
        .assert()
        .success();
}
#[test]
fn get_bucket_policy_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["get-bucket-policy", "--help"])
        .assert()
        .success();
}
#[test]
fn put_bucket_policy_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["put-bucket-policy", "--help"])
        .assert()
        .success();
}
#[test]
fn delete_bucket_policy_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["delete-bucket-policy", "--help"])
        .assert()
        .success();
}
#[test]
fn get_bucket_versioning_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["get-bucket-versioning", "--help"])
        .assert()
        .success();
}
#[test]
fn put_bucket_versioning_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["put-bucket-versioning", "--help"])
        .assert()
        .success();
}

#[test]
fn get_bucket_lifecycle_configuration_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["get-bucket-lifecycle-configuration", "--help"])
        .assert()
        .success();
}
#[test]
fn put_bucket_lifecycle_configuration_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["put-bucket-lifecycle-configuration", "--help"])
        .assert()
        .success();
}
#[test]
fn delete_bucket_lifecycle_configuration_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["delete-bucket-lifecycle-configuration", "--help"])
        .assert()
        .success();
}
#[test]
fn get_bucket_encryption_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["get-bucket-encryption", "--help"])
        .assert()
        .success();
}
#[test]
fn put_bucket_encryption_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["put-bucket-encryption", "--help"])
        .assert()
        .success();
}
#[test]
fn delete_bucket_encryption_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["delete-bucket-encryption", "--help"])
        .assert()
        .success();
}
#[test]
fn get_bucket_cors_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["get-bucket-cors", "--help"])
        .assert()
        .success();
}
#[test]
fn put_bucket_cors_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["put-bucket-cors", "--help"])
        .assert()
        .success();
}
#[test]
fn delete_bucket_cors_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["delete-bucket-cors", "--help"])
        .assert()
        .success();
}
#[test]
fn get_public_access_block_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["get-public-access-block", "--help"])
        .assert()
        .success();
}
#[test]
fn put_public_access_block_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["put-public-access-block", "--help"])
        .assert()
        .success();
}
#[test]
fn delete_public_access_block_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["delete-public-access-block", "--help"])
        .assert()
        .success();
}
#[test]
fn get_bucket_website_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["get-bucket-website", "--help"])
        .assert()
        .success();
}
#[test]
fn put_bucket_website_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["put-bucket-website", "--help"])
        .assert()
        .success();
}
#[test]
fn delete_bucket_website_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["delete-bucket-website", "--help"])
        .assert()
        .success();
}
#[test]
fn get_bucket_logging_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["get-bucket-logging", "--help"])
        .assert()
        .success();
}
#[test]
fn put_bucket_logging_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["put-bucket-logging", "--help"])
        .assert()
        .success();
}
#[test]
fn get_bucket_notification_configuration_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["get-bucket-notification-configuration", "--help"])
        .assert()
        .success();
}
#[test]
fn put_bucket_notification_configuration_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["put-bucket-notification-configuration", "--help"])
        .assert()
        .success();
}

#[test]
fn top_level_help_lists_new_bucket_subcommands() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "get-bucket-lifecycle-configuration",
        ))
        .stdout(predicate::str::contains("get-bucket-encryption"))
        .stdout(predicate::str::contains("get-bucket-cors"))
        .stdout(predicate::str::contains("get-public-access-block"))
        .stdout(predicate::str::contains("get-bucket-website"))
        .stdout(predicate::str::contains("get-bucket-logging"))
        .stdout(predicate::str::contains(
            "get-bucket-notification-configuration",
        ));
}

#[test]
fn top_level_help_lists_auto_complete_shell() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--auto-complete-shell"));
}

#[test]
fn top_level_auto_complete_shell_runs() {
    // Smoke: top-level --auto-complete-shell bash should exit 0 with
    // non-empty stdout (the shell completion script).
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["--auto-complete-shell", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete")); // bash completion scripts contain `complete -F`
}

#[test]
fn ls_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["ls", "--help"])
        .assert()
        .success();
}

#[test]
fn clean_help_works() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .args(["clean", "--help"])
        .assert()
        .success();
}

#[test]
fn top_level_help_lists_ls_and_clean() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("ls"))
        .stdout(predicate::str::contains("clean"));
}

#[test]
fn version_short_flag_prints_pkg_version() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .arg("-V")
        .assert()
        .success()
        // The version output should at least contain the crate name and the
        // semver from Cargo.toml. Whether it includes commit/target/rustc
        // depends on whether the `version` feature was compiled in.
        .stdout(predicate::str::contains("s7cmd 0.1.2"));
}

#[test]
fn version_long_flag_prints_pkg_version() {
    Command::cargo_bin("s7cmd")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("s7cmd 0.1.2"));
}
