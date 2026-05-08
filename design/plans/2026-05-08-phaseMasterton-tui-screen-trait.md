# Phase Masterton — Plan

**Spec**: `design/specs/phase-masterton-overview.md`
**Date**: 2026-05-08
**Status**: Plan — pending spec review approval
**Dependency**: v0.21.1 (Norris.6 closeout, clean tree, 763 tests green)

---

## Sub-phase ordering

```
Masterton.1 ─── Chrome trait + per-screen accessors
Masterton.2.1 ── Screen trait scaffold + AppContext + ScreenAction
Masterton.2.2 ── Migrate Queries → impl Screen
Masterton.2.3 ── Migrate Schedule → impl Screen
Masterton.2.4 ── Migrate Transactions → impl Screen
Masterton.2.5 ── Migrate Tonight → impl Screen
Masterton.2.6 ── Migrate Goalies → impl Screen
Masterton.2.7 ── Migrate Playoffs → impl Screen
Masterton.2.8 ── Sub-screens (PlayerById / Team / CompsById / GoalieDetailById / etc.)
Masterton.3 ──── Standalone runner
Masterton.4 ──── Closeout (CHANGELOG + CLAUDE.md + v0.22.0 tag)
```

Each numbered item ships as one commit. Each commit must:
- compile cleanly (`cargo build -p icelines-cli`)
- pass the full bin test suite at the same baseline (no
  regressions)
- not change any user-visible behavior (no keybind / render delta)

## Pre-flight

- [x] v0.21.1 tagged + pushed
- [x] `cargo test --workspace` green at HEAD (verified 2026-05-08)
- [x] Spec written (`phase-masterton-overview.md`)
- [ ] 8-role review applied (in progress)
- [ ] Masterton.1 begins

## Masterton.1 — Chrome trait

**Goal**: Each screen module declares its header title + keybind
hints via `ScreenChrome`. The shell renders both consistently
across screens. No behavior change in dispatch yet; this lays
the foundation for Masterton.2.

### M.1.1 — Define `ScreenChrome` and `KeyHint`

New file `tui/chrome.rs`:

```rust
#![allow(clippy::module_name_repetitions)]

//! Phase Masterton.1 — declarative TUI chrome contract.

#[derive(Debug, Clone)]
pub struct ScreenChrome {
    /// Screen title shown in the breadcrumb area of the header.
    /// e.g. "Stats / Queries — country=CAN AND age<25".
    pub title: String,
    /// Keybind hints rendered as chips in the footer.
    pub keybinds: Vec<KeyHint>,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyHint {
    pub key: &'static str,
    pub action: &'static str,
}

impl KeyHint {
    pub const fn new(key: &'static str, action: &'static str) -> Self {
        Self { key, action }
    }
}

/// Default keybinds always available — appended automatically by
/// the chrome renderer so screens don't have to repeat them.
pub const GLOBAL_KEYBINDS: &[KeyHint] = &[
    KeyHint::new("Tab", "next tab"),
    KeyHint::new("?", "help"),
    KeyHint::new("q", "quit"),
];
```

Wire into `tui/mod.rs` via `pub mod chrome;`.

### M.1.2 — Per-screen `chrome()` accessors

For each existing screen module, add a free function:

```rust
// tui/screens/queries.rs
pub fn chrome(state: &QueriesState, _app_ctx: &AppContext) -> ScreenChrome {
    let title = match &state.mode {
        QueryMode::Build => "Stats / Queries".to_owned(),
        QueryMode::FilterEdit => "Stats / Queries / Filter".to_owned(),
        QueryMode::SaveName => "Stats / Queries / Save".to_owned(),
        QueryMode::LoadList => "Stats / Queries / Load".to_owned(),
        QueryMode::SortPicker => "Stats / Queries / Sort".to_owned(),
    };
    let keybinds = match &state.mode {
        QueryMode::Build => vec![
            KeyHint::new("f", "filter"),
            KeyHint::new("/", "sort"),
            KeyHint::new("s", "save"),
            KeyHint::new("l", "load"),
            KeyHint::new("o", "toggle section"),
            KeyHint::new("←/→", "edit"),
            KeyHint::new("Space", "focus results"),
        ],
        QueryMode::FilterEdit => vec![
            KeyHint::new("Enter", "apply"),
            KeyHint::new("Esc", "cancel"),
            KeyHint::new("?", "grammar"),
            KeyHint::new("↑↓", "history"),
        ],
        // ... other modes
    };
    ScreenChrome { title, keybinds }
}
```

