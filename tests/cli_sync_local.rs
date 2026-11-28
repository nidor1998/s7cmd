//! Process-level regression tests for the `sync` local-storage fixes that
//! arrived with s3sync 1.64.1 (nidor1998/s3sync#267).
//!
//! Every test syncs between two local directories
//! (`--allow-both-local-storage`), so no AWS access or network is needed.
//!
//! Covered here:
//! - `--delete` fails, and deletes nothing, when a local directory cannot be
//!   read (it used to treat the unreadable part as removed);
//! - a symbolic link loop stays a warning with `--delete`;
//! - parallel listing of local directories lists every file exactly once;
//! - a registered preprocess callback no longer makes objects synced to
//!   local storage count as skipped.

mod common;

use common::{run, s7cmd_cmd_clean_env};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Run `s7cmd sync --allow-both-local-storage <args> <source>/ <target>/`.
fn run_local_sync(args: &[&str], source: &Path, target: &Path) -> (Option<i32>, String, String) {
    let mut cmd = s7cmd_cmd_clean_env();
    cmd.args(["sync", "--allow-both-local-storage"])
        .args(args)
        .arg(format!("{}/", source.to_str().unwrap()))
        .arg(format!("{}/", target.to_str().unwrap()));
    run(&mut cmd)
}

/// Every regular file under `root`, as `/`-separated paths relative to it.
fn files_under(root: &Path) -> BTreeSet<String> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeSet<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            // symlink_metadata: never follow a link, so a link loop is not walked.
            let file_type = fs::symlink_metadata(&path).unwrap().file_type();
            if file_type.is_dir() {
                walk(root, &path, files);
            } else if file_type.is_file() {
                let relative = path.strip_prefix(root).unwrap();
                files.insert(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    let mut files = BTreeSet::new();
    walk(root, root, &mut files);
    files
}

fn set_of(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|path| path.to_string()).collect()
}

fn write_file(root: &Path, relative: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, relative.as_bytes()).unwrap();
}

/// Value of `name=` in the `sync summary` log line (printed with `-v`).
fn summary_value(stdout: &str, name: &str) -> u64 {
    let summary = stdout
        .lines()
        .find(|line| line.contains("sync summary"))
        .unwrap_or_else(|| panic!("no sync summary line; stdout={stdout}"));
    let field = format!(" {name}=");
    let start = summary
        .find(&field)
        .unwrap_or_else(|| panic!("no {name} in summary: {summary}"))
        + field.len();
    summary[start..]
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap_or_else(|_| panic!("{name} is not a number in summary: {summary}"))
}

// ---------------------------------------------------------------
// --delete with a local directory that cannot be read
// ---------------------------------------------------------------

