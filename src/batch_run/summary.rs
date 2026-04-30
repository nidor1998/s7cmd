//! End-of-run summary line.

use std::time::Duration;

#[derive(Debug, Default)]
pub struct Summary {
    pub ok: u64,
    pub failed: u64,
    pub skipped: u64,
    pub elapsed: Duration,
}

impl Summary {
    pub fn record(&mut self, exit_code: i32) {
        if exit_code == 0 {
            self.ok += 1;
        } else {
            self.failed += 1;
        }
    }

    pub fn format(&self) -> String {
        let secs = self.elapsed.as_secs_f32();
        format!(
            "batch-run: {} ok, {} failed, {} skipped, elapsed {:.1}s",
            self.ok, self.failed, self.skipped, secs
        )
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
    }

    #[test]
    fn record_increments_failed_on_nonzero() {
        let mut s = Summary::default();
        s.record(1);
        s.record(3);
        s.record(130);
        assert_eq!(s.failed, 3);
        assert_eq!(s.ok, 0);
    }

    #[test]
    fn format_includes_all_counts_and_elapsed() {
        let mut s = Summary::default();
        s.ok = 47;
        s.failed = 2;
        s.skipped = 1;
        s.elapsed = Duration::from_millis(12_400);
        let line = s.format();
        assert_eq!(line, "batch-run: 47 ok, 2 failed, 1 skipped, elapsed 12.4s");
    }

    #[test]
    fn format_zero_elapsed() {
        let s = Summary::default();
        assert_eq!(
            s.format(),
            "batch-run: 0 ok, 0 failed, 0 skipped, elapsed 0.0s"
        );
    }
}
