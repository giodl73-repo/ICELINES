# Phase Norris — Plan

**Spec**: `design/specs/phase-norris-overview.md`
**Date**: 2026-05-07
**Status**: Plan — ready for execution
**Dependency**: v0.20.3 (clean tree, all tests green)

---

## Sub-phase ordering

```
Norris.1 ─── Queries (pilot)
Norris.2 ─── Schedule
Norris.3 ─── Transactions
Norris.4 ─── Smaller screens batched (Goalies + Playoffs + Tonight + Search + Reports)
Norris.5 ─── Closeout: docs + v0.21.0 tag
```

Each sub-phase is one git commit. Each commit must:
- compile cleanly (`cargo build -p icelines-cli`)
- pass the full bin test suite (`cargo test -p icelines-cli --bin icelines`)
- pass any L1/L2 integration suites that touched the screen
- not change any user-visible behavior (no keybind / render delta)

## Pre-flight

- [x] v0.20.3 tagged + pushed
- [x] `cargo test --workspace` green at HEAD (verified 2026-05-07)
- [x] Spec written (`phase-norris-overview.md`)
- [ ] Norris.1 begins

## Norris.1 — Queries extraction (pilot)

**Goal**: Move the 17+ queries-related fields off `App` into a new
`QueriesState` struct living in
`icelines-cli/src/tui/screens/queries.rs`.

### N.1.1 — Define `QueriesState`

In `tui/screens/queries.rs`, add at the top of the file (after the
existing `QueryField` / `QuerySection` definitions):

```rust
/// Phase Norris.1 — per-screen state struct for the Queries tab.
/// Replaces the 17+ `query_*` / `sort_picker_*` fields previously
/// scattered across `App`. Owned by App as `app.queries`.
pub struct QueriesState {
    pub fields: Vec<QueryField>,
    pub field_idx: usize,
    pub sections: Vec<QuerySection>,
    pub result_scroll: usize,
    pub mode: crate::tui::app::QueryMode,
    pub results_focused: bool,
    pub save_name: String,
    pub saved_list: Vec<(String, String)>,

    // Phase Art Ross (Wave 23-24d)
    pub filter_text: String,
    pub filter_error: Option<String>,
    pub filter_plan: Option<icelines_query::QueryPlan>,
    pub filter_history: std::collections::VecDeque<String>,
    pub filter_history_cursor: Option<usize>,
    pub filter_show_help: bool,

    // Phase Lindsay L.3.4 — sort picker
    pub sort_picker_query: String,
    pub sort_picker_idx: usize,
    pub sort_stat_pick: Option<icelines_core::stats_catalog::StatId>,

    // Phase Lindsay L.4 — career-table preset on the player card
    // (technically belongs to PlayerById screen but lives in App
    // today; investigate during the move whether it should stay here
    // or migrate to a future PlayerCardState).
    pub career_table_preset: crate::tui::screens::player::CareerTablePreset,
}

impl Default for QueriesState {
    fn default() -> Self {
        Self {
            fields: default_fields(),
            field_idx: 0,
            sections: default_sections(),
            result_scroll: 0,
            mode: crate::tui::app::QueryMode::Build,
            results_focused: false,
            save_name: String::new(),
            saved_list: Vec::new(),

            filter_text: String::new(),
            filter_error: None,
            filter_plan: None,
            filter_history: std::collections::VecDeque::new(),
            filter_history_cursor: None,
            filter_show_help: false,

            sort_picker_query: String::new(),
            sort_picker_idx: 0,
            sort_stat_pick: None,

            career_table_preset:
                crate::tui::screens::player::CareerTablePreset::Default,
        }
    }
}
```

**File**: `icelines-cli/src/tui/screens/queries.rs` (existing, append to top).

### N.1.2 — Add `pub queries: QueriesState` to App

In `tui/app.rs`:

1. Add the field (alongside the existing `screen` etc.):
   ```rust
   pub queries: crate::tui::screens::queries::QueriesState,
   ```