#[cfg(target_family = "unix")]
mod unreadable_directory {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// Makes a directory unreadable until dropped, so the temporary
    /// directory can always be cleaned up, even when an assertion fails.
    struct Unreadable<'a>(&'a Path);

    impl<'a> Unreadable<'a> {
        fn new(dir: &'a Path) -> Self {
            fs::set_permissions(dir, fs::Permissions::from_mode(0o000)).unwrap();
            // Built before the check, so the permissions are restored even
            // when the check fails.
            let unreadable = Self(dir);
            assert!(
                fs::read_dir(dir).is_err(),
                "tests must not run as root: root can read every directory"
            );
            unreadable
        }
    }

    impl Drop for Unreadable<'_> {
        fn drop(&mut self) {
            let _ = fs::set_permissions(self.0, fs::Permissions::from_mode(0o755));
        }
    }

    /// Source and target that are in sync, except for one stale file that
    /// exists only in the target.
    fn synced_dirs_with_stale_target_file() -> (tempfile::TempDir, tempfile::TempDir) {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        for root in [source.path(), target.path()] {
            write_file(root, "readable/a.dat");
            write_file(root, "denied/b.dat");
        }
        write_file(target.path(), "stale.dat");
        (source, target)
    }

    const ALL_TARGET_FILES: [&str; 3] = ["denied/b.dat", "readable/a.dat", "stale.dat"];

    /// The source directory `denied/` cannot be read. Its files still exist
    /// in the source, so the matching target files must not be deleted: the
    /// sync fails and nothing at all is deleted. Before s3sync 1.64.1 the
    /// directory was skipped with a warning and `denied/b.dat` was deleted
    /// from the target.
    #[test]
    fn delete_fails_and_deletes_nothing_when_source_directory_is_unreadable() {
        for max_parallel_listings in ["1", "16"] {
            let (source, target) = synced_dirs_with_stale_target_file();
            let denied = source.path().join("denied");
            let unreadable = Unreadable::new(&denied);

            let (code, stdout, stderr) = run_local_sync(
                &["--delete", "--max-parallel-listings", max_parallel_listings],
                source.path(),
                target.path(),
            );
            drop(unreadable);

            assert_eq!(
                code,
                Some(1),
                "--max-parallel-listings {max_parallel_listings}: an unreadable source directory \
                 must fail a --delete sync; stdout={stdout}\nstderr={stderr}"
            );
            assert_eq!(
                files_under(target.path()),
                set_of(&ALL_TARGET_FILES),
                "--max-parallel-listings {max_parallel_listings}: nothing may be deleted"
            );
        }
    }

    /// The same applies to a local target that cannot be listed: the stale
    /// files in the part that could be listed must not be deleted either.
    /// The source has nothing under `denied/`, so nothing is written there
    /// and the listing failure is the only thing that can fail the sync.
    /// Before s3sync 1.64.1 the directory was skipped with a warning and
    /// `stale.dat` was deleted.
    #[test]
    fn delete_fails_and_deletes_nothing_when_target_directory_is_unreadable() {
        for max_parallel_listings in ["1", "16"] {
            let source = tempfile::tempdir().unwrap();
            let target = tempfile::tempdir().unwrap();
            write_file(source.path(), "readable/a.dat");
            for relative in ALL_TARGET_FILES {
                write_file(target.path(), relative);
            }
            let denied = target.path().join("denied");
            let unreadable = Unreadable::new(&denied);

            let (code, stdout, stderr) = run_local_sync(
                &["--delete", "--max-parallel-listings", max_parallel_listings],
                source.path(),
                target.path(),
            );
            drop(unreadable);

            assert_eq!(
                code,
                Some(1),
                "--max-parallel-listings {max_parallel_listings}: an unreadable target directory \
                 must fail a --delete sync; stdout={stdout}\nstderr={stderr}"
            );
            assert_eq!(
                files_under(target.path()),
                set_of(&ALL_TARGET_FILES),
                "--max-parallel-listings {max_parallel_listings}: nothing may be deleted"
            );
        }
    }

    /// Without `--delete` nothing can be deleted by mistake, so an
    /// unreadable source directory stays a warning and the rest is synced.
    #[test]
    fn unreadable_source_directory_is_a_warning_without_delete() {
        for max_parallel_listings in ["1", "16"] {
            let source = tempfile::tempdir().unwrap();
            let target = tempfile::tempdir().unwrap();
            write_file(source.path(), "readable/a.dat");
            write_file(source.path(), "denied/b.dat");
            let denied = source.path().join("denied");
            let unreadable = Unreadable::new(&denied);

            let (code, stdout, stderr) = run_local_sync(
                &["--max-parallel-listings", max_parallel_listings],
                source.path(),
                target.path(),
            );
            drop(unreadable);

            assert_eq!(
                code,
                Some(3),
                "--max-parallel-listings {max_parallel_listings}: stdout={stdout}\nstderr={stderr}"
            );
            assert_eq!(files_under(target.path()), set_of(&["readable/a.dat"]));
        }
    }

    /// `--warn-as-error` turns that warning into a failure.
    #[test]
    fn unreadable_source_directory_fails_with_warn_as_error() {
        for max_parallel_listings in ["1", "16"] {
            let source = tempfile::tempdir().unwrap();
            let target = tempfile::tempdir().unwrap();
            write_file(source.path(), "readable/a.dat");
            write_file(source.path(), "denied/b.dat");
            let denied = source.path().join("denied");
            let unreadable = Unreadable::new(&denied);

            let (code, stdout, stderr) = run_local_sync(
                &[
                    "--warn-as-error",
                    "--max-parallel-listings",
                    max_parallel_listings,
                ],
                source.path(),
                target.path(),
            );
            drop(unreadable);

            assert_eq!(
                code,
                Some(1),
                "--max-parallel-listings {max_parallel_listings}: stdout={stdout}\nstderr={stderr}"
            );
        }
    }
}

