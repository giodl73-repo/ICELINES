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
// Current MDI chrome is data-first: scores ribbon, workspace body,
// per-screen keybinds, verb hints, and the command bar. The command
// bar row is modal — chip-mode when input is empty, prompt-mode
// when non-empty.
//
// Per spec glass-5: strict launch-time mode. Dashboard mode is set at
// launch; resize narrows panes adaptively (Adams.4) but never
// flips back to SDI mid-session (except when width drops below
// 100 cols, where MDI literally can't fit and we fall back to
// SDI render for that frame).

#![allow(clippy::module_name_repetitions)]

#[cfg(test)]
use icelines_core::WorkbenchPaneModelId;
use icelines_core::{
    workbench_experience, workbench_pane_binding, WorkbenchExperience, WorkbenchExperienceId,
    WorkbenchId, WorkbenchLayoutError, WorkbenchLayoutRecord, WorkbenchPaneBinding,
    WorkbenchPaneBindingId, WorkbenchSurface, WorkbenchZone, WORKBENCH_CATALOG,
};

// ── Layout state ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdiFocus {
    ActivityRail,
    LeftPane,
    Workspace,
    RightPane,
}

/// Phase Jack Adams.1 — MDI dashboard runtime state. Held by
/// `App` as `app.mdi: Option<MdiLayout>` — Some when the user
/// launched in dashboard mode, None for SDI modes (`--classic`
/// and `--standalone`).
#[derive(Debug)]
pub struct MdiLayout {
    /// Pulse 03 — which visible workbench zone has keyboard focus.
    /// The center workspace remains the default so existing per-screen
    /// key handling stays intact until the user tabs into the rail/panes.
    pub focus: MdiFocus,

    /// Pulse 03 — selected row in the shared activity/catalog rail.
    pub catalog_selected: usize,

    /// Compose the Bench.03 — active bound experience, if the
    /// current workspace was selected through a shared experience.
    /// Free-form activity-rail navigation clears this back to None.
    pub active_experience: Option<WorkbenchExperienceId>,

    /// Compose the Bench.03 — shared pane-binding identities backing
    /// the concrete side panes. The render path derives both the title
    /// and body from the binding so labels never drift from content.
    pub left_pane_binding: WorkbenchPaneBindingId,
    pub right_pane_binding: WorkbenchPaneBindingId,

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

    /// Transient success / handoff feedback shown in the command
    /// row after a successful command while sticky command focus is
    /// retained. Cleared on the next edit or cancel.
    pub flash_info: Option<String>,

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
            focus: MdiFocus::Workspace,
            catalog_selected: 0,
            active_experience: None,
            left_pane_binding: WorkbenchPaneBindingId::FavoritesLeft,
            right_pane_binding: WorkbenchPaneBindingId::ScheduleRight,
            show_favorites: true,
            show_schedule: true,
            command_input: String::new(),
            command_history: std::collections::VecDeque::new(),
            command_history_cursor: None,
            flash_error: None,
            flash_info: None,
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
    pub activity_catalog: bool,
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
            activity_catalog: true,
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

    pub fn set_side_pane_visible(&mut self, pane: SidePane, visible: bool) {
        match pane {
            SidePane::Favorites => {
                self.show_favorites = visible;
                if !visible && self.focus == MdiFocus::LeftPane {
                    self.focus = MdiFocus::Workspace;
                }
            }
            SidePane::Schedule => {
                self.show_schedule = visible;
                if !visible && self.focus == MdiFocus::RightPane {
                    self.focus = MdiFocus::Workspace;
                }
            }
        }
    }

    pub fn toggle_side_pane(&mut self, pane: SidePane) -> bool {
        let visible = match pane {
            SidePane::Favorites => !self.show_favorites,
            SidePane::Schedule => !self.show_schedule,
        };
        self.set_side_pane_visible(pane, visible);
        visible
    }

    pub fn focus_next(&mut self) {
        self.focus = self.next_focus(false);
    }

    pub fn focus_prev(&mut self) {
        self.focus = self.next_focus(true);
    }

    fn next_focus(&self, reverse: bool) -> MdiFocus {
        let order = self.focus_order();
        let current = order
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or(0);
        let next = if reverse {
            current.checked_sub(1).unwrap_or(order.len() - 1)
        } else {
            (current + 1) % order.len()
        };
        order[next]
    }

    fn focus_order(&self) -> Vec<MdiFocus> {
        let mut order = vec![MdiFocus::ActivityRail];
        if self.show_favorites {
            order.push(MdiFocus::LeftPane);
        }
        order.push(MdiFocus::Workspace);
        if self.show_schedule {
            order.push(MdiFocus::RightPane);
        }
        order
    }

    pub fn select_next_catalog_entry(&mut self) {
        let len = WORKBENCH_CATALOG.len();
        if len > 0 {
            self.catalog_selected = (self.catalog_selected + 1).min(len - 1);
        }
    }