2. Remove the 17+ existing `query_*` / `sort_picker_*` fields:
   - `query_fields`
   - `query_field_idx`
   - `query_sections`
   - `query_result_scroll`
   - `query_mode`
   - `query_results_focused`
   - `query_save_name`
   - `query_saved_list`
   - `query_filter_text`
   - `query_filter_error`
   - `query_filter_plan`
   - `query_filter_history`
   - `query_filter_history_cursor`
   - `query_filter_show_help`
   - `sort_picker_query`
   - `sort_picker_idx`
   - `sort_stat_pick`
   - `career_table_preset`

3. In `App::new`, replace the 17 init lines with:
   ```rust
   queries: crate::tui::screens::queries::QueriesState::default(),
   ```

### N.1.3 — Mechanical rename across the codebase

Search-replace pass (NOT `replace_all` — review each hit):

| Old | New |
|---|---|
| `app.query_fields` | `app.queries.fields` |
| `self.query_fields` | `self.queries.fields` |
| `app.query_field_idx` | `app.queries.field_idx` |
| `self.query_field_idx` | `self.queries.field_idx` |
| `app.query_sections` | `app.queries.sections` |
| `self.query_sections` | `self.queries.sections` |
| `app.query_result_scroll` | `app.queries.result_scroll` |
| `self.query_result_scroll` | `self.queries.result_scroll` |
| `app.query_mode` | `app.queries.mode` |
| `self.query_mode` | `self.queries.mode` |
| `app.query_results_focused` | `app.queries.results_focused` |
| `self.query_results_focused` | `self.queries.results_focused` |
| `app.query_save_name` | `app.queries.save_name` |
| `self.query_save_name` | `self.queries.save_name` |
| `app.query_saved_list` | `app.queries.saved_list` |
| `self.query_saved_list` | `self.queries.saved_list` |
| `app.query_filter_text` | `app.queries.filter_text` |
| `self.query_filter_text` | `self.queries.filter_text` |
| `app.query_filter_error` | `app.queries.filter_error` |
| `self.query_filter_error` | `self.queries.filter_error` |
| `app.query_filter_plan` | `app.queries.filter_plan` |
| `self.query_filter_plan` | `self.queries.filter_plan` |
| `app.query_filter_history` | `app.queries.filter_history` |
| `self.query_filter_history` | `self.queries.filter_history` |
| `app.query_filter_history_cursor` | `app.queries.filter_history_cursor` |
| `self.query_filter_history_cursor` | `self.queries.filter_history_cursor` |
| `app.query_filter_show_help` | `app.queries.filter_show_help` |
| `self.query_filter_show_help` | `self.queries.filter_show_help` |
| `app.sort_picker_query` | `app.queries.sort_picker_query` |
| `self.sort_picker_query` | `self.queries.sort_picker_query` |
| `app.sort_picker_idx` | `app.queries.sort_picker_idx` |
| `self.sort_picker_idx` | `self.queries.sort_picker_idx` |
| `app.sort_stat_pick` | `app.queries.sort_stat_pick` |
| `self.sort_stat_pick` | `self.queries.sort_stat_pick` |
| `app.career_table_preset` | `app.queries.career_table_preset` |
| `self.career_table_preset` | `self.queries.career_table_preset` |

**Files affected** (estimated):
- `icelines-cli/src/tui/app.rs` — field definitions + init + ~150 access sites in handlers + tests
- `icelines-cli/src/tui/screens/queries.rs` — ~50 access sites (renderers + helpers + L0 tests)
- `icelines-cli/src/tui/screens/mod.rs` — ~10 access sites (L1 tests, render dispatcher)
- `icelines-cli/src/tui/screens/player.rs` — `career_table_preset` access (~5 sites)
- `icelines-cli/src/tui/screens/depth.rs` — none expected
- `icelines-cli/src/tui/screens/comps.rs` — none expected
- `icelines-cli/src/tui/screens/transactions.rs` — none expected
- Other screen files — none expected

**Tactical**:
- Use `Grep` to enumerate sites first, then iterate `Edit` per site.
- Run `cargo build -p icelines-cli` after every dozen edits to catch
  drift quickly.
