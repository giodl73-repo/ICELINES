// Phase Jack Adams.1 — MDI dashboard layout state.
//
// After Norris extracted per-screen state and Masterton declared
// per-screen chrome, Phase Jack Adams composes both into a
// multi-pane dashboard. `MdiLayout` holds the dashboard-only
// state: which side panes are visible, the chat-CLI command
// bar's input + history, the transient flash slot for command
// errors.
//
// Per spec forge-1/forge-2: workspace screen identity reuses the
// existing `crate::tui::app::Screen` enum. App::screen IS the
// workspace discriminator in MDI mode. No parallel
// `WorkspaceScreen` enum.
//
// Per spec glass-1: 2-row chrome total (Scores ribbon + combined
// footer/cmdbar). The cmdbar row is modal — chip-mode when the
// input is empty, prompt-mode when non-empty.
//
// Per spec glass-5: strict launch-time mode. Dashboard mode is set at
// launch; resize narrows panes adaptively (Adams.4) but never
// flips back to SDI mid-session (except when width drops below
// 100 cols, where MDI literally can't fit and we fall back to
// SDI render for that frame).

#![allow(clippy::module_name_repetitions)]
// Phase Adams.1 ships layout scaffolding (SidePane,
// COMMAND_HISTORY_CAP, push_command_history, history-cursor /
// flash-error fields) that Adams.2 will consume. Until then
// dead_code warnings would fire — allow at the module level;
// the lint becomes meaningful again once Adams.2 wires them.
#![allow(dead_code)]

// ── Layout state ─────────────────────────────────────────────────────────────

/// Phase Jack Adams.1 — MDI dashboard runtime state. Held by
/// `App` as `app.mdi: Option<MdiLayout>` — Some when the user
/// launched in dashboard mode, None for SDI modes (`--classic`
/// and `--standalone`).
#[derive(Debug)]
pub struct MdiLayout {
    /// User-toggleable side-pane visibility. Combined with
    /// `effective_panes(width)`'s adaptive auto-drop to decide
    /// what actually renders.
    pub show_favorites: bool,
    pub show_schedule: bool,

    /// Current text in the command bar. Empty → footer shows
    /// keybind chips (chip-mode); non-empty → footer shows the
    /// `>` prompt + cursor (prompt-mode).
    pub command_input: String,

    /// In-memory ring of recent successful command executions.
    /// Up/Down navigates this when the cmd bar has focus. Capped
    /// at `COMMAND_HISTORY_CAP`; deduped against the front entry.
    pub command_history: std::collections::VecDeque<String>,

    /// Cursor into `command_history` while user is walking it
    /// with Up/Down. None = live edit (the input shows whatever
    /// the user typed). Some(i) = showing history[i]; typing or
    /// Backspace breaks navigation back to None.
    pub command_history_cursor: Option<usize>,

    /// Transient flash slot for command-bar feedback. Set when
    /// `parse_command` fails OR `execute_command` returns an
    /// error. Cleared on the next successful action. Renders in
    /// the cmdbar row in red, replacing the `>` prompt for ~2s
    /// per spec glass-4.
    pub flash_error: Option<String>,

    /// Phase Adams.2 — command bar has captured keyboard input.
    /// Set true when user types `:` or `/` (entry triggers).
    /// Reset to false on Enter (after submit), Escape, or
    /// Backspace at empty input. While true, all key actions
    /// route to `handle_command_bar` and bypass per-screen
    /// keybinds.
    pub command_bar_focused: bool,

    /// Phase Adams.6 — AI fallback in flight. Holds the
    /// oneshot receiver for the spawned provider call. None
    /// means no AI request is pending. The render loop polls
    /// this each tick via `App::mdi_poll_ai`. When ready, the
    /// returned command string is re-parsed and executed.
    ///
    /// Holding the input alongside the receiver lets the poll
    /// path surface a clear flash on parse-of-AI-response
    /// errors (we keep the original user input around so the
    /// user can edit it manually).
    pub ai_pending: Option<AiPending>,
}

/// Phase Adams.6 — pending AI fallback request. Stored on
/// `MdiLayout::ai_pending` until the spawned task either
/// completes or the request is cancelled (Esc).
pub struct AiPending {
    pub original_input: String,
    pub rx: tokio::sync::oneshot::Receiver<Result<String, crate::ai::AiError>>,
    pub provider_name: &'static str,
    pub started_at: std::time::Instant,
}

impl std::fmt::Debug for AiPending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiPending")
            .field("original_input", &self.original_input)
            .field("provider_name", &self.provider_name)
            .field("rx", &"<oneshot::Receiver>")
            .field("started_at", &self.started_at)
            .finish()
    }
}

/// Phase Adams.1 — max number of recent commands retained in
/// `MdiLayout::command_history`. Bounded so the cmdbar history
/// ring doesn't grow unboundedly across a long session.
pub const COMMAND_HISTORY_CAP: usize = 50;

