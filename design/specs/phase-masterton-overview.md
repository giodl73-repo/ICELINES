# Phase Masterton — Overview

**Trophy**: Bill Masterton Memorial Trophy (perseverance, sportsmanship, dedication to hockey — long-term unglamorous infrastructure work)
**Version**: 1.0 (initial)
**Date**: 2026-05-08
**Status**: Spec — pending 8-role review
**Plan**: `design/plans/2026-05-08-phaseMasterton-tui-screen-trait.md`

---

## Vision in one paragraph

Phase Norris factored TUI **state** into per-screen structs. Phase
Masterton factors TUI **control flow** out of the same monolith.
Today `App::handle()` is one ~2,000-line match that knows every
keybind for every screen, and `render_nav` is hardcoded around the
multi-tab layout. After Masterton each screen module implements a
uniform `Screen` trait owning three concerns — state, render,
handle — and a `ScreenChrome { title, keybinds }` declarative
contract. The App layer becomes a thin orchestrator that dispatches
to the active screen. Two payoffs: (1) any single screen can be
hosted standalone via a simple runner (`icelines tui leaders
--standalone` boots only that screen with no tab strip and no
cross-screen plumbing), and (2) every screen's header/footer is
consistent because the shell renders it from the trait, not from
ad-hoc imperative status strings.

## Locked decisions

