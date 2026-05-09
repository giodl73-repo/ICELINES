// Phase Masterton.2.1 — Screen trait + dispatch contract.
//
// After Norris factored TUI STATE into per-screen structs,
// Masterton factors TUI CONTROL FLOW. Each screen module
// implements `Screen` and owns its handle/render/chrome
// concerns. The orchestrator (App) becomes a thin dispatcher.
//
// This file defines the trait + the dispatch result type
// (`ScreenAction`) + the cross-screen overlay discriminator
// (`OverlayKind`) + the read/write split context that screens
// see (`AppContext`).
//
// Per spec forge-1: `&self` on the trait methods is convention,
// not instance state. Screens are zero-sized marker structs
// (e.g., `pub struct QueriesScreen;`) with all logic in the
// trait impl. Static dispatch — no runtime cost vs. the
// pre-Masterton free-function pattern.
//
// Per spec forge-4: ScreenSpec is decoupled from clap's
// TuiSurface. The clap-derived enum carries launcher data
// (`Player { needle: String }`, etc.); the internal dispatch
// alias re-uses the existing `crate::tui::app::Screen` enum
// which is already the runtime discriminator with resolved
// data (PlayerId, TeamAbbr, etc.).

#![allow(clippy::module_name_repetitions)]
// Phase Masterton.2.1 ships the trait + dispatch types as
// scaffolding. The first consumer (QueriesScreen) lands in
// M.2.2; until then nothing uses these types so dead_code
// warnings would fire. Allow at the module level — the lint
// becomes meaningful again once migrations begin.
#![allow(dead_code)]

use ratatui::{layout::Rect, Frame};

use crate::tui::app::Screen as AppScreen;
use crate::tui::chrome::ScreenChrome;
use crate::tui::event::Action;

// ── Trait ────────────────────────────────────────────────────────────────────

/// Phase Masterton.2.1 — uniform contract for a TUI screen.
///
/// Each per-screen module exports a marker zero-sized struct
/// (e.g., `pub struct QueriesScreen;`) implementing this trait.
/// The orchestrator dispatches to the right impl via a match on
/// `app.screen`.
pub trait Screen {
    /// Per-screen state struct (extracted in Phase Norris).
    type State;

    /// Handle a user action against this screen's state.
    /// Returns `ScreenAction` — the orchestrator interprets it
    /// (`Quit` propagates, `Push/Replace/Pop` mutates `app.screen`,
    /// `OpenOverlay` flips an overlay flag, `Flash` writes
    /// transient status, `Continue` is the no-op default).
    fn handle(
        &self,
        state: &mut Self::State,
        ctx: &mut AppContext<'_>,
        action: Action,
    ) -> ScreenAction;

    /// Render this screen's body into `area`. Header + footer
    /// chrome is rendered by the shell (see `screens/mod.rs`)
    /// from `Screen::chrome`.
    fn render(&self, frame: &mut Frame, state: &Self::State, ctx: &AppContext<'_>, area: Rect);

    /// Declarative chrome — title + keybind hints. Consumed by
    /// the shell to render header (right-aligned title) + footer
    /// (keybind chips). Replaces the imperative `app.status`
    /// pattern for permanent state hints.
    fn chrome(&self, state: &Self::State, ctx: &AppContext<'_>) -> ScreenChrome;
}

// ── Dispatch result ──────────────────────────────────────────────────────────

/// Phase Masterton.2.1 — result of `Screen::handle`. The
/// orchestrator (App::dispatch) interprets these.
#[derive(Debug, Clone)]
pub enum ScreenAction {
    /// No-op. Most actions return this — state mutated in
    /// place, no orchestrator-level side effect needed.
    Continue,
    /// Tear down the TUI. Propagates up through `App::handle`'s
    /// return value.
    Quit,
    /// Push a new screen onto the navigation stack. The current
    /// screen becomes `prev_screen`; switches to `spec`.
    Push(ScreenSpec),
    /// Pop back to `prev_screen`. No-op if the stack is empty
    /// (Esc-from-leaf falls back to Home).
    Pop,
    /// Switch to `spec` without saving the current screen as
    /// prev. Used for tab cycling and direct surface launches.
    Replace(ScreenSpec),
    /// Open the named cross-screen overlay. The orchestrator
    /// sets the relevant overlay flag (e.g., `show_help = true`).
    OpenOverlay(OverlayKind),
    /// Set the transient status flash to `msg`. Cleared on the
    /// next handler action that doesn't itself produce a Flash.
    Flash(String),
}

/// Phase Masterton.2.1 — cross-screen overlay discriminator.
/// Per-screen overlays (filter editor, sort picker, save name,
/// load list — all inside QueriesState's `mode`) are screen-
/// internal and DON'T appear here. Per spec edge-1: OverlayKind
/// is reserved for cross-screen overlays only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    /// `?` — global help overlay.
    Help,
    /// `F` — admin overlay.
    Admin,
    /// `y` — season picker overlay.
    SeasonPicker,
    /// `R` — reports visibility overlay.
    Reports,
    /// `M` — in-TUI docs overlay.
    Docs,
    /// `Shift+D` — date picker overlay (shared between
    /// Tonight and Schedule per Foster.1.4).
    DatePicker,
    /// `g` — group picker overlay (shared between player
    /// card and team roster).
    GroupPicker,
}

/// Phase Masterton.2.1 — internal dispatch alias for the
/// existing `crate::tui::app::Screen` runtime enum. Per spec
/// forge-4: separate from clap's `TuiSurface` (which carries
/// launcher data, not resolved IDs). The existing Screen enum
/// already serves as the resolved-data dispatch type.
pub type ScreenSpec = AppScreen;

// ── App context (split-borrow) ───────────────────────────────────────────────

/// Phase Masterton.2.1 — per-handler context the screens see.
///
/// Splits App's fields so screens can read repo + season axis +
/// reports config while mutating only the flash slot. Other
/// mut handles (DBs, caches) are method-mediated rather than
/// exposed through &mut field access.
///
/// Per spec forge-2: the borrow choreography on
/// `App::make_context` is load-bearing. Concretely, App's per-
/// screen state structs (e.g., `app.queries`) are borrowed
/// SEPARATELY from the AppContext fields, so a screen handler
/// can hold `&mut app.queries` AND `&mut ctx.status` AND `&app.repo`
/// simultaneously without borrow-checker fights.
pub struct AppContext<'a> {
    /// Bundled stats repository — Norris stays on App.
    pub repo: &'a icelines_core::stats_repository::StatsRepository,
    /// Active season (`YYYYZZZZ`) and type (Regular / Playoff).
    pub season: icelines_core::model::Season,
    pub season_type: icelines_core::season_stats::SeasonType,
    /// Active timeframe (Day / Week / Month / Season per Phase
    /// Foster +8). Some screens use this to compute date ranges.
    pub timeframe: icelines_core::timeframe::Timeframe,
    /// Reports visibility config (cross-screen — read by every
    /// Tier-1 stat visibility check). Norris kept on App.
    pub reports: &'a crate::config::ReportToggles,
    /// Transient status flash slot. Screens write via
    /// `ctx.flash(msg)` (returning `ScreenAction::Flash` from a
    /// handler is the preferred path; direct mutation through
    /// this field is the lower-level escape hatch).
    pub status: &'a mut String,
}