    pub fn select_prev_catalog_entry(&mut self) {
        self.catalog_selected = self.catalog_selected.saturating_sub(1);
    }

    pub fn selected_workbench_id(&self) -> Option<WorkbenchId> {
        WORKBENCH_CATALOG
            .get(self.catalog_selected)
            .map(|entry| entry.id)
    }

    pub fn select_workbench_id(&mut self, id: WorkbenchId) {
        if let Some(idx) = WORKBENCH_CATALOG.iter().position(|entry| entry.id == id) {
            self.catalog_selected = idx;
        }
    }

    pub fn active_experience(&self) -> Option<&'static WorkbenchExperience> {
        self.active_experience.and_then(workbench_experience)
    }

    pub fn left_pane_binding(&self) -> Option<&'static WorkbenchPaneBinding> {
        workbench_pane_binding(self.left_pane_binding)
    }

    pub fn right_pane_binding(&self) -> Option<&'static WorkbenchPaneBinding> {
        workbench_pane_binding(self.right_pane_binding)
    }

    #[cfg(test)]
    pub fn left_pane_model(&self) -> Option<WorkbenchPaneModelId> {
        self.left_pane_binding().map(|binding| binding.pane_model)
    }

    #[cfg(test)]
    pub fn right_pane_model(&self) -> Option<WorkbenchPaneModelId> {
        self.right_pane_binding().map(|binding| binding.pane_model)
    }

    pub fn apply_experience(&mut self, experience: &'static WorkbenchExperience) {
        self.active_experience = Some(experience.id);
        if let Some(id) = experience
            .left_pane
            .filter(|id| Self::binding_is_tui_zone(*id, WorkbenchZone::LeftPane))
        {
            self.left_pane_binding = id;
        }
        if let Some(id) = experience
            .right_pane
            .filter(|id| Self::binding_is_tui_zone(*id, WorkbenchZone::RightPane))
        {
            self.right_pane_binding = id;
        }
    }

    pub fn clear_active_experience(&mut self) {
        self.active_experience = None;
    }

    pub fn apply_persisted_layout(
        &mut self,
        layout: &WorkbenchLayoutRecord,
    ) -> Result<(), WorkbenchLayoutError> {
        layout.validate_for_surface(WorkbenchSurface::Tui)?;
        let center = layout.center_id()?;
        if let Some(index) = WORKBENCH_CATALOG
            .iter()
            .position(|entry| entry.id == center)
        {
            self.catalog_selected = index;
        }
        self.left_pane_binding = layout.left_id()?;
        self.right_pane_binding = layout.right_id()?;
        self.active_experience = layout.experience_id()?;
        self.show_favorites = true;
        self.show_schedule = true;
        Ok(())
    }

    pub fn cycle_left_pane(&mut self, reverse: bool) -> Option<&'static WorkbenchPaneBinding> {
        let id = Self::next_binding_id(WorkbenchZone::LeftPane, self.left_pane_binding, reverse)?;
        self.left_pane_binding = id;
        self.clear_active_experience();
        self.left_pane_binding()
    }

    pub fn cycle_right_pane(&mut self, reverse: bool) -> Option<&'static WorkbenchPaneBinding> {
        let id = Self::next_binding_id(WorkbenchZone::RightPane, self.right_pane_binding, reverse)?;
        self.right_pane_binding = id;
        self.clear_active_experience();
        self.right_pane_binding()
    }

    fn next_binding_id(
        zone: WorkbenchZone,
        current: WorkbenchPaneBindingId,
        reverse: bool,
    ) -> Option<WorkbenchPaneBindingId> {
        let bindings: Vec<_> = crate::tui::workbench::tui_pane_bindings_for_zone(zone).collect();
        if bindings.is_empty() {
            return None;
        }
        let current_idx = bindings
            .iter()
            .position(|binding| binding.id == current)
            .unwrap_or(0);
        let next_idx = if reverse {
            current_idx.checked_sub(1).unwrap_or(bindings.len() - 1)
        } else {
            (current_idx + 1) % bindings.len()
        };
        Some(bindings[next_idx].id)
    }

    fn binding_is_tui_zone(id: WorkbenchPaneBindingId, zone: WorkbenchZone) -> bool {
        crate::tui::workbench::tui_pane_bindings_for_zone(zone).any(|binding| binding.id == id)
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

    #[test]
    fn l0_mdi_applies_persisted_workbench_layout() {
        let layout = WorkbenchLayoutRecord::new(
            "tonight",
            WorkbenchId::Scores,
            WorkbenchPaneBindingId::FavoritesLeft,
            WorkbenchPaneBindingId::ScheduleRight,
            Some(WorkbenchExperienceId::TonightBench),
        )
        .expect("valid persisted layout");
        let mut mdi = MdiLayout::default();

        mdi.apply_persisted_layout(&layout).unwrap();

        assert_eq!(mdi.selected_workbench_id(), Some(WorkbenchId::Scores));
        assert_eq!(mdi.left_pane_binding, WorkbenchPaneBindingId::FavoritesLeft);
        assert_eq!(
            mdi.right_pane_binding,
            WorkbenchPaneBindingId::ScheduleRight
        );
        assert_eq!(
            mdi.active_experience,
            Some(WorkbenchExperienceId::TonightBench)
        );
    }

    // ── Phase Adams.4 — adaptive visibility ────────────────────────────────

    /// At ≥160 cols, all four regions visible.
    #[test]
    fn l0_adams_effective_panes_at_160_full_mdi() {
        let m = MdiLayout::default();
        let v = m.effective_panes(160);
        assert!(v.activity_catalog);
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
        assert!(v.activity_catalog);
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
        assert!(v.activity_catalog);
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
        assert!(v.activity_catalog);
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
            assert!(
                v.activity_catalog,
                "activity catalog must be true at width {width}"
            );
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

    #[test]
    fn l0_call_the_changes_mdi_default_focuses_workspace() {
        let m = MdiLayout::default();
        assert_eq!(m.focus, MdiFocus::Workspace);
        assert_eq!(m.catalog_selected, 0);
        assert_eq!(m.active_experience, None);
        assert_eq!(m.left_pane_binding, WorkbenchPaneBindingId::FavoritesLeft);
        assert_eq!(m.right_pane_binding, WorkbenchPaneBindingId::ScheduleRight);
        assert_eq!(
            m.left_pane_model(),
            Some(WorkbenchPaneModelId::FavoritesNavigator)
        );
        assert_eq!(
            m.right_pane_model(),
            Some(WorkbenchPaneModelId::ScheduleInspector)
        );
    }

    #[test]
    fn l0_call_the_changes_mdi_focus_cycles_visible_zones() {
        let mut m = MdiLayout::default();
        m.focus_next();
        assert_eq!(m.focus, MdiFocus::RightPane);
        m.focus_next();
        assert_eq!(m.focus, MdiFocus::ActivityRail);
        m.focus_next();
        assert_eq!(m.focus, MdiFocus::LeftPane);
        m.focus_prev();
        assert_eq!(m.focus, MdiFocus::ActivityRail);
    }

    #[test]
    fn l0_call_the_changes_mdi_focus_skips_hidden_panes() {
        let mut m = MdiLayout {
            show_favorites: false,
            show_schedule: false,
            ..Default::default()
        };
        m.focus = MdiFocus::Workspace;
        m.focus_next();
        assert_eq!(m.focus, MdiFocus::ActivityRail);
        m.focus_next();
        assert_eq!(m.focus, MdiFocus::Workspace);
    }

    #[test]
    fn l0_compose_the_bench_left_pane_cycles_tui_bindings() {
        let mut m = MdiLayout::default();

        let selected = m.cycle_left_pane(false).unwrap();

        assert_ne!(selected.id, WorkbenchPaneBindingId::FavoritesLeft);
        assert_eq!(selected.id, m.left_pane_binding);
        assert_eq!(m.left_pane_model(), Some(selected.pane_model));
    }

    #[test]
    fn l0_compose_the_bench_right_pane_cycles_tui_bindings() {
        let mut m = MdiLayout::default();

        let selected = m.cycle_right_pane(false).unwrap();

        assert_ne!(selected.id, WorkbenchPaneBindingId::ScheduleRight);
        assert_eq!(selected.id, m.right_pane_binding);
        assert_eq!(m.right_pane_model(), Some(selected.pane_model));
    }

    #[test]
    fn l0_compose_the_bench_experience_applies_bound_panes() {
        let mut m = MdiLayout {
            left_pane_binding: WorkbenchPaneBindingId::SavedQueriesLeft,
            right_pane_binding: WorkbenchPaneBindingId::DocsHelpRight,
            ..Default::default()
        };

        let experience =
            workbench_experience(WorkbenchExperienceId::TonightBench).expect("known experience");
        m.apply_experience(experience);

        assert_eq!(
            m.active_experience,
            Some(WorkbenchExperienceId::TonightBench)
        );
        assert_eq!(m.left_pane_binding, WorkbenchPaneBindingId::FavoritesLeft);
        assert_eq!(m.right_pane_binding, WorkbenchPaneBindingId::ScheduleRight);
    }

    #[test]
    fn l0_call_the_changes_catalog_selection_saturates() {
        let mut m = MdiLayout::default();
        m.select_prev_catalog_entry();
        assert_eq!(m.catalog_selected, 0);
        for _ in 0..WORKBENCH_CATALOG.len() + 5 {
            m.select_next_catalog_entry();
        }
        assert_eq!(m.catalog_selected, WORKBENCH_CATALOG.len() - 1);
    }
}