For Phase 1, `AppContext` is a stub — just `&App` for now. It
becomes a real type in Masterton.2.

Repeat for: schedule, transactions, tonight (misc), goalies,
playoffs.

### M.1.3 — Refactor `screens/mod.rs::render`

Replace the hardcoded `render_nav` + `app.status` footer with:

```rust
pub fn render(f: &mut Frame, app: &App) {
    let chrome = active_chrome(app);  // dispatches to per-screen chrome()
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // header (tabs + title)
            Constraint::Min(0),     // body
            Constraint::Length(2),  // footer (keybinds + transient flash)
        ])
        .split(f.area());

    render_header(f, app, &chrome, chunks[0]);
    render_body(f, app, chunks[1]);
    render_footer(f, app, &chrome, chunks[2]);
}
```

`render_header` shows the tab strip on row 1 and the screen
title on row 2 (right-aligned breadcrumb). `render_footer` shows
keybind chips on row 1 and the transient flash on row 2.

### M.1.4 — Migrate `app.status = "..."` sites to `flash`

Today's pattern:
```rust
self.status = "Saved query 'centerleaders'  ·  l=load  s=save".to_owned();
```

Becomes:
```rust
self.flash("Saved query 'centerleaders'");  // transient — chrome
                                            // shows the keybinds.
```

Where `flash(msg)` is a method on App that just sets
`self.status = msg.into()` for now — same field, different API.
Masterton.2 promotes flash to live on `AppContext`.

This is a per-screen mechanical pass; the `&self.status = ...`
lines that mix permanent + transient should be untangled
(permanent → declarative keybinds; transient → flash). Reviewer
should sanity-check no information is lost.

### M.1.5 — Tests

`tui/chrome.rs` gets ~6 L0 tests asserting:
- `ScreenChrome` clones cleanly
- `KeyHint::new` const fn
- `GLOBAL_KEYBINDS` invariants
- Default trait instances

Each per-screen `chrome()` accessor gets ~2 L0 tests:
- Default state yields a sensible chrome
- A non-default state (e.g., FilterEdit mode for Queries)
  yields the mode-specific chrome

~12 L0 tests total, in `tui::chrome::tests` and
`tui::screens::<screen>::masterton_chrome_tests`.

### M.1.6 — Gauntlet

```
cargo build -p icelines-cli
cargo test -p icelines-cli --bin icelines  # 763 + 12 = 775
cargo test --release -p icelines-cli --test art_ross_w23_tui_filter
cargo test --release -p icelines-cli --test persona_wave23
cargo clippy -p icelines-cli --no-deps -- -D warnings  # check
```

### M.1.7 — Commit

```
Phase Masterton.1 — declarative TUI chrome (header + footer)

Each screen module exports a `chrome()` accessor returning
ScreenChrome { title, keybinds }. The shell renders both
consistently across screens — header carries tabs + breadcrumb
title; footer carries keybind chips + transient flash slot.

No keybind change. ~12 L0 tests. Foundation for Masterton.2.
```

## Masterton.2 — Screen trait + per-screen migrations

### M.2.1 — Trait scaffold + AppContext + ScreenAction

New file `tui/screen.rs`:

```rust
//! Phase Masterton.2 — Screen trait + dispatch contract.

use crate::tui::chrome::ScreenChrome;
use crate::tui::event::Action;
use ratatui::{layout::Rect, Frame};

pub trait Screen {
    type State;

    fn handle(
        &self,
        state: &mut Self::State,
        ctx: &mut AppContext<'_>,
        action: Action,
    ) -> ScreenAction;

    fn render(
        &self,
        frame: &mut Frame,
        state: &Self::State,
        ctx: &AppContext<'_>,
        area: Rect,
    );

    fn chrome(
        &self,
        state: &Self::State,
        ctx: &AppContext<'_>,
    ) -> ScreenChrome;
}

#[derive(Debug, Clone)]
pub enum ScreenAction {
    Continue,
    Quit,
    Push(ScreenSpec),
    Pop,
    Replace(ScreenSpec),
    OpenOverlay(OverlayKind),
    Flash(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OverlayKind {
    Help,
    Admin,
    SeasonPicker,
    Reports,
    Docs,
    DatePicker,
    GroupPicker,
}

// ScreenSpec mirrors the existing crate::cli::TuiSurface mapping.
pub use crate::cli::TuiSurface as ScreenSpec;
```