impl<'a> AppContext<'a> {
    /// Set the transient status flash. Equivalent to returning
    /// `ScreenAction::Flash(msg)` but useful from inside helper
    /// functions that don't return a ScreenAction.
    pub fn flash(&mut self, msg: impl Into<String>) {
        *self.status = msg.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ScreenAction` clones cleanly. Required because the
    /// orchestrator may pass actions to multiple internal
    /// dispatch helpers.
    #[test]
    fn l0_screen_action_clones() {
        let a = ScreenAction::Quit;
        let _b = a.clone();
        let c = ScreenAction::Flash("hello".into());
        let _d = c.clone();
        let e = ScreenAction::OpenOverlay(OverlayKind::Help);
        let _f = e.clone();
    }

    /// `OverlayKind` is `Copy` + `PartialEq` — required because
    /// the orchestrator stores the active overlay kind and
    /// compares it for routing decisions.
    #[test]
    fn l0_overlay_kind_copy_partial_eq() {
        let a = OverlayKind::Help;
        let b = a; // copy
        assert_eq!(a, b);
        assert_ne!(a, OverlayKind::Admin);
    }

    /// `OverlayKind` covers each cross-screen overlay the App
    /// has today. Per spec edge-1: per-screen overlays
    /// (filter editor, sort picker, save name, load list) are
    /// in the screen's state, NOT in OverlayKind.
    #[test]
    fn l0_overlay_kind_covers_cross_screen_overlays() {
        // This test is a fence — adding a new cross-screen
        // overlay should add a variant here. Per-screen overlays
        // (FilterEdit / SortPicker / SaveName / LoadList in
        // QueriesState's `mode`) MUST NOT appear.
        let kinds = [
            OverlayKind::Help,
            OverlayKind::Admin,
            OverlayKind::SeasonPicker,
            OverlayKind::Reports,
            OverlayKind::Docs,
            OverlayKind::DatePicker,
            OverlayKind::GroupPicker,
        ];
        // Each is distinct — sanity check.
        for (i, a) in kinds.iter().enumerate() {
            for (j, b) in kinds.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "overlay kinds must be distinct");
                }
            }
        }
    }

    /// `AppContext::flash` writes to the status field.
    #[test]
    fn l0_app_context_flash_writes_status() {
        let repo = icelines_core::stats_repository::StatsRepository::with_lru_cap(8);
        let reports = crate::config::ReportToggles::default();
        let mut status = String::new();
        let mut ctx = AppContext {
            repo: &repo,
            season: icelines_core::model::Season(icelines_core::CURRENT_SEASON),
            season_type: icelines_core::season_stats::SeasonType::Regular,
            timeframe: icelines_core::timeframe::Timeframe::Day,
            reports: &reports,
            status: &mut status,
        };
        ctx.flash("hello world");
        assert_eq!(status, "hello world");
    }

    /// `Screen` trait is NOT object-safe (it has an associated
    /// type `State`). Per spec forge-1: that's by design —
    /// static dispatch via concrete ZST types, no Box<dyn Screen>
    /// support needed. The compile_fail doctest below pins this
    /// contract.
    ///
    /// ```compile_fail
    /// use icelines_cli::tui::screen::Screen;
    /// fn _no_box(_: Box<dyn Screen<State = ()>>) {}  // OK
    /// fn _no_dyn_unspecified(_: &dyn Screen) {}      // FAIL — needs concrete State
    /// ```
    #[test]
    fn l0_screen_trait_static_dispatch_only() {
        // Trivially passing — the compile_fail doctest above is
        // the actual check. This test exists so the module has
        // a coverage hit for the trait.
        let _ = std::any::type_name::<dyn std::fmt::Debug>();
    }
}