// ---------------------------------------------------------------
// Symbolic link loop
// ---------------------------------------------------------------

#[cfg(target_family = "unix")]
mod symlink_loop {
    use super::*;

    /// `dir/self -> ..` is a loop once links are followed (the default).
    fn source_with_symlink_loop() -> tempfile::TempDir {
        let source = tempfile::tempdir().unwrap();
        write_file(source.path(), "dir/file.dat");
        std::os::unix::fs::symlink("..", source.path().join("dir/self")).unwrap();
        source
    }

    /// A symbolic link loop hides no files (they are listed through their
    /// real path), so with `--delete` it is still only a warning: the sync
    /// completes, and the deletion of stale target files is still carried
    /// out.
    #[test]
    fn symlink_loop_is_a_warning_with_delete() {
        for max_parallel_listings in ["1", "16"] {
            let source = source_with_symlink_loop();
            let target = tempfile::tempdir().unwrap();
            write_file(target.path(), "stale.dat");

            let (code, stdout, stderr) = run_local_sync(
                &["--delete", "--max-parallel-listings", max_parallel_listings],
                source.path(),
                target.path(),
            );

            assert_eq!(
                code,
                Some(3),
                "--max-parallel-listings {max_parallel_listings}: a symlink loop must be a \
                 warning; stdout={stdout}\nstderr={stderr}"
            );
            assert!(stdout.contains("File system loop found"), "stdout={stdout}");
            assert_eq!(
                files_under(target.path()),
                set_of(&["dir/file.dat"]),
                "--max-parallel-listings {max_parallel_listings}: the file must be synced and \
                 the stale file deleted"
            );
        }
    }

    /// `--warn-as-error` still makes the loop fatal.
    #[test]
    fn symlink_loop_fails_with_warn_as_error() {
        for max_parallel_listings in ["1", "16"] {
            let source = source_with_symlink_loop();
            let target = tempfile::tempdir().unwrap();

            let (code, stdout, stderr) = run_local_sync(
                &[
                    "--warn-as-error",
                    "--max-parallel-listings",
                    max_parallel_listings,
                ],
                source.path(),
                target.path(),
            );

            assert_eq!(
                code,
                Some(1),
                "--max-parallel-listings {max_parallel_listings}: stdout={stdout}\nstderr={stderr}"
            );
        }
    }
}

// ---------------------------------------------------------------
// Parallel listing of local directories
// ---------------------------------------------------------------

/// A tree that is wider than one directory and deeper than the default
/// `--max-parallel-listing-max-depth`, with files at every level.
fn build_deep_tree(root: &Path) -> BTreeSet<String> {
    let mut expected = BTreeSet::new();
    let mut add = |relative: String| {
        write_file(root, &relative);
        expected.insert(relative);
    };

    add("root.dat".to_string());
    for dir_index in 0..5 {
        let mut dir = format!("dir{dir_index}");
        for depth in 0..4 {
            add(format!("{dir}/a.dat"));
            add(format!("{dir}/b.dat"));
            dir = format!("{dir}/depth{depth}");
        }
    }
    expected
}