impl Default for MdiLayout {
    fn default() -> Self {
        Self {
            show_favorites: true,
            show_schedule: true,
            command_input: String::new(),
            command_history: std::collections::VecDeque::new(),
            command_history_cursor: None,
            flash_error: None,
            command_bar_focused: false,
            ai_pending: None,
        }
    }
}

// ── Side-pane discriminator ──────────────────────────────────────────────────

/// Phase Adams.1 — discriminator for the user-toggleable side
/// panes. Used by the `Hide` / `Show` slash commands and by the
/// `Ctrl+H` / `Ctrl+L` keybinds to flip the right field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidePane {
    Favorites,
    Schedule,
}

// ── Adaptive visibility ──────────────────────────────────────────────────────

/// Phase Adams.4 — visibility decision for the four MDI regions
/// at a given terminal width. Combines the user's manual toggles
/// (`mdi.show_favorites`, `mdi.show_schedule`) with the adaptive
/// auto-drop thresholds.
///
/// Spec adaptive thresholds:
/// - **≥160 cols**: full MDI — all four regions visible.
/// - **120-159**: drop Schedule (rightmost first — schedule is
///   more transient than favorites).
/// - **100-119**: drop Favorites too — workspace gets the body.
/// - **<100**: too narrow for MDI; caller falls back to SDI
///   render for this frame (see `MdiLayout::collapse_to_sdi`).
///
/// `scores` and `workspace` are always true when this struct is
/// constructed (the caller has already passed the
/// `collapse_to_sdi` check).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneVisibility {
    pub scores: bool,
    pub favorites: bool,
    pub workspace: bool,
    pub schedule: bool,
}

impl MdiLayout {
    /// Phase Adams.4 — adaptive auto-drop. Manual toggles take
    /// precedence over the adaptive default — e.g., a user who
    /// manually hid Favorites at ≥160 cols still has it hidden
    /// (the adaptive layer says "visible" but the manual toggle
    /// says "hide", and the manual toggle wins).
    ///
    /// Caller must have already verified `width >= 100` via
    /// `collapse_to_sdi(width) == false`. At narrower widths the
    /// caller falls back to the SDI render path for the frame.
    pub fn effective_panes(&self, width: u16) -> PaneVisibility {
        // Spec adaptive thresholds: ≥160 full / 120-159 drop
        // Schedule / 100-119 drop Favorites / <100 SDI fallback.
        let adaptive_favorites = width >= 120;
        let adaptive_schedule = width >= 160;

        PaneVisibility {
            scores: true,
            favorites: self.show_favorites && adaptive_favorites,
            workspace: true,
            schedule: self.show_schedule && adaptive_schedule,
        }
    }

    /// Phase Adams.4 — `true` when the terminal is too narrow
    /// for any reasonable MDI rendering. Caller branches to the
    /// SDI render path for this frame.
    pub fn collapse_to_sdi(width: u16) -> bool {
        width < 100
    }
}

// ── Command history helper ──────────────────────────────────────────────────