- After all renames pass build, run `cargo test -p icelines-cli --bin icelines`.
  Expect 692 passing (same as v0.20.3).

### N.1.4 — Run the gauntlet

```
cargo build -p icelines-cli            # must compile clean
cargo test -p icelines-cli --bin icelines  # 692 passing, 0 failed
cargo test --release -p icelines-cli --test art_ross_w23_tui_filter
cargo test --release -p icelines-cli --test persona_wave23
```

If any test fails: investigate, fix, re-run. Don't ship a sub-phase
with red tests.

### N.1.5 — Commit

```
Phase Norris.1 — extract QueriesState (TUI architecture refactor)

App was a 3,800-line god-object holding state for every TUI screen.
Norris.1 extracts the 17+ Queries-tab fields into a QueriesState
struct living in tui/screens/queries.rs. App now reads
`app.queries.filter_text` instead of `app.query_filter_text` etc.

Pure internal refactor — no keybind change, no UX delta. ~200
access-site renames across app.rs / screens/queries.rs / screens/mod.rs
/ screens/player.rs. All existing tests pass unchanged.

Bin suite: 692/692 green (same as v0.20.3).
```

## Norris.2 — Schedule extraction

**Goal**: Same pattern, applied to Schedule.

### N.2.1 — Define `ScheduleState`

In `tui/screens/schedule.rs` (file already exists, append to top):

```rust
pub struct ScheduleState {
    pub query: String,
    pub filter: SearchFilter,            // already pub in this module
    pub filter_err: Option<String>,
    pub search_mode: bool,
    pub selected: usize,
    pub week: String,                    // ISO yyyy-mm-dd of Monday
    pub week_cache: WeekCache,           // already pub
    pub team_cache: TeamCache,           // already pub
}
```

(Resolve type imports as needed — many of these are already in
scope inside `schedule.rs`.)

### N.2.2 — Move + rename

Same pattern as N.1: add `pub schedule: ScheduleState` on App,
remove the 8 `schedule_*` fields, search-replace
`app.schedule_X` → `app.schedule.X`.

**Files**: app.rs, screens/mod.rs, screens/schedule.rs, possibly
others touching schedule state.

### N.2.3 — Test sweep + commit

Same gauntlet as N.1.5.

## Norris.3 — Transactions extraction

### N.3.1 — Define `TransactionsState`

In `tui/screens/transactions.rs`:

```rust
pub struct TransactionsState {
    pub rows: Vec<icelines_core::Transaction>,
    pub fetched_at: String,
    pub stale: bool,
    pub selected: usize,
    pub search_query: String,
    pub search_active: bool,
}
```

### N.3.2 — Move + rename + sweep + commit

Mirror Norris.1 / N.2 pattern.

## Norris.4 — Smaller screens batched

Single commit covering five small extractions. Each state struct
holds 2-4 fields; no individual one warrants a dedicated commit.

### N.4.1 — Define states

In their respective `tui/screens/<screen>.rs` files:

```rust
// goalies.rs
pub struct GoaliesState {
    pub selected: usize,
    pub sort: usize,
    pub min_gp: u32,
}

// playoffs.rs
pub struct PlayoffsState {
    pub cache: PlayoffsCache,
    pub round: usize,
    pub series: usize,
}

// misc.rs (or a new tonight.rs)
pub struct TonightState {
    pub picker_open: bool,
    pub picker_input: String,
}

// search.rs
pub struct SearchState {
    pub query: String,
    pub results: Vec<...>, // existing type
}

// (Reports overlay — `reports_selected` only;
//  the `reports` config struct stays on App.)
pub struct ReportsState {
    pub selected: usize,
}
```

### N.4.2 — Move + rename (all five at once) + sweep + commit

Same gauntlet. Single commit because each state struct is small
enough that bundling keeps reviewer effort proportional.

## Norris.5 — Closeout

### N.5.1 — Audit App struct

After Norris.4, `App` should hold only cross-screen state. Skim the
struct and verify nothing screen-specific snuck through. Flag any
fields whose home is unclear with a TODO comment for a follow-up.