/// Parallel listing was reworked so that subdirectories are listed
/// concurrently. Whatever the parallelism, every file must be listed, and
/// synced, exactly once.
#[test]
fn parallel_listing_syncs_every_file_exactly_once() {
    for max_parallel_listings in ["1", "2", "16"] {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let expected = build_deep_tree(source.path());
        assert_eq!(expected.len(), 1 + 5 * 4 * 2);

        let (code, stdout, stderr) = run_local_sync(
            &["-v", "--max-parallel-listings", max_parallel_listings],
            source.path(),
            target.path(),
        );

        assert_eq!(
            code,
            Some(0),
            "--max-parallel-listings {max_parallel_listings}: stdout={stdout}\nstderr={stderr}"
        );
        assert_eq!(
            files_under(target.path()),
            expected,
            "--max-parallel-listings {max_parallel_listings}"
        );
        // A file listed twice would be transferred twice.
        assert_eq!(
            summary_value(&stdout, "transferred_object"),
            expected.len() as u64,
            "--max-parallel-listings {max_parallel_listings}: stdout={stdout}"
        );
    }
}

/// The same holds for the listing of a local target: with `--delete`,
/// exactly the stale files are deleted, at every depth.
#[test]
fn parallel_listing_deletes_exactly_the_stale_target_files() {
    for max_parallel_listings in ["1", "16"] {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let expected = build_deep_tree(source.path());
        build_deep_tree(target.path());
        let stale = [
            "stale.dat",
            "dir0/stale.dat",
            "dir4/depth0/depth1/depth2/stale.dat",
            "only-in-target/stale.dat",
        ];
        for relative in stale {
            write_file(target.path(), relative);
        }

        let (code, stdout, stderr) = run_local_sync(
            &[
                "-v",
                "--delete",
                "--max-parallel-listings",
                max_parallel_listings,
            ],
            source.path(),
            target.path(),
        );

        assert_eq!(
            code,
            Some(0),
            "--max-parallel-listings {max_parallel_listings}: stdout={stdout}\nstderr={stderr}"
        );
        assert_eq!(
            files_under(target.path()),
            expected,
            "--max-parallel-listings {max_parallel_listings}"
        );
        assert_eq!(
            summary_value(&stdout, "deleted"),
            stale.len() as u64,
            "--max-parallel-listings {max_parallel_listings}: stdout={stdout}"
        );
    }
}

// ---------------------------------------------------------------
// Preprocess callback with a local target
// ---------------------------------------------------------------

/// A preprocess callback is never called for a local target. Because local
/// storage returns no ETag, a registered callback used to make every synced
/// object look cancelled: it was counted as skipped and a `SYNC_CANCEL`
/// event was triggered, although the file had been written.
#[test]
fn preprocess_callback_does_not_count_local_target_objects_as_skipped() {
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    write_file(source.path(), "a.dat");
    write_file(source.path(), "dir/b.dat");

    let scripts = tempfile::tempdir().unwrap();
    let preprocess_script = scripts.path().join("preprocess.lua");
    fs::write(
        &preprocess_script,
        "function preprocess_callback(source_object, upload_metadata)\n    return true, upload_metadata\nend\n",
    )
    .unwrap();
    // SYNC_COMPLETE is 1 << 4 and SYNC_CANCEL is 1 << 12.
    let event_script = scripts.path().join("event.lua");
    fs::write(
        &event_script,
        r#"function on_event(event_data)
    if event_data.event_type == 1 << 4 then
        print("EVENT SYNC_COMPLETE " .. event_data.key)
    elseif event_data.event_type == 1 << 12 then
        print("EVENT SYNC_CANCEL " .. event_data.key)
    end
end
"#,
    )
    .unwrap();

    let (code, stdout, stderr) = run_local_sync(
        &[
            "-v",
            "--preprocess-callback-lua-script",
            preprocess_script.to_str().unwrap(),
            "--event-callback-lua-script",
            event_script.to_str().unwrap(),
        ],
        source.path(),
        target.path(),
    );

    assert_eq!(code, Some(0), "stdout={stdout}\nstderr={stderr}");
    assert_eq!(files_under(target.path()), set_of(&["a.dat", "dir/b.dat"]));
    assert_eq!(
        summary_value(&stdout, "transferred_object"),
        2,
        "stdout={stdout}"
    );
    assert_eq!(summary_value(&stdout, "skipped"), 0, "stdout={stdout}");
    assert!(
        stdout.contains("EVENT SYNC_COMPLETE a.dat")
            && stdout.contains("EVENT SYNC_COMPLETE dir/b.dat"),
        "stdout={stdout}"
    );
    assert!(
        !stdout.contains("EVENT SYNC_CANCEL"),
        "no object was cancelled; stdout={stdout}"
    );
}