/// Phase Adams.1 — push `entry` onto the front of the cmdbar
/// history ring (newest first). Dedupes against an identical
/// existing front so hammering Enter on the same command doesn't
/// fill the ring with duplicates. Trims the back when the ring
/// is at cap.
pub fn push_command_history(history: &mut std::collections::VecDeque<String>, entry: String) {
    if let Some(front) = history.front() {
        if front == &entry {
            return;
        }
    }
    history.push_front(entry);
    while history.len() > COMMAND_HISTORY_CAP {
        history.pop_back();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Phase Jack Adams.1 — MdiLayout contract ────────────────────────────

    /// Default MDI has both side panes visible — first launch
    /// shows the full dashboard at wide widths.
    #[test]
    fn l0_adams_mdi_default_both_side_panes_visible() {
        let m = MdiLayout::default();
        assert!(m.show_favorites);
        assert!(m.show_schedule);
    }

    /// Default command bar is empty (chip-mode footer kicks in).
    #[test]
    fn l0_adams_mdi_default_cmdbar_empty() {
        let m = MdiLayout::default();
        assert_eq!(m.command_input, "");
        assert!(m.command_history.is_empty());
        assert!(m.command_history_cursor.is_none());
        assert!(m.flash_error.is_none());
    }

    // ── Phase Adams.4 — adaptive visibility ────────────────────────────────

    /// At ≥160 cols, all four regions visible.
    #[test]
    fn l0_adams_effective_panes_at_160_full_mdi() {
        let m = MdiLayout::default();
        let v = m.effective_panes(160);
        assert!(v.scores);
        assert!(v.favorites);
        assert!(v.workspace);
        assert!(v.schedule);
    }

    /// At 159 (one below ≥160), Schedule drops first.
    #[test]
    fn l0_adams_effective_panes_at_159_drops_schedule() {
        let m = MdiLayout::default();
        let v = m.effective_panes(159);
        assert!(v.scores);
        assert!(v.favorites);
        assert!(v.workspace);
        assert!(
            !v.schedule,
            "Schedule must drop at width 159 (under ≥160 threshold)"
        );
    }

    /// At 119 (one below ≥120), Favorites drops too.
    #[test]
    fn l0_adams_effective_panes_at_119_drops_favorites() {
        let m = MdiLayout::default();
        let v = m.effective_panes(119);
        assert!(v.scores);
        assert!(
            !v.favorites,
            "Favorites must drop at width 119 (under ≥120 threshold; the spec says <100 for full Favorites drop, but Adams.4 thresholds are stricter — anything <120 keeps only workspace + scores)"
        );
        assert!(v.workspace);
        assert!(!v.schedule);
    }

    /// At 100 (just above the SDI fallback boundary), both
    /// favorites and schedule are dropped — workspace + scores
    /// only.
    #[test]
    fn l0_adams_effective_panes_at_100_workspace_only() {
        let m = MdiLayout::default();
        let v = m.effective_panes(100);
        assert!(v.scores);
        assert!(!v.favorites);
        assert!(v.workspace);
        assert!(!v.schedule);
    }

    /// `collapse_to_sdi(99)` is true — too narrow to render MDI.
    #[test]
    fn l0_adams_collapse_to_sdi_at_99() {
        assert!(MdiLayout::collapse_to_sdi(99));
    }

    /// `collapse_to_sdi(100)` is false — exactly at the boundary,
    /// MDI renders.
    #[test]
    fn l0_adams_collapse_to_sdi_at_100_is_false() {
        assert!(!MdiLayout::collapse_to_sdi(100));
    }

    /// Manual toggle overrides the adaptive default — user
    /// hides Favorites at width 200, the adaptive layer would
    /// keep it visible, but the manual toggle wins.
    #[test]
    fn l0_adams_manual_toggle_overrides_adaptive() {
        let m = MdiLayout {
            show_favorites: false,
            ..Default::default()
        };
        let v = m.effective_panes(200);
        assert!(!v.favorites, "manual hide must override adaptive 'show'");
        // Schedule is still visible (manual toggle on, adaptive on).
        assert!(v.schedule);
    }

    // ── Property-style adaptive layout test (post-review bench-4) ─────────

    /// For every reasonable width 80..220 step 4, the result is
    /// a valid PaneVisibility (no panic, doesn't claim widths
    /// the layout can't satisfy). The `collapse_to_sdi` boundary
    /// at 100 is checked separately; this test runs above it
    /// (≥100) where MDI is supposed to render.
    #[test]
    fn l0_adams_effective_panes_property_at_every_width() {
        let m = MdiLayout::default();
        for width in (100u16..=220).step_by(4) {
            let v = m.effective_panes(width);
            // Workspace + scores are always true when MDI
            // renders.
            assert!(v.scores, "scores must be true at width {width}");
            assert!(v.workspace, "workspace must be true at width {width}");
            // Favorites is visible iff width ≥ 120.
            assert_eq!(v.favorites, width >= 120, "favorites at width {width}");
            // Schedule is visible iff width ≥ 160.
            assert_eq!(v.schedule, width >= 160, "schedule at width {width}");
        }
    }

    // ── Command history ring ──────────────────────────────────────────────

    /// `push_command_history` adds to the front, dedupes
    /// against an identical front entry.
    #[test]
    fn l0_adams_push_command_history_dedupes_consecutive() {
        use std::collections::VecDeque;
        let mut h: VecDeque<String> = VecDeque::new();
        push_command_history(&mut h, "stats".into());
        push_command_history(&mut h, "stats".into()); // dup, ignored
        push_command_history(&mut h, "goalies".into());
        push_command_history(&mut h, "stats".into()); // not consecutive — kept
        assert_eq!(h.len(), 3);
        assert_eq!(h[0], "stats");
        assert_eq!(h[1], "goalies");
        assert_eq!(h[2], "stats");
    }

    /// `push_command_history` caps at `COMMAND_HISTORY_CAP`.
    #[test]
    fn l0_adams_push_command_history_caps_at_max() {
        use std::collections::VecDeque;
        let mut h: VecDeque<String> = VecDeque::new();
        for i in 0..(COMMAND_HISTORY_CAP + 5) {
            push_command_history(&mut h, format!("cmd-{i}"));
        }
        assert_eq!(h.len(), COMMAND_HISTORY_CAP);
        // Front is the newest push.
        assert_eq!(h[0], format!("cmd-{}", COMMAND_HISTORY_CAP + 4));
        // Back is the oldest entry that survived (oldest 5 fell off).
        assert_eq!(h[COMMAND_HISTORY_CAP - 1], "cmd-5");
    }

    // ── SidePane discriminator ────────────────────────────────────────────

    /// `SidePane` is `Copy + PartialEq`. Required because the
    /// orchestrator routes `Hide(Favorites)` / `Show(Schedule)`
    /// commands through pattern matches.
    #[test]
    fn l0_adams_sidepane_copy_partial_eq() {
        let a = SidePane::Favorites;
        let b = a; // copy
        assert_eq!(a, b);
        assert_ne!(a, SidePane::Schedule);
    }
}
