//! Progress bar wrapper around indicatif. Only drawn in read-all mode
//! when stderr is a TTY and `--no-summary` is not set.

use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;

pub struct Progress {
    bar: Option<ProgressBar>,
    ok: u64,
    failed: u64,
}

impl Progress {
    /// Build a new bar. If progress should not be shown (streaming mode,
    /// `--no-summary`, or non-TTY stderr), return a Progress with no bar.
    pub fn new(total: u64, show: bool) -> Self {
        if !show {
            return Self {
                bar: None,
                ok: 0,
                failed: 0,
            };
        }
        let bar = ProgressBar::new(total);
        bar.set_style(
            ProgressStyle::with_template(
                "[{bar:40.cyan/blue}] {pos}/{len} ({msg}) {elapsed_precise}",
            )
            .expect("hard-coded template")
            .progress_chars("=> "),
        );
        bar.set_message("0 ok, 0 failed");
        Self {
            bar: Some(bar),
            ok: 0,
            failed: 0,
        }
    }

    /// Decide whether to draw the bar. Returns true only when
    /// no_summary=false, streaming=false, AND stderr is a TTY.
    pub fn should_show(no_summary: bool, streaming: bool) -> bool {
        if no_summary || streaming {
            return false;
        }
        std::io::stderr().is_terminal()
    }

    /// Record a completion (success or failure) and update the message.
    pub fn tick(&mut self, exit_code: i32) {
        if exit_code == 0 {
            self.ok += 1;
        } else {
            self.failed += 1;
        }
        if let Some(bar) = &self.bar {
            bar.set_message(format!("{} ok, {} failed", self.ok, self.failed));
            bar.inc(1);
        }
    }

    pub fn finish(&self) {
        if let Some(bar) = &self.bar {
            bar.finish();
        }
    }

    /// Stop the bar at its current position (do NOT advance to 100%).
    /// Use when fail-fast tripped or SIGINT fired — the visible position
    /// then reflects how far execution actually got, not a misleading
    /// "fully complete" jump.
    pub fn abandon(&self) {
        if let Some(bar) = &self.bar {
            bar.abandon();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_show_false_when_no_summary() {
        assert!(!Progress::should_show(true, false));
    }

    #[test]
    fn should_show_false_when_streaming() {
        assert!(!Progress::should_show(false, true));
    }

    #[test]
    fn should_show_false_when_no_summary_and_streaming() {
        assert!(!Progress::should_show(true, true));
    }

    // We don't test the TTY-true branch because the test runner's
    // stderr is typically not a TTY; the boolean wiring is exercised
    // by the should_show=false tests above and the Progress::new
    // branch is exercised below.

    #[test]
    fn new_with_show_false_has_no_bar() {
        let p = Progress::new(10, false);
        assert!(p.bar.is_none());
        assert_eq!(p.ok, 0);
        assert_eq!(p.failed, 0);
    }

    #[test]
    fn tick_counts_success_and_failure_without_bar() {
        let mut p = Progress::new(5, false);
        p.tick(0);
        p.tick(0);
        p.tick(1);
        p.tick(3);
        assert_eq!(p.ok, 2);
        assert_eq!(p.failed, 2);
    }

    #[test]
    fn finish_no_op_without_bar() {
        let p = Progress::new(0, false);
        p.finish(); // must not panic
    }

    #[test]
    fn abandon_no_op_without_bar() {
        let p = Progress::new(0, false);
        p.abandon(); // must not panic
    }

    // The following tests exercise the `show=true` branch of `Progress::new`
    // and the `Some(bar)` branches of `tick`, `finish`, and `abandon`.
    // Indicatif's `ProgressBar` is safe to construct in tests; it does not
    // attempt to draw to a non-TTY stderr (and we set a hidden draw target
    // implicitly because the test runner's stderr is not a TTY — but we
    // also belt-and-suspenders by not asserting on visible output).

    #[test]
    fn new_with_show_true_creates_bar() {
        let p = Progress::new(10, true);
        assert!(p.bar.is_some());
        assert_eq!(p.ok, 0);
        assert_eq!(p.failed, 0);
    }

    #[test]
    fn tick_with_bar_updates_counts_and_does_not_panic() {
        let mut p = Progress::new(5, true);
        p.tick(0);
        p.tick(1);
        p.tick(0);
        p.tick(2);
        assert_eq!(p.ok, 2);
        assert_eq!(p.failed, 2);
    }

    #[test]
    fn finish_with_bar_does_not_panic() {
        let mut p = Progress::new(2, true);
        p.tick(0);
        p.tick(0);
        p.finish();
    }

    #[test]
    fn abandon_with_bar_does_not_panic() {
        let mut p = Progress::new(3, true);
        p.tick(0);
        p.abandon();
    }

    // Calling `should_show(false, false)` reaches the `is_terminal()` line.
    // The boolean result depends on whether the test runner's stderr is a
    // TTY, but the line is exercised either way.
    #[test]
    fn should_show_reaches_is_terminal_branch() {
        // Don't assert the exact bool — just call to cover line 46.
        let _ = Progress::should_show(false, false);
    }
}