`AppContext` lives in the same file (or `tui/app_context.rs`):

```rust
pub struct AppContext<'a> {
    pub repo: &'a icelines_core::stats_repository::StatsRepository,
    pub clock: &'a dyn icelines_core::freshness::Clock,
    pub season: icelines_core::model::Season,
    pub season_type: icelines_core::season_stats::SeasonType,
    pub timeframe: icelines_core::timeframe::Timeframe,
    pub reports: &'a crate::config::ReportToggles,
    pub status: &'a mut String,  // flash slot
    // ... other read-mostly handles screens need
}
```

App holds a `fn make_context(&mut self) -> AppContext<'_>` that
borrows the right fields.

### M.2.2 — Migrate Queries (pilot)

`tui/screens/queries.rs`:

```rust
pub struct QueriesScreen;

impl Screen for QueriesScreen {
    type State = QueriesState;

    fn handle(
        &self,
        state: &mut QueriesState,
        ctx: &mut AppContext<'_>,
        action: Action,
    ) -> ScreenAction {
        // Move the existing handler logic from App::handle's
        // Screen::Queries arm here. Mutations to QueriesState
        // are direct; flash messages return Flash(msg);
        // navigation returns Push/Pop/Replace.
        // ...
    }

    fn render(&self, frame: &mut Frame, state: &QueriesState, ctx: &AppContext<'_>, area: Rect) {
        // Existing render code, parameterized on state + ctx
        // instead of &App.
    }

    fn chrome(&self, state: &QueriesState, ctx: &AppContext<'_>) -> ScreenChrome {
        // Promotes the M.1.2 free fn to a trait method.
    }
}
```

App's `handle` Queries arm collapses to:
```rust
Screen::Queries => {
    let mut ctx = self.make_context();
    let action = QueriesScreen.handle(&mut self.queries, &mut ctx, action);
    self.dispatch(action);
}
```

`App::dispatch(action: ScreenAction)` is the new orchestrator
hub — it interprets ScreenAction (Quit propagates up,
Push/Pop/Replace mutate `self.screen`, OpenOverlay flips the
relevant overlay flag, Flash writes to status).

**Test continuity**: existing `app.handle(Action::Char('f'))`
calls still work because they go through `App::handle` which
internally routes to `QueriesScreen.handle`. Per-screen tests
that poke `app.queries.X` still work because the state struct is
unchanged.

The hard part: untangling cross-screen overlay handling. The
orchestrator runs FIRST and short-circuits if an overlay is
open; only then does it dispatch to the active screen. That logic
currently lives inside `App::handle` and needs lifting before the
per-screen migrations begin.

### M.2.3 — M.2.7 — Schedule / Transactions / Tonight / Goalies / Playoffs

Mirror M.2.2 for each. Each commits independently. Bin suite
must match the pre-migration baseline at every commit.

### M.2.8 — Sub-screens

Player card, team roster, comps, goalie detail, etc. Each gets
its own `impl Screen`. Cross-tab cycling skips them (Tab only
moves between the 9 main tabs).

### M.2.9 — App::handle shrinks

After all 7 migrations, `App::handle` is:

```rust
pub fn handle(&mut self, action: Action) -> bool {
    // 1. Overlay precedence — if any overlay is open, route here.
    if self.is_any_overlay_open() {
        return self.handle_overlay(action);
    }

    // 2. Global keybinds — Tab/Shift+Tab/Esc-from-leaf/etc.
    if let Some(handled) = self.handle_global(action) {
        return handled;
    }

    // 3. Active screen.
    let mut ctx = self.make_context();
    let screen_action = match self.screen {
        Screen::Queries => QueriesScreen.handle(&mut self.queries, &mut ctx, action),
        Screen::Schedule => ScheduleScreen.handle(&mut self.schedule, &mut ctx, action),
        // ... etc
    };
    self.dispatch(screen_action)
}
```

Target: ~600 lines (was ~2,000).

## Masterton.3 — Standalone runner

### M.3.1 — `tui::standalone::run`