### N.5.2 — Update CLAUDE.md

Add a bullet under "What's been built" noting that the TUI screen
state lives in per-screen `<Screen>State` structs (in case a future
session is editing the TUI).

### N.5.3 — Update CHANGELOG.md

```
## v0.21.0 — 2026-05-07 — Phase Norris (TUI architecture refactor)

Internal refactor only — no behavior change. App struct went from
3,800 lines / 80+ fields holding state for every TUI screen, to a
~1,000-line coordinator that delegates to per-screen state structs.

Each TUI screen now owns its state in a `<Screen>State` struct
living in its `tui/screens/<screen>.rs` module:

- QueriesState (Norris.1) — 17 fields
- ScheduleState (Norris.2) — ~10 fields
- TransactionsState (Norris.3) — ~8 fields
- GoaliesState / PlayoffsState / TonightState / SearchState /
  ReportsState (Norris.4) — 2-4 fields each

Migration discipline: tests update access patterns
(`app.query_filter_text` → `app.queries.filter_text`) in the same
commit as the extraction. ~50 mechanical rename sites per screen.

Bin suite size unchanged at 692 passing. No keybind change, no
UX delta, no surface change.
```

### N.5.4 — Cargo bump + tag

- `Cargo.toml`: 0.20.3 → 0.21.0 (minor bump because internal
  architecture changed enough to warrant a clean version boundary)
- `git commit -m "v0.21.0 — Phase Norris (TUI architecture refactor)"`
- `git tag -a v0.21.0`
- `git push origin master`
- `git push origin v0.21.0`

## Files added

| File | Purpose | Sub-phase |
|---|---|---|
| `design/specs/phase-norris-overview.md` | Spec | Pre-N.1 |
| `design/plans/2026-05-07-phaseNorris-tui-state-extraction.md` | Plan (this doc) | Pre-N.1 |

No new source files — every `<Screen>State` is appended to its
existing `tui/screens/<screen>.rs`.

## Acceptance for Phase Norris

- All four sub-phases ship as their own commits
- Bin suite size unchanged: 692 passing through the entire phase
- L1/L2 integration suites unchanged: art_ross_w23_tui_filter (12),
  persona_wave23 (14), persona_wave25 (10), and every other suite
  pass at every commit
- App struct < 1,500 lines after Norris.4
- No keybind change, no render change, no save-format change
- v0.21.0 tag cut on the closeout commit

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Test churn cascades and hides regressions | High | Per-screen extraction is isolated; full suite runs after every commit; mechanical rename uses Grep first to enumerate exhaustively |
| Borrow-checker fights with split state | Med | Keep handlers on `&mut self App`; the ownership boundary stays "App owns everything", just the field access path changes |
| `Default` impl can't be derived for state with non-Default field types | Med | Hand-write `default()` per state struct; mirrors the existing init pattern in `App::new` |
| `career_table_preset` migration ambiguity (it's TUI-wide but currently lumped with queries) | Low | Norris.1 keeps it on QueriesState; future PlayerCardState (out of scope here) can reclaim it |
| Public-API leak through pub fields on State structs | Low | `pub(crate)` only — integration tests in `icelines-cli/tests/` don't touch internals (they go through subprocess) |
| Re-running App::new() in tests creates state structs that don't match constructor expectations | Low | Each `<Screen>State::default()` mirrors the field-by-field init in App::new today |

## Out-of-plan items (deferred to post-Norris)

- **Removing tab cycling / "dedicated mode"** — option 1 from the
  original framing. If the user wants this, it's a separate phase
  ("Phase Lady Byng II" or similar UX-polish trophy reuse).
- **Adding `tui career` surface** — option 3 from the original
  framing. Same character; separate phase.
- **`f` filter overlay on Goalies / Player(peers) / Compare(similar)
  screens** — substantial UX additions; separate phase.
- **Per-screen "controller" trait abstraction** (uniform handle/render
  interface across screens) — would require a larger architectural
  pass; Norris stays minimal.
