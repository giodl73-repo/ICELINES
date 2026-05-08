# Phase Norris — Overview

**Trophy**: James Norris Memorial Trophy (best defenseman — anchors the
back end, foundational structural play)
**Version**: 1.0 (initial)
**Date**: 2026-05-07
**Status**: Spec — ready for implementation
**Plan**: `design/plans/2026-05-07-phaseNorris-tui-state-extraction.md`

---

## Vision in one paragraph

The `App` struct in `icelines-cli/src/tui/app.rs` is ~3,800 lines and
holds state for every TUI screen: Queries, Schedule, Transactions,
Goalies, Playoffs, Tonight, Reports, Search, Favorites, plus shared
cross-screen scaffolding. Adding a new screen means picking which of
the existing 80+ fields to wedge it next to; refactoring a single
screen risks rippling through every test that says
`app.query_*` or `app.schedule_*`; reading the struct definition is
exhausting. Phase Norris extracts per-screen state into its own
struct so each screen owns its own concerns. After Norris,
`App` keeps only the cross-screen coordinator state (`screen`,
`repo`, `active_season`, `status`, `prev_screen`, `selected`,
overlays, etc.) and each screen lives in its own module with its
own state struct, accessed as `app.queries.filter_text` instead of
`app.query_filter_text`. Tabs still cycle, keybinds don't change,
no UX delta — pure internal refactor.

## Locked decisions

| Decision | Choice | Rationale |
|---|---|---|
| Architectural pattern | Per-screen state struct as a field on App (not enum-of-state) | Preserves today's behavior: state persists across tab switches. Switching to enum-of-state would lose that and require re-loading on every tab cycle. |
| Phasing | One screen at a time, each ships as a self-contained commit | Bounded blast radius. ~50 test sites per screen; one-shot churn per screen keeps history clear. |
| State-struct naming | `<Screen>State` (e.g., `QueriesState`, `ScheduleState`) | Symmetric with existing `QueryField`, `QuerySection` etc. |
| Module home | Inline at the top of `tui/screens/<screen>.rs` | Keeps the state next to the renderer that serves it; no extra file per screen. |
| Migration discipline | Move all of a screen's fields in one commit; no gradual half-extracts | Avoids the "is this field still on App or on QueriesState?" cognitive load. |
| Test access pattern | Tests update field access (`app.query_filter_text` → `app.queries.filter_text`) in the same commit as the extraction | One-shot churn; reviewer reads one diff. |
| Cross-screen state stays on App | `screen`, `repo`, `status`, `prev_screen`, `selected`, `active_season`, `active_type`, overlays (`show_help`, `show_admin`, `show_season_picker`, `show_reports_overlay`, `group_picker_open`), `reports`, `clock`, sync state | These coordinate across screens; pulling them out forces dependency-injection complexity for marginal benefit. |
| Constructor | Each `<Screen>State` exposes `default()` (when every field has a `Default`) or `new()` (when one doesn't, e.g., needs CURRENT_SEASON). | Same pattern App already uses for nested fields like `query_fields = default_fields()`. |
| Derived traits | `#[derive(Debug)]` on every `<Screen>State`. No `Clone` or `PartialEq` (not needed by any consumer today; can add later if a use case appears). | `Debug` is required for panic-formatter consistency with today's `App` and for `dbg!()` debugging. — forge-1 |
| Visibility | `pub struct <Screen>State` with `pub` fields, matching `pub struct App`'s field visibility today. Restrict to `pub(crate)` only if a field needs to stay module-private; flag those individually. | Integration tests in `icelines-cli/tests/` go through subprocess, not direct field access, so external visibility doesn't widen the public API. — forge-2 |
| Clippy lint policy | Add `#![allow(clippy::module_name_repetitions)]` at the top of each `tui/screens/<screen>.rs` that gains a `<Screen>State`. | `QueriesState` inside `queries` module trips this lint; renaming each state struct (e.g., to `State`) loses the cross-module readability. The lint is a style preference, not a correctness signal. — forge-4 |
| Saved-query JSON contract | Unchanged. The serializer (`fields_and_filter_to_json`) keeps its signature; the v2 envelope shape (`{"version":2, "fields":[…], "filter_text":"…"}`) is preserved bit-for-bit. | Norris is a memory-layout refactor, not a persistence-format refactor. The on-disk SQLite schema and the JSON contract are off-limits. — wire-1 |
| Per-sub-phase test pattern | **L0 contract tests** (~10) on `<Screen>State::default()` — pin the starting-state contract so a future refactor can't silently change it. **L1 sequencing tests** (~3) chaining handler calls through realistic multi-step sessions to prove state transitions land correctly across actions. **No L2** — the TUI is interactive; subprocess can't drive keystrokes, so L2 is reserved for the CLI / web surfaces (already covered). | Confirmed during Norris.1 — see `screens/queries.rs::tests::l0_norris_*` (10 contract tests) and `screens/mod.rs::app_snapshot_tests::l1_norris_*` (3 sequencing tests). Pattern is the canonical bar for Norris.2/3/4. |
| No keybind / UX changes | Strictly an internal refactor. | The user's brief: "i want each screen to be separate but i guess i dont mind if its easy to go between them" — keep tab cycling, keep keybinds, just clean the layout. |

## Sub-phase ordering

```
Norris.1 ─── Queries (pilot, most state, ~17 fields)
                  │
                  └─→ Norris.2 ─── Schedule (~10 fields)
                                     │
                                     └─→ Norris.3 ─── Transactions (~8 fields)
                                                        │
                                                        └─→ Norris.4 ─── Smaller screens (batched)
                                                                          (Goalies + Playoffs + Tonight + Reports + Search)
```

Pilot first so the pattern is locked in before fanning out. Largest
screen first (Queries) so the worst test-churn case is the first one
we de-risk; subsequent screens are mechanical applications of the
proven pattern.

## Out of scope (deferred or handled separately)

- **Removing tab cycling / "dedicated mode"** — this was option 1
  from the user's initial framing, redirected away from. Tabs stay
  cycling.
- **Adding new TUI screens** (e.g., `tui career` for the Phase
  Calder cohort) — separate phase if pursued.
- **Adding `f` filter overlay to Goalies / Player(peers) /
  Compare(similar)** — separate phase. Norris doesn't change behavior.
- **Splitting `icelines` into multiple binaries** — out of scope
  per user framing; one binary stays.
- **Rewriting any screen's render logic** — only the state struct
  moves; render functions get their access updated to `app.<screen>.<field>`.
- **Migrating off ratatui** — still ratatui.
- **TUI framework abstractions** (e.g., per-screen "controller" trait
  with a uniform handle/render interface) — would force a larger
  rewrite. Norris stays minimal: state extraction only.

## Surface coverage matrix

Norris is a TUI-only refactor. CLI and web surfaces are unaffected.

| Capability | CLI | TUI | Web |
|---|---|---|---|
| Per-screen state | n/a | New: each screen owns its `<Screen>State` struct | n/a |
| Tab cycling | n/a | Unchanged — `Tab` / `Shift+Tab` still cycle through screens | n/a |
| Surface launchers | Unchanged — `icelines tui league`, `tui stats`, etc. still boot to their target screen | Unchanged | n/a |
| Saved queries / DB | Unchanged — `GroupDb` stays in the same place | n/a | n/a |
| Keybinds | n/a | Every keybind unchanged | n/a |

## Sub-phase summaries

### Norris.1 — Queries (pilot, ~1 day)

**Why first**: most state on App (17+ fields after Wave 23/24
landed), area we're most actively working in (test coverage is
freshest), and validating the pattern here de-risks the rest.