// ---------------------------------------------------------------
// Size suffixes are case-sensitive
// ---------------------------------------------------------------

/// A suffix ending in a lowercase `b` means bits: `8Kb` is 1,000 bytes, one
/// eighth of `8KB`. This is what the help text now documents; the behavior
/// itself is unchanged.
#[test]
fn lowercase_b_size_suffix_is_parsed_as_bits() {
    let source = tempfile::tempdir().unwrap();
    // Between 8Kb (1,000 bytes) and 8KB (8,000 bytes).
    fs::write(source.path().join("medium.dat"), vec![b'x'; 1500]).unwrap();
    fs::write(source.path().join("small.dat"), vec![b'x'; 10]).unwrap();

    let cases = [
        ("--filter-larger-size", "8Kb", set_of(&["medium.dat"])),
        ("--filter-larger-size", "8KB", set_of(&[])),
        ("--filter-smaller-size", "8Kb", set_of(&["small.dat"])),
        (
            "--filter-smaller-size",
            "8KB",
            set_of(&["medium.dat", "small.dat"]),
        ),
    ];
    for (flag, size, expected) in cases {
        let target = tempfile::tempdir().unwrap();
        let (code, stdout, stderr) = run_local_sync(&[flag, size], source.path(), target.path());
        assert_eq!(
            code,
            Some(0),
            "{flag} {size}: stdout={stdout}\nstderr={stderr}"
        );
        assert_eq!(files_under(target.path()), expected, "{flag} {size}");
    }
}

/// The converted byte count must still meet the minimum of the option, so
/// `8mb` (1,000,000 bytes) is below the 5MiB minimum of the multipart
/// options and the 1MiB minimum of `--rate-limit-bandwidth`.
#[test]
fn lowercase_b_size_suffix_below_option_minimum_exits_2() {
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();

    for flag in [
        "--multipart-threshold",
        "--multipart-chunksize",
        "--rate-limit-bandwidth",
    ] {
        let (code, _stdout, stderr) = run_local_sync(&[flag, "8mb"], source.path(), target.path());
        assert_eq!(
            code,
            Some(2),
            "{flag} 8mb must be rejected; stderr={stderr}"
        );
        assert!(stderr.contains("invalid value '8mb'"), "stderr={stderr}");

        // The same number of megabytes is accepted.
        let (code, stdout, stderr) = run_local_sync(&[flag, "8MB"], source.path(), target.path());
        assert_eq!(
            code,
            Some(0),
            "{flag} 8MB must be accepted; stdout={stdout}\nstderr={stderr}"
        );
    }
}

/// The examples given in the help text are accepted by their options.
#[test]
fn size_suffix_examples_from_help_are_accepted() {
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    write_file(source.path(), "a.dat");

    let (code, stdout, stderr) = run_local_sync(
        &[
            "--multipart-threshold",
            "64Mib",
            "--multipart-chunksize",
            "64Mib",
            "--rate-limit-bandwidth",
            "64Mib",
            "--filter-smaller-size",
            "8Mb",
            "--lua-vm-memory-limit",
            "8Mb",
        ],
        source.path(),
        target.path(),
    );
    assert_eq!(code, Some(0), "stdout={stdout}\nstderr={stderr}");
    assert_eq!(files_under(target.path()), set_of(&["a.dat"]));
}