| Decision | Choice | Rationale |
|---|---|---|
| Architectural pattern | **Trait-based screen controllers** (`pub trait Screen { type State; fn handle(...); fn render(...); fn chrome(...); }`), not enum-of-state | Lets each screen evolve at its own pace; preserves cross-tab state continuity (today's behavior); fits the Norris.1-6 per-screen-module layout. |
| App role | **Stays as orchestrator** — owns `repo`, `screen` discriminator, cross-screen overlay state (`date_picker`, `group_picker`, `show_help`, `show_admin`, `show_reports_overlay`, `show_season_picker`, `show_docs`), `reports` config, time/season axis, status string. | The trait is the boundary, not full module isolation. App provides an `AppContext` (or similar) that screens read through; screens never reach into each other. |
| Handler return | **`ScreenAction` enum**: `Continue`, `Quit`, `Push(ScreenSpec)`, `Pop`, `Replace(ScreenSpec)`, `OpenOverlay(OverlayKind)`. Returned by `Screen::handle()`. | Explicit dispatch result (rather than the screen mutating App through `&mut self`). The orchestrator decides what to do with the action. Push/Pop semantics give us a screen stack for free. |
| Chrome | **Declarative** — `ScreenChrome { title, keybinds }` returned from `Screen::chrome(&state, &app_ctx)`. `keybinds: Vec<(&'static str, &'static str)>` like `[("f", "filter"), ("s", "save"), ("/", "sort")]`. | Replaces the imperative `app.status = "Queries · p:projections  ←/→:edit  Tab:focus results"` pattern. The shell renders header (tabs + title) and footer (keybind chips) uniformly across screens. Transient status messages still possible via `AppContext::flash(msg)`. |
| Migration shape | **Screen-by-screen, behind a feature flag is overkill**: refactor each screen to `impl Screen`, ship one screen per commit, App's giant match shrinks one branch at a time. | Mirrors the Norris.1-4 incremental pattern. Each commit must pass the full bin suite at the same baseline (no regressions). Test coverage carries through. |
| Standalone runner | **Thin wrapper** — `tui::standalone::run<S: Screen>(initial_state)`. Builds a minimal AppContext (repo, clock, status), runs the event loop against just that one screen, no tab strip. | Useful for surface launchers that want a focused experience and for testing a screen in isolation outside the full TUI. |
| Tab cycling | **Stays via `ScreenAction::Replace`** in the orchestrator. The orchestrator handles `Action::Tab` / `Action::TabPrev` itself (not delegated to the active screen) and dispatches via Replace to advance the screen. | Tab is a global concern, not a per-screen concern. Today's behavior preserved. |
| Cross-screen overlays | **Owned by the orchestrator** — `date_picker`, `group_picker`, `show_help`, `show_admin`, `show_reports_overlay`, `show_season_picker`, `show_docs`. The orchestrator's `handle()` runs FIRST; if any overlay is open the orchestrator handles the action. Only when no overlay is active does the screen's `handle()` run. | Clean priority: overlays first, screen second. Avoids forcing every screen to know about every overlay. |
| `AppContext` shape | A struct holding `&StatsRepository`, `&dyn Clock`, `Season`, `SeasonType`, `Timeframe`, `&ReportToggles`, `&mut Status`, plus mutable handles for things like favorites DB. | Screens read through this; screens never see `&mut App` directly. Forces a clean dependency boundary. |
| Status flash channel | `AppContext::flash(msg: impl Into<String>)` writes to a transient status slot that the chrome footer renders below the keybind chips. Cleared on next screen action. | Replaces today's `self.status = "..."` pattern; transient messages still work but they don't compete with declarative keybinds. |

## Sub-phase ordering

```
Masterton.1 ─── Chrome trait (~1 day)
                       │
                       └─→ Masterton.2 ─── Screen trait + per-screen migrations
                                              ~3-5 days, 6 screens
                                              │
                                              └─→ Masterton.3 ─── Standalone runner (~1 day)
                                                                    │
                                                                    └─→ Masterton.4 ─── Closeout (CHANGELOG + v0.22.0 tag)
```

Phase 1 is foundation — declarative chrome metadata, no behavior
change. Phase 2 is the heavyweight — every screen migrates.
Phase 3 is small after Phase 2 (the trait makes standalone
trivial). Phase 4 cuts the version.

## Out of scope

- **Renaming surface launchers** — `icelines tui leaders` /
  `tui stats` etc. stay as today.
- **Splitting `icelines` into multiple binaries** — single binary
  stays. Standalone runner is in-process.
- **Rewriting any screen's rendering logic** — only the
  trait-impl boundary moves; the actual render code stays the
  same shape, just relocated.
- **Migrating off ratatui** — still ratatui.
- **Removing tab cycling** — Tab/Shift+Tab stay as today,
  handled by the orchestrator.
- **Touching the cross-screen overlays** (date picker, group
  picker, season picker, reports, help, admin, docs) — they keep
  their pre-Masterton form. The orchestrator routes around them.
- **Migrating saved-query JSON** — Norris contract preserved.
  No persistence schema change.
- **Adding new keybinds or new screens** — Masterton is purely
  structural.

## Surface coverage matrix

Masterton is a TUI-only refactor. CLI and web surfaces are
unaffected.

| Capability | CLI | TUI | Web |
|---|---|---|---|
| Per-screen `impl Screen` | n/a | New: each screen module gets a `Screen` impl | n/a |
| Chrome contract (`ScreenChrome`) | n/a | New: declarative title + keybinds | n/a |
| Standalone runner | n/a | New: `tui::standalone::run<S: Screen>` | n/a |
| Surface launchers | Unchanged — `icelines tui league`, `tui stats`, etc. work as today | Unchanged | n/a |
| Tab cycling | n/a | Unchanged — orchestrator handles Tab | n/a |
| Keybinds | n/a | Every keybind unchanged (declarative metadata, same key → same action) | n/a |

## Sub-phase summaries

### Masterton.1 — Chrome trait (~1 day)

`tui::chrome::ScreenChrome { title: String, keybinds: Vec<KeyHint> }`
in a new `tui/chrome.rs` module. Each screen module exports
`pub fn chrome(state: &Self::State, app_ctx: &AppContext) -> ScreenChrome`
as a free function (Phase 1 doesn't yet require the full
`Screen` trait — just the chrome accessor).

`screens/mod.rs::render` looks up the active screen's chrome via
a match (mirrors today's screen dispatch) and renders:
- **Header** (top row): tabs + season/playoff indicators + the
  screen's `chrome.title` as a breadcrumb.
- **Footer** (bottom row): keybind chips from `chrome.keybinds`,
  with the transient status slot beneath if non-empty.

The status string today (`app.status`) becomes the transient flash
slot; permanent keybind hints move to declarative `keybinds`.

**Test budget**: ~12 tests — 2 per screen × 6 screens, asserting
the chrome accessor returns sensible defaults and reflects state
changes (e.g., FilterEdit mode flips to a different keybind set).

### Masterton.2 — Screen trait + per-screen migrations (~3-5 days)

```rust
pub trait Screen {
    type State;

    fn handle(&self, state: &mut Self::State, ctx: &mut AppContext, action: Action) -> ScreenAction;
    fn render(&self, frame: &mut Frame, state: &Self::State, ctx: &AppContext, area: Rect);
    fn chrome(&self, state: &Self::State, ctx: &AppContext) -> ScreenChrome;
}

pub enum ScreenAction {
    Continue,
    Quit,
    Push(ScreenSpec),
    Pop,
    Replace(ScreenSpec),
    OpenOverlay(OverlayKind),
    Flash(String),  // sets transient status, no screen change
}
```

The trait lives in `tui::screen` (new module). Each screen module
adds a marker zero-sized struct (e.g., `pub struct QueriesScreen;`)
that implements the trait. The screen's existing free
functions (`handle_*`, `render`) become trait methods.

Per-screen migration order (largest first to de-risk early):

1. Queries (most state, freshest test coverage post-Norris)
2. Schedule
3. Transactions
4. Tonight (Scores)
5. Goalies
6. Playoffs

Each migration is one commit:
- Commit N: `Phase Masterton.2.<screen> — migrate <Screen> to Screen trait`
- App's `handle()` match arm for that screen collapses to
  `<Screen>::handle(&mut state, &mut ctx, action)`
- Bin suite must match the pre-migration baseline exactly

After all 6, `App::handle()` is mostly a thin dispatcher to the
active screen + overlay handling.

**Test budget**: 0 new tests for the migration itself (existing
tests carry through), + ~6 L1 tests asserting `ScreenAction`
return shapes for representative actions per screen (so future
refactors can't accidentally break the dispatch contract).

### Masterton.3 — Standalone runner (~1 day)

```rust
pub fn run<S: Screen>(
    screen: S,
    initial_state: S::State,
    ctx: &mut AppContext,
) -> Result<(), Box<dyn std::error::Error>>
```

Lives in `tui::standalone`. Sets up the terminal, runs an event
loop, dispatches to `Screen::handle` and `Screen::render` directly,
no tab strip. Returns on `ScreenAction::Quit` or Esc-from-leaf.

Wire surface launchers like `icelines tui leaders --standalone`
that route through this runner instead of the multi-tab App.

Optional polish: when `--standalone` is omitted, today's
multi-tab experience continues. The flag is purely additive.

**Test budget**: ~5 L2 subprocess tests asserting `--standalone`
boots cleanly for each surface that supports it.

### Masterton.4 — Closeout

CHANGELOG entry, CLAUDE.md "What's been built" bullet,
Cargo.toml bump (0.21.1 → 0.22.0 — minor bump because the
internal architecture changed substantially), commit + tag
v0.22.0 + push.

## Total budget

- ~5-7 working days
- ~25 new tests across the phase (12 chrome + 6 trait L1 + 5 L2 standalone + 2 closeout sanity)
- App struct stays at ~38 fields (Norris already won the field-count battle); the win here is **dispatch shrinkage**: `App::handle()` from ~2,000 lines → maybe ~600 (overlay handling + tab/escape/picker dispatch only).
- Each sub-phase ships as its own commit; final commit cuts v0.22.0

## Pre-flight checklist

- [x] v0.21.1 shipped (Phase Norris.6 complete)
- [x] Bin suite green at HEAD (763/763)
- [x] All 6 per-screen state structs landed (Queries, Schedule, Transactions, Goalies, Playoffs, Tonight) — Masterton.2 builds on top of these
- [x] Picker overlays factored (DatePickerState, GroupPickerState) — orchestrator can route around them cleanly
- [ ] Spec reviewed via 8-role pass
- [ ] Masterton.1 starts

## Cross-cutting open items

1. **AppContext mutability boundary** — what fields does the
   screen need `&mut` on vs `&`? First cut: `&` everything except
   `status`/flash and any DB handles. If a screen needs to mutate
   the repo, it goes through a method on `AppContext` (e.g.,
   `ctx.refresh_repo()`), not direct field access. Reviewer
   should sanity-check the read/write split per screen.
2. **Overlay precedence** — orchestrator runs `handle()` first;
   if any overlay is open it eats the action. Decision: which
   overlays count? The cross-screen ones (date_picker,
   group_picker, season_picker, reports, help, admin, docs).
   Per-screen overlays (filter editor, sort picker, save name,
   load list — all in QueriesState's `mode`) stay screen-internal
   because they're tied to that screen's state.
3. **Sub-screens** — Player card (`Screen::PlayerById`), team
   roster (`Screen::Team`), comps (`Screen::CompsById`) etc. Are
   these "screens" in the trait sense, or sub-views of their
   parent screens? Decision: each gets its own `impl Screen`
   because they have distinct keybinds and distinct chrome. Tab
   cycling skips them (only the 9 main tabs cycle).
4. **Default keybinds in chrome** — every screen has access to
   `?` (help), `q` (quit), `Tab` (cycle). These are
   orchestrator-level keys. Should they appear in the screen's
   `keybinds` list or be appended automatically by the chrome
   renderer? Decision: appended automatically — keeps each screen's
   declarative keybinds focused on screen-specific actions.
5. **Flash vs. permanent status** — today's `app.status` mixes
   "current state" (e.g., "Goalies sort: SV%") with transient
   feedback (e.g., "Saved query 'centerleaders'"). Post-Masterton:
   declarative keybinds replace permanent state messaging; flash
   replaces transient feedback. Reviewer should sanity-check that
   no information is lost.
6. **Test continuity** — existing L0/L1 tests poke `app.X` and
   call `app.handle()`. Post-Masterton, the test pattern shifts
   to `screen.handle(&mut state, &mut ctx, action)`. We can write
   a thin `App::handle_for_test()` wrapper that preserves the
   old shape so existing tests don't churn — but at the cost of
   keeping the old API alive. Decision: rip the bandaid; tests
   migrate alongside their screens. Each per-screen migration
   commit updates its own tests.
