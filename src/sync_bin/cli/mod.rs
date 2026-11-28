// Vendored from s3sync@1.58.6
//   src/bin/s3sync/cli/mod.rs
// Adjustments: stripped #[cfg(test)] mod tests;
//              run() now returns Result<i32> instead of calling
//              std::process::exit (so it can be invoked from
//              batch-run without killing the process mid-batch);
//              show_sync_report_summary() carries the annotation_matches /
//              annotation_mismatch fields added in s3sync 1.59.0.
//              SIGINT exit code 130 added: after the pipeline stops, a
//              run interrupted by Ctrl+C returns 130 instead of falling
//              through to the success/error mapping — landed in s7cmd
//              first and adopted by upstream s3sync in 1.62.0 (which
//              calls std::process::exit; s7cmd returns the code so
//              batch-run survives), same pattern as s3rm-rs@1.6.0 /
//              s3ls-rs@1.3.0.

use anyhow::{Result, anyhow};
use s3sync::Config;
use s3sync::pipeline::Pipeline;
use s3sync::types::token::create_pipeline_cancellation_token;
use s3sync::types::{SYNC_REPORT_SUMMERY_NAME, SyncStatsReport};
use std::sync::MutexGuard;
use tokio::time::Instant;
use tracing::{debug, error, info};

mod ctrl_c_handler;
mod indicator;
mod ui_config;

const EXIT_CODE_SUCCESS: i32 = 0;
#[allow(dead_code)]
const EXIT_CODE_ERROR: i32 = 1;
#[allow(dead_code)]
const EXIT_CODE_INVALID_ARGS: i32 = 2;
const EXIT_CODE_WARNING: i32 = 3;
/// Conventional exit code for termination by Ctrl+C: 128 + SIGINT(2), the
/// shell encoding that lets scripts distinguish user interruption from
/// real failures.
const SIGINT_EXIT_CODE: i32 = 130;

pub async fn run(config: Config) -> Result<i32> {
    #[allow(unused_assignments)]
    let mut has_warning = false;

    {
        let cancellation_token = create_pipeline_cancellation_token();

        ctrl_c_handler::spawn_ctrl_c_handler(cancellation_token.clone());

        let start_time = Instant::now();
        debug!("sync pipeline start.");

        // When reporting sync status, a sync summary log is not needed.
        let log_sync_summary = !config.report_sync_status;

        let stderr_tracing = config
            .tracing_config
            .as_ref()
            .is_some_and(|tracing_config| tracing_config.stderr_tracing);

        let mut pipeline = Pipeline::new(config.clone(), cancellation_token).await;
        let indicator_join_handle = indicator::show_indicator(
            pipeline.get_stats_receiver(),
            ui_config::is_progress_indicator_needed(&config),
            ui_config::is_show_result_needed(&config),
            log_sync_summary,
            config.dry_run,
            stderr_tracing,
        );

        pipeline.run().await;
        indicator_join_handle.await?;

        let duration_sec = format!("{:.3}", start_time.elapsed().as_secs_f32());

        // Ctrl+C takes precedence over whatever the pipeline recorded: once
        // SIGINT is received the run is "interrupted", even if the forced
        // shutdown also surfaced errors or warnings. Returned (not
        // process::exit) so batch-run survives and buckets the line as
        // skipped.
        if ctrl_c_handler::is_ctrl_c_received() {
            debug!(duration_sec = duration_sec, "sync cancelled by user.");
            return Ok(SIGINT_EXIT_CODE);
        }

        if pipeline.has_error() {
            error!(duration_sec = duration_sec, "s7cmd sync failed.");

            return Err(anyhow!("s7cmd sync failed."));
        }

        has_warning = pipeline.has_warning();

        if config.report_sync_status {
            show_sync_report_summary(pipeline.get_sync_stats_report().lock().unwrap());
        }

        debug!(
            duration_sec = duration_sec,
            "s7cmd sync has been completed."
        );
    }

    if has_warning {
        return Ok(EXIT_CODE_WARNING);
    }

    Ok(EXIT_CODE_SUCCESS)
}