```rust
pub fn run<S: Screen>(
    screen: S,
    initial_state: S::State,
    ctx: &mut AppContext<'_>,
) -> anyhow::Result<()> {
    // Set up terminal (enter raw mode, alt screen, etc.)
    let mut terminal = ratatui::Terminal::new(...)?;

    let mut state = initial_state;
    loop {
        // Render
        terminal.draw(|f| {
            let chunks = chrome_layout(f.area());
            render_header_no_tabs(f, chunks[0], &screen.chrome(&state, ctx));
            screen.render(f, &state, ctx, chunks[1]);
            render_footer(f, chunks[2], &screen.chrome(&state, ctx), ctx.status);
        })?;

        // Event
        let action = poll_action()?;
        let outcome = screen.handle(&mut state, ctx, action);
        match outcome {
            ScreenAction::Quit => break,
            ScreenAction::Flash(msg) => *ctx.status = msg,
            // Push/Pop/Replace are no-ops in standalone mode —
            // there's only one screen.
            _ => {}
        }
    }

    // Tear down terminal
    Ok(())
}
```

### M.3.2 — Surface-launcher wiring

Add `--standalone` flag to `Tui` clap subcommand. When set,
route through `standalone::run` instead of the multi-tab App.

### M.3.3 — Tests

L2 subprocess tests asserting `--standalone` boots cleanly for
each surface. ~5 tests.

## Masterton.4 — Closeout

### M.4.1 — Audit

Skim `App::handle` to confirm it's mostly orchestrator-only.
Flag any leftover screen-specific dispatch with TODOs.

### M.4.2 — CHANGELOG + CLAUDE.md

Add v0.22.0 entry summarizing the trait migration. CLAUDE.md
"What's been built" gets a Phase Masterton bullet.

### M.4.3 — Cargo bump + tag

- 0.21.1 → 0.22.0 (minor bump — internal architecture changed
  substantially)
- Commit, tag v0.22.0, push commits + tag.

## Acceptance for Phase Masterton

- All sub-phases ship as their own commits
- Bin suite grows by ~25 (12 chrome + 6 screen-trait L1 + 5 standalone L2 + 2 closeout sanity)
- All Phase Norris suites unchanged (763 baseline preserved at every commit; final count ~788)
- L1/L2 integration suites unchanged (art_ross_w23_tui_filter, persona_wave23, persona_wave25)
- `cargo clippy -p icelines-cli --no-deps` introduces no new lints
- Saved-query JSON contract unchanged (Norris contract preserved)
- App's `handle()` < 800 lines after Masterton.2.9 (was ~2,000)
- No keybind change, no render change, no save-format change
- v0.22.0 tag cut on the closeout commit

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Overlay-precedence dispatch bugs | High | Lift overlay handling out of App::handle FIRST (before per-screen migrations); test the orchestrator's overlay-first branching independently |
| AppContext borrow-checker fights | Med | Keep AppContext fields `&` where possible; only `&mut` what screens demonstrably need (status flash, anything they push to). Where it bites, lift cross-state reads into locals before the &mut borrow |
| Tab cycling regression | High | Tab/Shift+Tab handled by orchestrator, not screens. Test: `app.handle(Tab)` from each screen advances to the next, exactly as today |
| Test churn cascades | High | Each per-screen migration commit updates ONLY that screen's tests. Bin suite count must match pre-migration baseline EXACTLY at every commit; a drift means a test got accidentally dropped or doubled |
| Standalone runner desync from main runner | Med | Both runners delegate to the SAME `Screen::handle/render/chrome` methods. Drift caught by the L2 subprocess tests for `--standalone` |
| Trait-object overhead | Low | Screens are ZSTs (zero-sized marker structs); calls dispatch statically. No runtime cost vs. the current free-function pattern |
| Sub-screens (PlayerById etc.) under-covered | Med | M.2.8 fans out 5+ sub-screens. Each gets its own Screen impl; existing tests carry through |

## Out-of-plan items (deferred to post-Masterton)

- **Removing tab cycling** — option 1 from the original TUI
  factoring framing, redirected away from. Tabs still cycle.
- **Per-screen binaries** (separate crates / cargo bin targets) —
  out of scope; standalone runner is in-process.
- **Plugin architecture** for third-party screens — way out of
  scope.
- **Migrating overlays to the trait** — date_picker /
  group_picker / etc. stay imperative for now. Could be a future
  Masterton.5 if there's appetite.
- **Removing the App struct entirely** in favor of pure
  composition — would require pulling repo / cross-screen state
  out of a central owner; substantial restructuring; not
  warranted for the value.
