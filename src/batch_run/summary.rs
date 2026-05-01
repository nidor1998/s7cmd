//! End-of-run summary line.

use std::time::Duration;

#[derive(Debug, Default)]
pub struct Summary {
    pub ok: u64,
    pub failed: u64,
    pub warning: u64,
    pub skipped: u64,
    pub elapsed: Duration,
}

impl Summary {
    /// Bucket by exit code, matching `Progress::tick` and `is_stop_worthy`:
    /// 0 → ok, 3/4 (`EXIT_CODE_WARNING`, `EXIT_CODE_NOT_FOUND`) → warning,
    /// 130 (`EXIT_CODE_INTERRUPTED`, the conventional Unix code for SIGINT —
    /// returned by per-subcommand cancellation handlers when the user hits
    /// Ctrl-C) → skipped, any other non-zero → failed.
    pub fn record(&mut self, exit_code: i32) {
        match exit_code {
            0 => self.ok += 1,
            3 | 4 => self.warning += 1,
            130 => self.skipped += 1,
            _ => self.failed += 1,
        }
    }

    pub fn format(&self) -> String {
        let secs = self.elapsed.as_secs_f32();
        format!(
            "batch-run: {} succeeded, {} failed, {} warnings, {} skipped, elapsed {:.1}s",
            self.ok, self.failed, self.warning, self.skipped, secs
        )
    }

    /// Machine-readable summary used when `--json-tracing` is set on
    /// `batch-run`. Carries the same five counters as `format()`. Elapsed
    /// is rounded to milliseconds — fine-grained enough for log analysis,
    /// coarse enough to avoid float-noise tails like `3.4000000000001`.
    pub fn format_json(&self) -> String {
        let elapsed_seconds = (self.elapsed.as_millis() as f64) / 1000.0;
        serde_json::json!({
            "summary": "batch-run",
            "succeeded": self.ok,
            "failed": self.failed,
            "warnings": self.warning,
            "skipped": self.skipped,
            "elapsed_seconds": elapsed_seconds,
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_increments_ok_on_zero() {
        let mut s = Summary::default();
        s.record(0);
        s.record(0);
        assert_eq!(s.ok, 2);
        assert_eq!(s.failed, 0);
        assert_eq!(s.warning, 0);
    }

    #[test]
    fn record_increments_warning_on_3_and_4() {
        let mut s = Summary::default();
        s.record(3);
        s.record(4);
        s.record(4);
        assert_eq!(s.warning, 3);
        assert_eq!(s.failed, 0);
        assert_eq!(s.ok, 0);
    }

    #[test]
    fn record_increments_failed_on_other_nonzero() {
        let mut s = Summary::default();
        s.record(1);
        s.record(2);
        s.record(255);
        assert_eq!(s.failed, 3);
        assert_eq!(s.warning, 0);
        assert_eq!(s.ok, 0);
        assert_eq!(s.skipped, 0);
    }

    #[test]
    fn record_increments_skipped_on_130() {
        let mut s = Summary::default();
        s.record(130);
        s.record(130);
        assert_eq!(s.skipped, 2);
        assert_eq!(s.failed, 0);
        assert_eq!(s.warning, 0);
        assert_eq!(s.ok, 0);
    }

    #[test]
    fn format_includes_all_counts_and_elapsed() {
        let s = Summary {
            ok: 47,
            failed: 2,
            warning: 5,
            skipped: 1,
            elapsed: Duration::from_millis(12_400),
        };
        assert_eq!(
            s.format(),
            "batch-run: 47 succeeded, 2 failed, 5 warnings, 1 skipped, elapsed 12.4s"
        );
    }

    #[test]
    fn format_zero_elapsed() {
        let s = Summary::default();
        assert_eq!(
            s.format(),
            "batch-run: 0 succeeded, 0 failed, 0 warnings, 0 skipped, elapsed 0.0s"
        );
    }

    #[test]
    fn format_json_contains_all_fields() {
        let s = Summary {
            ok: 48,
            failed: 1,
            warning: 2,
            skipped: 1,
            elapsed: Duration::from_millis(3400),
        };
        let json: serde_json::Value = serde_json::from_str(&s.format_json()).unwrap();
        assert_eq!(json["summary"], "batch-run");
        assert_eq!(json["succeeded"], 48);
        assert_eq!(json["failed"], 1);
        assert_eq!(json["warnings"], 2);
        assert_eq!(json["skipped"], 1);
        assert_eq!(json["elapsed_seconds"], 3.4);
    }

    #[test]
    fn format_json_zero_elapsed() {
        let s = Summary::default();
        let json: serde_json::Value = serde_json::from_str(&s.format_json()).unwrap();
        assert_eq!(json["succeeded"], 0);
        assert_eq!(json["failed"], 0);
        assert_eq!(json["warnings"], 0);
        assert_eq!(json["skipped"], 0);
        assert_eq!(json["elapsed_seconds"], 0.0);
    }

    #[test]
    fn format_json_rounds_to_milliseconds() {
        let s = Summary {
            elapsed: Duration::from_micros(3_456_789), // 3.456789 s
            ..Summary::default()
        };
        let json: serde_json::Value = serde_json::from_str(&s.format_json()).unwrap();
        assert_eq!(json["elapsed_seconds"], 3.456);
    }
}
