//! Phase Foster +10 — TUI sync banner widget state machine.
//!
//! Drains `SyncEvent` from the channel `launch_eager_sync` returns
//! and surfaces a one-line status string in the TUI's status bar.
//! Pure state machine (no rendering / no channel wiring) so the
//! truth table is unit-testable without an async runtime; the TUI
//! app loop integration is a follow-up that wires the channel +
//! calls `update` on each event.
//!
//! Display rules:
//! - During refresh: `"Refreshing N · M failed so far"` updates as
//!   each event arrives.
//! - On Done: `"Refreshed N · 2.1s"` — sticks for `IDLE_HIDE` seconds
//!   then disappears so the status bar reverts to normal.
//! - Hidden by default before any event arrives.

#![allow(dead_code)]

use std::time::{Duration, Instant};

use icelines_fetch::sync_engine::SyncEvent;

/// How long the post-Done summary lingers before fading out.
pub const IDLE_HIDE: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct SyncBanner {
    refreshed: usize,
    failed: usize,
    completed_at: Option<Instant>,
    completion_summary: Option<String>,
}

impl Default for SyncBanner {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncBanner {
    pub fn new() -> Self {
        Self {
            refreshed: 0,
            failed: 0,
            completed_at: None,
            completion_summary: None,
        }
    }

    /// Apply an event from the sync engine. Idempotent — calling
    /// twice with the same event is safe (the engine emits each
    /// event once but the widget shouldn't crash if a test replays).
    pub fn update(&mut self, event: SyncEvent) {
        match event {
            SyncEvent::Refreshed { .. } => {
                self.refreshed += 1;
                // Reset any previous completion state — a new event
                // means a fresh sync run.
                self.completed_at = None;
                self.completion_summary = None;
            }
            SyncEvent::Failed { .. } => {
                self.failed += 1;
                self.completed_at = None;
                self.completion_summary = None;
            }
            SyncEvent::Done {
                refreshed,
                failed,
                elapsed,
            } => {
                self.refreshed = refreshed;
                self.failed = failed;
                self.completed_at = Some(Instant::now());
                self.completion_summary = Some(format_done(refreshed, failed, elapsed));
            }
        }
    }

    /// Current display text. Returns `None` when nothing should
    /// render (no events yet, or post-Done idle window expired).
    /// Caller passes `now` so tests can drive the IDLE_HIDE timer
    /// deterministically.
    pub fn text_at(&self, now: Instant) -> Option<String> {
        if let Some(done_at) = self.completed_at {
            if now.duration_since(done_at) > IDLE_HIDE {
                return None;
            }
            return self.completion_summary.clone();
        }
        if self.refreshed == 0 && self.failed == 0 {
            return None;
        }
        Some(format!(
            "Refreshing — {} done{}",
            self.refreshed,
            if self.failed > 0 {
                format!(", {} failed", self.failed)
            } else {
                String::new()
            }
        ))
    }

    /// Production sibling — uses `Instant::now()`. Tests prefer
    /// `text_at` for deterministic timer assertions.
    pub fn text(&self) -> Option<String> {
        self.text_at(Instant::now())
    }
}

fn format_done(refreshed: usize, failed: usize, elapsed: Duration) -> String {
    let secs = elapsed.as_secs_f32();
    if failed == 0 {
        format!("Refreshed {refreshed} · {secs:.1}s")
    } else {
        format!("Refreshed {refreshed} · {failed} failed · {secs:.1}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_fetch::manifest::{DataKey, DataKind};

    fn refreshed_event() -> SyncEvent {
        SyncEvent::Refreshed {
            kind: DataKind::Bios,
            key: DataKey::Season(icelines_core::model::Season(20252026)),
        }
    }

    fn failed_event() -> SyncEvent {
        SyncEvent::Failed {
            kind: DataKind::Stats,
            key: DataKey::Season(icelines_core::model::Season(20252026)),
            error: "5xx".into(),
        }
    }

    fn done_event(refreshed: usize, failed: usize, secs: u64) -> SyncEvent {
        SyncEvent::Done {
            refreshed,
            failed,
            elapsed: Duration::from_secs(secs),
        }
    }

    #[test]
    fn l0_foster_plus10_banner_hidden_before_any_event() {
        let banner = SyncBanner::new();
        assert!(banner.text().is_none(), "no events yet → hidden");
    }

    #[test]
    fn l0_foster_plus10_banner_in_progress_counts_refreshes() {
        let mut banner = SyncBanner::new();
        banner.update(refreshed_event());
        banner.update(refreshed_event());
        banner.update(refreshed_event());
        let txt = banner.text().expect("visible during refresh");
        assert!(txt.contains("3 done"), "got: {txt}");
        assert!(!txt.contains("failed"), "no failures yet, got: {txt}");
    }

    #[test]
    fn l0_foster_plus10_banner_in_progress_includes_failures() {
        let mut banner = SyncBanner::new();
        banner.update(refreshed_event());
        banner.update(failed_event());
        banner.update(refreshed_event());
        let txt = banner.text().expect("visible");
        assert!(txt.contains("2 done"), "got: {txt}");
        assert!(txt.contains("1 failed"), "got: {txt}");
    }

    #[test]
    fn l0_foster_plus10_banner_done_renders_summary_with_elapsed() {
        let mut banner = SyncBanner::new();
        banner.update(refreshed_event());
        banner.update(done_event(1, 0, 2));
        let txt = banner.text().expect("visible immediately after Done");
        assert!(txt.contains("Refreshed 1"), "got: {txt}");
        assert!(txt.contains("2.0s"), "got: {txt}");
        assert!(!txt.contains("failed"), "no failures, got: {txt}");
    }

    #[test]
    fn l0_foster_plus10_banner_done_with_failures_includes_count() {
        let mut banner = SyncBanner::new();
        banner.update(done_event(5, 2, 3));
        let txt = banner.text().expect("visible");
        assert!(txt.contains("Refreshed 5"));
        assert!(txt.contains("2 failed"));
        assert!(txt.contains("3.0s"));
    }

    #[test]
    fn l0_foster_plus10_banner_hides_after_idle_timeout() {
        let mut banner = SyncBanner::new();
        banner.update(done_event(1, 0, 1));
        let done_at = banner.completed_at.unwrap();
        // Just before timeout — visible.
        let near = done_at + IDLE_HIDE - Duration::from_millis(100);
        assert!(
            banner.text_at(near).is_some(),
            "visible just before timeout"
        );
        // Just after — hidden.
        let past = done_at + IDLE_HIDE + Duration::from_millis(100);
        assert!(banner.text_at(past).is_none(), "hidden after IDLE_HIDE");
    }

    #[test]
    fn l0_foster_plus10_new_event_after_done_resets_to_in_progress() {
        // A second sync run after the first completes should show
        // the in-progress text again, not the stale Done summary.
        let mut banner = SyncBanner::new();
        banner.update(done_event(3, 0, 1));
        banner.update(refreshed_event()); // new run begins
        let txt = banner.text().expect("visible");
        assert!(
            !txt.starts_with("Refreshed"),
            "stale Done summary leaked, got: {txt}"
        );
        assert!(txt.contains("done"), "in-progress format, got: {txt}");
    }
}
