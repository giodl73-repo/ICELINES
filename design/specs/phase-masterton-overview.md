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
| Trait-method receiver | `&self` on every method, even though screens are ZSTs (zero-sized marker structs). | Idiomatic Rust; preserves object-safety should we ever want `Box<dyn Screen>`; zero runtime cost on a ZST. Spec calls this out so future readers know `&self` is convention, not a hint that screens carry instance state. — forge-1 |
| Screen stack depth | **One-deep** `prev_screen: Option<ScreenSpec>` on App, matching today's behavior. `ScreenAction::Push(spec)` replaces `prev_screen` with the current screen and switches to `spec`; `ScreenAction::Pop` walks back one level. | Adding a real `Vec<Screen>` stack would change Esc semantics (Esc walks all the way up vs. one level up), which is a UX change. Norris kept tabs cycling unchanged; Masterton should likewise preserve today's nav model. If a deeper stack is wanted later, it's an additive change. — forge-3 |
| `ScreenSpec` shape | **Define separately** from `crate::cli::TuiSurface`. The clap-derived `TuiSurface` carries surface-launcher data (`Player { needle: String }`, etc.); the internal dispatch enum needs only the screen identity, not clap-driven sub-data. Provide `From<TuiSurface> for ScreenSpec` for surface-launcher wiring. | Decouples internal dispatch from clap. Lets `ScreenAction::Push(ScreenSpec::PlayerById(pid))` carry resolved data instead of a `String` needle. — forge-4 |

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

**Test budget** (post-review bench-1/2):

- **Pre-migration regression fence per screen** (~6 tests, one
  L1 dispatch-smoke per screen). Captures the current
  key→state behavior across a representative action set
  (~10 keys per screen). The same test runs post-migration to
  confirm bit-for-bit equivalence. THESE LAND BEFORE ANY
  MIGRATION COMMIT — they're the fence.
- **ScreenAction return-shape coverage** (~10 tests):
  `Quit` propagates from any screen; `OpenOverlay(kind)` routes
  the right overlay flag; `Push/Replace/Pop` mutate `screen`
  correctly; `Flash(msg)` sets status without other side
  effects; `Continue` is the no-op default. Covers each
  ScreenAction variant × representative screen pair.

Total ~16 new tests across Masterton.2, with the regression
fence shipped in M.2.1 (the trait scaffold commit) BEFORE any
per-screen migration begins.

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

**Test budget** (post-review bench-3): ~5 L2 subprocess tests
asserting `--standalone --help` output for each surface (no
TTY needed; cheap to run in CI). A future polish item could
add a `--render-once` debug flag for golden-snapshot frame
testing, but it's out of scope for Masterton.3.

### Masterton.4 — Closeout

CHANGELOG entry, CLAUDE.md "What's been built" bullet,
Cargo.toml bump (0.21.1 → 0.22.0 — minor bump because the
internal architecture changed substantially), commit + tag
v0.22.0 + push.

## Total budget

- ~5-7 working days
- ~33 new tests across the phase (post-review bench-1/2/3):
  - 12 chrome tests (M.1)
  - 6 pre-migration regression fence tests (M.2.1, before per-screen migrations)
  - 10 ScreenAction return-shape tests (M.2)
  - 5 L2 standalone `--help` smoke tests (M.3)
- App struct stays at ~38 fields (Norris already won the
  field-count battle); the win here is **dispatch shrinkage**:
  `App::handle()` from ~2,000 lines → maybe ~600 (overlay
  handling + tab/escape/picker dispatch only).
- Each sub-phase ships as its own commit; final commit cuts v0.22.0

## Pre-flight checklist

- [x] v0.21.1 shipped (Phase Norris.6 complete)
- [x] Bin suite green at HEAD (763/763)
- [x] All 6 per-screen state structs landed (Queries, Schedule, Transactions, Goalies, Playoffs, Tonight) — Masterton.2 builds on top of these
- [x] Picker overlays factored (DatePickerState, GroupPickerState) — orchestrator can route around them cleanly
- [ ] Spec reviewed via 8-role pass
- [ ] Masterton.1 starts

## Cross-cutting open items

