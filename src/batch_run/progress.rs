//! Progress bar wrapper around indicatif. Only drawn in read-all mode
//! when stderr is a TTY and `--no-progress` is not set. Decoupled from
//! `--no-summary`: that flag controls only the trailing summary line.

use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;

pub struct Progress {
    bar: Option<ProgressBar>,
    ok: u64,
    failed: u64,
    warning: u64,
    skipped: u64,
}

impl Progress {
    /// Build a new bar. If progress should not be shown (streaming mode,
    /// `--no-progress`, or non-TTY stderr), return a Progress with no bar.
    pub fn new(total: u64, show: bool) -> Self {
        if !show {
            return Self {
                bar: None,
                ok: 0,
                failed: 0,
                warning: 0,
                skipped: 0,
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
        bar.set_message("0 successes, 0 failures, 0 warnings, 0 skipped");
        Self {
            bar: Some(bar),
            ok: 0,
            failed: 0,
            warning: 0,
            skipped: 0,
        }
    }

    /// Decide whether to draw the bar. Returns true only when
    /// streaming=false, no_progress=false, AND stderr is a TTY.
    /// `--no-summary` no longer participates: the bar and the trailing
    /// summary line are independent visual elements, each controlled by
    /// its own flag.
    pub fn should_show(streaming: bool, no_progress: bool) -> bool {
        if streaming || no_progress {
            return false;
        }
        std::io::stderr().is_terminal()
    }

    /// Record a completion and update the message. Exit codes 3
    /// (`EXIT_CODE_WARNING`) and 4 (`EXIT_CODE_NOT_FOUND`) — kept literal
    /// to avoid a cross-module dep just for two numbers — count as
    /// warnings. Exit code 130 (the conventional Unix code for SIGINT,
    /// returned by per-subcommand cancellation handlers when the user
    /// hits Ctrl-C) counts as skipped. Everything else nonzero counts
    /// as a failure.
    pub fn tick(&mut self, exit_code: i32) {
        match exit_code {
            0 => self.ok += 1,
            3 | 4 => self.warning += 1,
            130 => self.skipped += 1,
            _ => self.failed += 1,
        }
        if let Some(bar) = &self.bar {
            bar.set_message(format!(
                "{} successes, {} failures, {} warnings, {} skipped",
                self.ok, self.failed, self.warning, self.skipped
            ));
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
    fn should_show_false_when_streaming() {
        assert!(!Progress::should_show(true, false));
    }

    #[test]
    fn should_show_false_when_no_progress() {
        assert!(!Progress::should_show(false, true));
    }

    #[test]
    fn should_show_false_when_streaming_and_no_progress() {
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
        assert_eq!(p.warning, 0);
        assert_eq!(p.skipped, 0);
    }

    #[test]
    fn tick_classifies_success_warning_failure_and_skipped_without_bar() {
        let mut p = Progress::new(9, false);
        p.tick(0);
        p.tick(0);
        p.tick(1);
        p.tick(2);
        p.tick(3);
        p.tick(4);
        p.tick(4);
        p.tick(130);
        p.tick(130);
        assert_eq!(p.ok, 2);
        assert_eq!(p.warning, 3); // 3, 4, 4
        assert_eq!(p.failed, 2); // 1, 2
        assert_eq!(p.skipped, 2); // 130, 130
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
        assert_eq!(p.warning, 0);
        assert_eq!(p.skipped, 0);
    }

    #[test]
    fn tick_with_bar_updates_counts_and_does_not_panic() {
        let mut p = Progress::new(5, true);
        p.tick(0);
        p.tick(1);
        p.tick(0);
        p.tick(2);
        p.tick(3);
        assert_eq!(p.ok, 2);
        assert_eq!(p.failed, 2);
        assert_eq!(p.warning, 1);
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

    // Calling `should_show(false, false)` reaches the `is_terminal()`
    // line. The boolean result depends on whether the test runner's
    // stderr is a TTY, but the line is exercised either way.
    #[test]
    fn should_show_reaches_is_terminal_branch() {
        let _ = Progress::should_show(false, false);
    }
}