Extract `QueriesState` covering: `query_fields`, `query_field_idx`,
`query_sections`, `query_result_scroll`, `query_mode`,
`query_results_focused`, `query_save_name`, `query_saved_list`,
`query_filter_text`, `query_filter_error`, `query_filter_plan`,
`query_filter_history`, `query_filter_history_cursor`,
`query_filter_show_help`, `sort_picker_query`, `sort_picker_idx`,
`sort_stat_pick`, `career_table_preset`.

Test sites to update: every test in `tui::app::tests`, `tui::screens::queries::tests`, `tui::screens::app_snapshot_tests`, plus any
integration tests that touch `app.query_*` (none expected — no
external test file uses these private fields directly).

**Test budget** (Norris.1 actual, 2026-05-08): 13 new tests
across two tiers — 10 L0 contract tests on
`QueriesState::default()` (mode is Build, no active filter,
field editor populated, no saved state, overlays off, sort
picker unset, career_table_preset = Default, App::new wires
through ::default(), Debug derives) + 3 L1 sequencing tests
(history accumulates across 3 successive Enters; refinement
workflow re-applies extended filter; SaveName/FilterEdit/Build
mode transitions don't leak state across editors). Bin suite:
702/702 (was 692, +10 L0); L1 sequencing tests run inside the
existing `app_snapshot_tests` module.

### Norris.2 — Schedule (~0.5 day)

Extract `ScheduleState` covering: `schedule_query`,
`schedule_filter`, `schedule_filter_err`, `schedule_search_mode`,
`schedule_selected`, `schedule_week`, `schedule_week_cache`,
`schedule_team_cache`, plus any `schedule_*` fields added since.

**Test budget**: ~13 new tests per the Norris.1 pattern — 10 L0
contract tests on `<Screen>State::default()` + 3 L1 sequencing
tests covering multi-step handler sessions for this screen.

### Norris.3 — Transactions (~0.5 day)

Extract `TransactionsState` covering: `transactions`,
`transactions_fetched_at`, `transactions_stale`, `tx_selected`,
`tx_search_query`, `tx_search_active`, plus any `tx_*` fields.

**Test budget**: ~13 new tests per the Norris.1 pattern — 10 L0
contract tests on `<Screen>State::default()` + 3 L1 sequencing
tests covering multi-step handler sessions for this screen.

### Norris.4 — Smaller screens batched (~0.5 day)

Extract:
- `GoaliesState` (`goalie_selected`, `goalie_sort`, `goalie_min_gp`)
- `PlayoffsState` (`playoffs_cache`, `playoffs_round`, `playoffs_series`)
- `TonightState` (`scores_picker_open`, `scores_picker_input`)
- `SearchState` (`search_query`, `search_results`)
- `ReportsState` (`reports_selected` — note: `reports` config struct
  itself stays on App since it's read across screens for
  visibility-gating)

Each is small enough that bundling them into one commit is fine.

**Test budget**: ~13 new tests per the Norris.1 pattern — 10 L0
contract tests on `<Screen>State::default()` + 3 L1 sequencing
tests covering multi-step handler sessions for this screen.

## Total budget

- ~2.5 working days
- **~52 new tests** across the four sub-phases (13 per screen
  × 4 sub-phases ≈ 52). Pattern locked in Norris.1: 10 L0 contract
  tests on `<Screen>State::default()` + 3 L1 sequencing tests on
  multi-step handler sessions. No L2 — TUI is interactive, subprocess
  can't drive keystrokes (the CLI/web L2 surfaces already cover
  what's reachable that way).
- App struct goes from ~3,800 lines / 80+ fields → ~1,000-1,500
  lines / ~20-30 cross-screen fields
- Each sub-phase ships as its own commit; final commit cuts a
  v0.21.0 tag

## Pre-flight checklist

- [x] v0.20.3 shipped (Phase Art Ross polish + cohort filter); no
      pending TUI work outside Norris
- [x] Test suite is green (cargo test --workspace passes at HEAD)
- [x] Wave 23/24/24b/c/d (recent Queries-tab work) is settled —
      starting with Queries means landing on stable code
- [ ] Norris.1 starts (Queries extraction)

## Cross-cutting open items

1. **Borrow checker on cross-state mutations** — most handlers will
   continue to mutate via `&mut self` on `App`, accessing
   `self.queries.foo`. The handler layer doesn't change. The risk is
   cases where one mutation site needs both `&mut self.queries` and
   `&self.repo` simultaneously: today they live on the same struct so
   it's fine; post-Norris, the borrow checker may need a split-borrow
   helper. **Concrete mitigation**: lift the cross-state read into a
   local BEFORE the `&mut self.queries` mutation. Example pattern
   (forge-3):

   ```rust
   // Bad — fights the borrow checker:
   self.queries.filter_plan = Some(parse_query(/* needs */ &self.repo)?);

   // Good — read first, mutate second:
   let plan = parse_query(/* … */)?;
   self.queries.filter_plan = Some(plan);
   ```

   For more complex cases where the borrow truly must straddle two
   fields (rare), use a free function that takes both as separate
   parameters; don't try to pass `&mut App`.
2. **Tests using `App::new()` defaults** — some tests poke fields
   directly after `App::new(false)`. Post-Norris those become
   `app.queries.foo = …`. Mechanical search-replace, but high volume.
3. **The `App::new` init list** — currently a flat ~80-field
   initialization. Post-Norris each `<Screen>State::default()` fans
   out, so the init list shrinks but each state struct gets its own
   default impl. Net code stays the same; it's just relocated.
4. **No new `pub` exposure** — `<Screen>State` fields stay
   `pub(crate)` (or just `pub` within the crate; visibility matches
   today's `App` field visibility). Integration tests in
   `icelines-cli/tests/` don't touch these — they go through public
   surface (subprocess invocation). So visibility scope is unchanged.
5. **Where do test helpers live** — today `App` has scattered
   helpers like `App::query_views()` (returns `Vec<PlayerView>`).
   Post-Norris, do these stay on `App` or move to `QueriesState`?
   **Decision**: stay on `App` if they read `App::repo`; move to
   `QueriesState` only if they're pure functions of state. The repo
   stays on App.
6. **`career_table_preset` is a cross-screen dependency** — owned
   by `QueriesState` per Norris.1 but READ by `tui/screens/player.rs`'s
   render (the player card uses the preset to choose which career-
   stat columns to show). After Norris.1 this becomes
   `app.queries.career_table_preset` — awkward, since "the player
   card reaches into queries state." A future phase could introduce
   a `PlayerCardState` (or a shared `TableState` module) and migrate
   the preset there. Norris itself doesn't try to fix this; just
   flagging it. — glass-1
7. **Nav strip stays untouched** — `screens/mod.rs::render_nav`
   reads only `app.screen` (the discriminator) plus a couple of
   shared overlay flags. Confirmed by reading the function: it
   doesn't touch any screen-specific state. Norris extraction
   doesn't change `render_nav`. — glass-2
8. **Filter-plan eval site is a single rename** —
   `run_query_views_with_pick_and_plan(…, app.queries.filter_plan.as_ref(), …)`
   replaces the current `app.query_filter_plan.as_ref()`. There are
   exactly four call sites (3 in `app.rs` + 1 in `screens/queries.rs`
   per Wave 23 wiring). All mechanical; no semantic change. — edge-1