1. **AppContext mutability boundary + concrete construction** —
   what fields does the screen need `&mut` on vs `&`? First cut:
   `&` everything except `status`/flash and any DB handles. The
   borrow choreography on `App::make_context` is load-bearing
   (post-review forge-2). Concrete sketch:

   ```rust
   impl App {
       /// Splits App's fields so screens can read repo + clock
       /// while mutating only the flash slot. Other mut handles
       /// (e.g., favorites DB) are method-mediated rather than
       /// exposed through &mut field access.
       fn make_context(&mut self) -> AppContext<'_> {
           AppContext {
               repo: &self.repo,
               clock: &self.clock_for_screens,  // owned dyn Clock
               season: self.active_season_typed,
               season_type: self.active_type,
               timeframe: self.active_timeframe,
               reports: &self.reports,
               status: &mut self.status,  // ← only mut field
           }
       }
   }
   ```

   The trick: passing `&mut self` to `make_context` AND then
   calling a screen's `handle(&mut state, &mut ctx, action)`
   works because `state` (e.g., `self.queries`) is borrowed
   separately from the AppContext fields. This requires
   per-screen helpers like `App::queries_state_mut(&mut self)`
   that return `&mut self.queries` independently of
   `make_context()`. Where the borrow checker fights anyway,
   lift cross-state reads into locals before the &mut.
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
   call `app.handle()`. Post-Masterton, `app.handle()` STAYS as
   a thin orchestrator that internally routes to
   `screen.handle()`, so tests calling `app.handle(action)`
   continue to work without modification. Per-screen tests that
   poke `app.queries.X` also keep working (the state struct is
   unchanged). The migration is observable-equivalence-preserving
   by construction — but trust-but-verify (see bench-1 below).
7. **Behavior-equivalence regression fence** (bench-1) — the
   migration moves ~2,000 lines of dispatch logic. Bin suite
   carrying through doesn't *prove* parity unless the existing
   tests cover the dispatch sites being moved. Mitigation: BEFORE
   each per-screen migration, write a "dispatch smoke" L1 test
   that captures the screen's key→state behavior across a
   representative action set (~10 keys). Run the same test
   post-migration to confirm bit-for-bit equivalence. The
   pre-migration baseline serves as the regression fence; the
   post-migration green run is the equivalence proof.
8. **ScreenAction return-shape coverage** (bench-2) — at a
   minimum the L1 test budget should cover: `Quit` propagates
   from any screen, `OpenOverlay(kind)` routes to the right
   overlay flag, `Push/Replace/Pop` transition `screen` correctly,
   `Flash(msg)` sets status without other side effects, and
   `Continue` is the no-op default. ~10 tests covering each
   ScreenAction variant × representative screen pair.
9. **Standalone runner non-interactive trigger** (bench-3) — L2
   subprocess tests for `--standalone` will hang waiting for TTY
   input unless we add an exit trigger. Options: (a) add a
   `--render-once` flag that draws one frame and exits, useful
   for golden-snapshot testing; (b) stick to subprocess-with-
   timeout assertions on `--help` output for each surface; (c)
   spawn the binary, send a synthetic `q` to stdin, assert clean
   exit. (a) is the most useful for ongoing test infrastructure;
   (b) is the cheapest. Spec defers to (b) for Masterton.3 and
   leaves (a) as a future polish item.
10. **Header height + chrome budget** (glass-1) — target chrome
    is 1 row header + 1 row footer (was 2 + 1 pre-Masterton).
    Header carries the tab strip on the left and the screen title
    right-aligned on the same row when terminal is ≥120 cols; at
    narrower widths the title drops (tabs win). Footer renders
    keybind chips on the row, with the transient flash overlaid
    when present (replaces chips for the duration of the flash).
    No 2-row chrome — every row matters.
11. **Keybind chip overflow** (glass-2) — at narrow widths
    (`<80` cols), the rendered chip row would overflow.
    Strategy: chips render in declaration order; when the row
    is full, drop trailing chips with a `…` indicator. Each
    screen's `chrome.keybinds` should be ordered most-important
    first so truncation drops the least-useful keybinds. The
    `?` key (always-available help) is the explicit "see all
    keybinds" escape hatch — and as a global key, it's appended
    by the chrome renderer regardless of overflow.
12. **Standalone-mode chrome** (glass-3) — `tui::standalone::run`
    skips the tab strip entirely. Header carries only the screen
    title (1 row, left-aligned with a "← back" hint right-aligned
    on screens that support it). Footer is the screen's keybind
    chips + global keybinds (no Tab since there's no other
    screen to cycle to). Esc exits the standalone runner cleanly
    (returns to shell).
13. **AddToFavorites routing through the trait** (edge-1) — the
    filter overlay's `f` keybind comes through
    `Action::AddToFavorites` (intercepted in QueriesState::Build
    mode pre-Masterton). Post-Masterton, that intercept lives in
    `QueriesScreen::handle`. The orchestrator's first-pass
    overlay handling does NOT swallow `AddToFavorites` because
    AddToFavorites isn't an overlay-targeted action. So the flow
    is: orchestrator sees AddToFavorites with no overlay open →
    delegates to active screen → QueriesScreen.handle in Build
    mode intercepts and returns
    `ScreenAction::OpenOverlay(OverlayKind::FilterEditor)`...
    wait, that's a per-screen overlay (filter editor lives in
    QueriesState::mode), not a cross-screen one. So the screen
    just mutates its own state directly (returns Continue). The
    AddToFavorites-as-favorites-add action falls through to a
    different orchestrator branch (the favorites flow). Spec
    decision: filter-editor mode is per-screen (in QueriesState),
    so it's NOT in OverlayKind; OpenOverlay is reserved for
    cross-screen overlays only.