fn show_sync_report_summary(sync_stats_report: MutexGuard<'_, SyncStatsReport>) {
    info!(
        name = SYNC_REPORT_SUMMERY_NAME,
        number_of_objects = sync_stats_report.number_of_objects,
        etag_matches = sync_stats_report.etag_matches,
        checksum_matches = sync_stats_report.checksum_matches,
        metadata_matches = sync_stats_report.metadata_matches,
        tagging_matches = sync_stats_report.tagging_matches,
        not_found = sync_stats_report.not_found,
        etag_mismatch = sync_stats_report.etag_mismatch,
        checksum_mismatch = sync_stats_report.checksum_mismatch,
        metadata_mismatch = sync_stats_report.metadata_mismatch,
        tagging_mismatch = sync_stats_report.tagging_mismatch,
        etag_unknown = sync_stats_report.etag_unknown,
        checksum_unknown = sync_stats_report.checksum_unknown,
        // Added in s3sync 1.59.0 (--report-annotations-sync-status).
        annotation_matches = sync_stats_report.annotation_matches,
        annotation_mismatch = sync_stats_report.annotation_mismatch,
    );
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Mutex;

    use s3sync::config::args::parse_from_args;

    use super::*;

    /// Create a temp directory under the user's tmp dir using a UUID
    /// suffix. Removed at end of test via Drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("s7cmd_sync_unit_{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn s3_prefix(path: &str) -> (String, String) {
        match parse_from_args(vec!["s3sync", path, "/tmp/s7cmd-sync-prefix-test/"]) {
            Ok(args) => match Config::try_from(args).unwrap().source {
                s3sync::types::StoragePath::S3 { bucket, prefix } => (bucket, prefix),
                _ => panic!("{path} must be an S3 path"),
            },
            Err(e) => panic!("{path} must be accepted: {e}"),
        }
    }

    /// Any UTF-8 string is a valid S3 key, so the prefix of an S3 path is
    /// taken exactly as given (s3sync 1.64.1): `?` and `#` are part of it,
    /// percent escapes are not decoded, and `.` / `..` are not collapsed.
    /// Before, the path was parsed as a URL and a different key could be
    /// addressed silently.
    #[test]
    fn s3_prefix_is_taken_exactly_as_given() {
        let cases = [
            ("s3://test-bucket/data#old/", "data#old/"),
            ("s3://test-bucket/data?old/", "data?old/"),
            ("s3://test-bucket/data#", "data#"),
            ("s3://test-bucket/data?", "data?"),
            ("s3://test-bucket/a?b#c?d#e/", "a?b#c?d#e/"),
            ("s3://test-bucket/a#b?c/", "a#b?c/"),
            ("s3://test-bucket/#top", "#top"),
            ("s3://test-bucket/?top", "?top"),
            (
                "s3://test-bucket/dir/こんにちは#☃?x y/",
                "dir/こんにちは#☃?x y/",
            ),
            ("s3://test-bucket/a%23b%3Fc#d", "a%23b%3Fc#d"),
            ("s3://test-bucket/%FF", "%FF"),
            ("s3://test-bucket/100%", "100%"),
            ("s3://test-bucket/a%20b c", "a%20b c"),
            ("s3://test-bucket/a/../b", "a/../b"),
            ("s3://test-bucket/./a/./b/", "./a/./b/"),
            ("s3://test-bucket/a?x/../y#%2e%2e/z", "a?x/../y#%2e%2e/z"),
            ("s3://test-bucket/dir\\file", "dir\\file"),
            ("s3://test-bucket//", "/"),
            ("s3://test-bucket/", ""),
            ("s3://test-bucket", ""),
        ];

        for (path, expected_prefix) in cases {
            let (bucket, prefix) = s3_prefix(path);
            assert_eq!(bucket, "test-bucket", "path: {path}");
            assert_eq!(prefix, expected_prefix, "path: {path}");
        }
    }

    /// Without `/` after the bucket name, there is no prefix to attach `?`
    /// or `#` to, so the path is rejected instead of addressing something
    /// else.
    #[test]
    fn s3_path_with_query_or_fragment_after_bucket_is_rejected() {
        for path in [
            "s3://my-bucket?old",
            "s3://my-bucket#old",
            "s3://my-bucket?old/key",
            "s3://my-bucket#old/key",
        ] {
            for args in [
                vec!["s3sync", path, "/tmp/s7cmd-sync-prefix-test/"],
                vec!["s3sync", "/tmp/s7cmd-sync-prefix-test/", path],
            ] {
                assert!(parse_from_args(args).is_err(), "{path} must be rejected");
            }
        }
    }

    #[test]
    fn sigint_exit_code_is_130() {
        assert_eq!(SIGINT_EXIT_CODE, 130);
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn sigint_exit_code_follows_128_plus_signal_number_convention() {
        assert_eq!(
            SIGINT_EXIT_CODE,
            128 + nix::sys::signal::Signal::SIGINT as i32
        );
    }

    #[test]
    fn show_sync_report_summary_does_not_panic_with_default() {
        let report = Mutex::new(SyncStatsReport::default());
        show_sync_report_summary(report.lock().unwrap());
    }

    #[test]
    fn show_sync_report_summary_does_not_panic_with_populated_data() {
        let data = SyncStatsReport {
            number_of_objects: 5,
            etag_matches: 3,
            etag_mismatch: 1,
            etag_unknown: 1,
            checksum_matches: 2,
            checksum_mismatch: 1,
            checksum_unknown: 2,
            metadata_matches: 4,
            metadata_mismatch: 1,
            tagging_matches: 3,
            tagging_mismatch: 2,
            not_found: 1,
            annotation_matches: 2,
            annotation_mismatch: 1,
        };
        let report = Mutex::new(data);
        show_sync_report_summary(report.lock().unwrap());
    }

    /// Drives `run()` end-to-end via local-to-local sync (no AWS) so the
    /// callback registration, indicator wiring, pipeline run, and stats
    /// accounting paths get exercised. Errors from run() are intentionally
    /// ignored — the goal is coverage, not behavior verification.
    #[tokio::test]
    async fn run_pipeline_local_to_local() {
        let src_dir = TempDir::new();
        let tgt_dir = TempDir::new();
        std::fs::write(src_dir.path().join("file1.txt"), b"hello").unwrap();
        std::fs::write(src_dir.path().join("file2.txt"), b"world").unwrap();

        let src = format!("{}/", src_dir.path().to_string_lossy());
        let tgt = format!("{}/", tgt_dir.path().to_string_lossy());
        let args = vec![
            "s3sync",
            "--allow-both-local-storage",
            src.as_str(),
            tgt.as_str(),
        ];
        let config = Config::try_from(parse_from_args(args).unwrap()).unwrap();

        let _ = run(config).await;
    }

    /// Same as above but with `--dry-run` so the dry-run branches get
    /// exercised in both the run loop and the indicator.
    #[tokio::test]
    async fn run_pipeline_local_to_local_dry_run() {
        let src_dir = TempDir::new();
        let tgt_dir = TempDir::new();
        std::fs::write(src_dir.path().join("file.txt"), b"x").unwrap();

        let src = format!("{}/", src_dir.path().to_string_lossy());
        let tgt = format!("{}/", tgt_dir.path().to_string_lossy());
        let args = vec![
            "s3sync",
            "--allow-both-local-storage",
            "--dry-run",
            src.as_str(),
            tgt.as_str(),
        ];
        let config = Config::try_from(parse_from_args(args).unwrap()).unwrap();

        let _ = run(config).await;
    }

    /// `--report-sync-status` exercises the `if config.report_sync_status`
    /// arm in `run()` that hands the locked stats report to
    /// `show_sync_report_summary`. Local-to-local keeps the test
    /// off the network; we don't assert on exit code.
    #[tokio::test]
    async fn run_pipeline_local_to_local_report_sync_status() {
        let src_dir = TempDir::new();
        let tgt_dir = TempDir::new();
        std::fs::write(src_dir.path().join("file.txt"), b"x").unwrap();

        let src = format!("{}/", src_dir.path().to_string_lossy());
        let tgt = format!("{}/", tgt_dir.path().to_string_lossy());
        let args = vec![
            "s3sync",
            "--allow-both-local-storage",
            "--report-sync-status",
            src.as_str(),
            tgt.as_str(),
        ];
        let config = Config::try_from(parse_from_args(args).unwrap()).unwrap();

        let _ = run(config).await;
    }
}
