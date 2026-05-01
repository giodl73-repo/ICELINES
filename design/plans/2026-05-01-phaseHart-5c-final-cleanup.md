# Phase Hart.5c — Final Cleanup Spec (v0.2, post-review)

**Status**: v0.2 — incorporates 4-role review (forge / glass / bench / tape).
Ready to implement.
**Date**: 2026-05-01
**Trophy**: Hart (final cleanup sub-phase)
**Predecessor**: design/plans/2026-04-30-phaseHart-normalization.md (master Hart
plan), design/plans/2026-04-30-phaseHart-4-1-test-foundation.md
**Replaces**: nothing — additive but is the closing phase

---

## v0.1 → v0.2 changelog

Four parallel reviews (forge, glass, bench, tape) returned eight blockers and
~14 fixits. Punch list below; the rest of the spec is updated inline.

**BLOCKERs resolved**:

- **Tape #1** — `eligible_pos: Vec<Position>` is silently dropped by the new
  model. Resolution: keep `SeasonStats.position: Position` singular; add an
  explicit policy section ("Multi-position eligibility policy") below.
  The live data path (`aggregate.rs:98`) already produced `vec![primary]`;
  the depth-chart spec documents this as a known limitation
  (`design/specs/depth-chart.md:197`). Any test fixture that populated a
  multi-element `eligible_pos` was simulating a feature that never shipped.
  Hart.5c codifies the existing reality.

- **Bench #1** — `tests/integration_phase2.rs` cannot be silently deleted.
  It carries 12 L1 tests including known-value assertions
  (Beniers 179.0 / 130.0 / 122.0 / 50.0 / 440.0 etc) for `PlayerFilter`
  and fantasy scoring. Resolution: rewrite in 5c.4 against
  `StatsRepository` + `PlayerView` fixture pattern, NOT delete in 5c.7.

- **Bench #2** — `tests/mock_nhl_api.rs` cannot be silently deleted. Only L1
  fence on `NhlApiClient` parsers (parse_boxscore, parse_playoff_bracket,
  schedule serde, goalie serde). Resolution: replace with
  `tests/mock_nhl_api_loader.rs` that exercises the same parsers through
  `load_into_repo`, including diacritic round-trip (Slafkovský).

- **Glass #1** — `Screen::Player(usize)`, `Screen::Comps(usize)`,
  `Screen::GoalieDetail(usize)` encode indexes into `app.players` /
  `app.goalies`. After App restructure, indexes become invalid (per-frame
  view collection has no stable order). Resolution: switch all Screen
  variants to `PlayerId` keying. `Screen::Player(PlayerId)` resolves the
  view per-frame via `repo.view(pid, season, season_type)`. Selection state
  in list screens stays as `usize` because it indexes into the per-frame
  sorted iterator.

- **Glass #2** — Season switch via `reload_for_season` invalidates
  `dashboard_panel` cache + `league_context` (rank tables, percentile
  tables computed on the previous repo). Resolution: `reload_for_season`
  calls `repo.repo_swap(new_repo)` THEN `dashboard_panel.clear_cache()`
  THEN rebuilds `league_context` from the new repo. Spec covers this in
  D2 with a code sketch.

- **Glass #3** — `loader.rs` (Hart.5a) currently returns
  `(Vec<Player>, Vec<Goalie>)` to the TUI App. After 5c.6 the App owns
  `StatsRepository`, so the loader must hand off a repo. Resolution:
  loader returns `LoadOutcome` already (Hart.4); App calls
  `load_into_repo(&mut self.repo, ...)` directly. Spec D2 updated.

- **Forge #1** — "Currently migrated (8 surfaces)" was misleading: those
  consumers passed a `flat_view_legacy` Player to a function written
  against Player. They aren't actually `PlayerView` consumers. Resolution:
  rewrite the section to honestly reflect status.

- **Glass #4 / Forge #8** — Hart.5c.6 also touches `app.goalies` (10 screens
  read it). Resolution: 5c.6 deliverables list goalies explicitly; final
  gate is "`grep -n 'app\.players\|app\.goalies' icelines-cli/src/tui/`
  returns zero hits".

**FIXITs applied inline**:

- Tape #2 — `DepthChartSlot` widened with `gp: Option<u32>` (terminal renderer
  reads it for `unplaced` and `below_min_gp`).
- Tape #3 — `build_views_with_swap` doc-comment mandates explicit hypothetical
  contract.
- Tape #4 — `to_scheme_stats_view` cold-start mapping documented:
  `view.hits().unwrap_or(0)` etc.; parity test required.
- Tape #5 — `query.rs::SortMetric` audit table added (Toi cast, None-cold-start
  policy, CfPct/FfPct/XgfPct legacy `unwrap_or(50.0)` median sentinel).
- Tape #6 — Adapter parity golden snapshot in 5c.1 before any consumer migration.
- Tape #7 — Goalie `games_played` mapping verification added to 5c.7 audit.
- Tape #8 — `mock_nhl_api_loader.rs` must assert diacritic round-trip.
- Forge #2 / Bench #3 — `PlayerFilter` migration is its own explicit step
  (Hart.5c.0); it's the upstream blocker for filter-dependent commands.
- Forge #6 — `player_from_view` is `fn` (private) not `pub fn`; corrected.
- Forge #7 — "static_assertions-style allow-deprecated" was sloppy: it's the
  8 `#[allow(deprecated)]` annotations referencing `flat_view_legacy*`.
- Bench #4 / Glass #5 — TUI L2 snapshot test using `ratatui::backend::TestBackend`
  is now a required deliverable for 5c.6, not a Risk #5 mitigation.
- Glass #6 — D4 sketches a goalie variant `fixtures::test_repo_with_goalie`.

---

## Goal

Land the deletions that the master Hart plan promised: legacy `Player`,
`Goalie`, `model::GoalieSeasonStats`, `flat_view_legacy`,
`flat_view_legacy_goalies`, `player_builder` module, and the L1 test files
that exercise the legacy build pipeline (which are REWRITTEN against
the new pipeline, not deleted, except where strictly redundant).

After Hart.5c the codebase has exactly one model: the normalized
`PlayerIdentity` + `SeasonStats` + `PlayerView` shape from Hart.1-4.
Every consumer reads the model through `PlayerView` accessors. No
shim, no dual-shape data structures.

## Why this needs a spec

Sessions 1-2 of Hart.5b migrated the simple consumers cleanly. Then I
hit architectural depth (`DepthChart` embeds `Player`, `App` stores
`Vec<Player>` long-lived, `render_report` tests build `Player` literals)
and started introducing tactical shims to keep grinding:

- `DepthChartBuilder::build_views` that converted views→Player internally
- `player_from_view` made `pub` to enable `tonight::run_trade`'s hypothetical
- Site builder using the same shim

Those commits (`d8703e93` Hart.5b2d/g + `48328b9c` Hart.5b2k) have been
reverted as of `8c3567ef` + `7bf84d8c`. The state is clean again but
architecturally we're stuck — the next consumer migration WILL introduce
the same shims unless we resolve the underlying questions. This spec
resolves them.

## Multi-position eligibility policy (new in v0.2)

`Player.eligible_pos: Vec<Position>` exists in the legacy model but is
populated as `vec![primary]` everywhere on the live data path
(`aggregate.rs:98`, `player_builder.rs:97`, `csv_loader.rs:102` parses the
raw "C,LW,Util" string but `aggregate.rs` discards it). The depth-chart
spec already calls this out as a known limitation
(`design/specs/depth-chart.md:197-202`).

**Decision**: Hart.5c keeps `SeasonStats.position` singular. Multi-position
eligibility is not a feature that ships in Hart. If/when Yahoo-CSV-driven
multi-position eligibility becomes a real product feature, it gets a new
field (`eligible_pos: Vec<Position>` or similar) on `SeasonStats` then,
with proper data-path wiring from `csv_loader::CsvRow.eligible_pos` (which
already preserves the raw string).

**Test fallout**: any unit test that built a `Player` literal with a
multi-element `eligible_pos` to exercise depth-chart spill behavior was
testing a feature the live path never supported. Those tests are deleted
(in 5c.1) along with the depth-chart spill code path that read
`eligible_pos.iter().filter(|p| p.is_forward())`. The greedy assignment
becomes "primary forward position only".

## Scope

In:
- `PlayerFilter` migration (Hart.5c.0 — upstream blocker)
- DepthChart redesign + every consumer of DepthChart (Hart.5c.1)
- `scouting::render_report` + its tests (Hart.5c.2)
- `query.rs` subcommands (Hart.5c.3)
- `fantasy.rs` + `to_scheme_stats_view` + scheme integration (Hart.5c.4)
  — also rewrites `tests/integration_phase2.rs`
- `export.rs` 5 markdown shape renderers (Hart.5c.5)
- TUI App restructure + 10 TUI screens (Hart.5c.6)
- Final delete of legacy types + flat_view_legacy* + player_builder
  (Hart.5c.7) — also writes `tests/mock_nhl_api_loader.rs`

Out:
- Anything not blocking Player/Goalie deletion
- New features
- TUI redesign beyond what the App refactor minimally requires
- Multi-position eligibility (deferred per policy section above)

## Status of consumers (corrected v0.2)

Per Forge #1: `flat_view_legacy` is a Player-shaped façade over PlayerView,
so consumers calling it are NOT PlayerView consumers — they're Player
consumers. Honest accounting:

**Already consume `PlayerView` directly** (no Hart.5c work needed):
- `commands/players.rs` — uses `PlayerRepository::all_views`-equivalent path
- `commands/project.rs` — uses `cross_team::compute_*_views`
- `commands/mates.rs` — uses `mates_view`
- `commands/analysis.rs::run_class / run_peers / run_compare / run_group::Show`
  — migrated in Hart.5b2f
- `cross_team` views API (`compute_cross_team_metrics_views`,
  `mates_view`, `peers_view`, `compare_view`, `group_show_view`)

**Currently consume Player via `flat_view_legacy`** (this is the Hart.5c surface):
- `commands/team.rs` (DepthChartBuilder::build)
- `commands/tonight.rs::run_trade` (DepthChartBuilder::build + manual Player
  clone)
- `commands/scouting.rs::render_report` (takes `&Player` + `&[Player]`)
- `commands/query.rs` (multiple subcommands consuming `&Player`,
  `PlayerFilter::apply` with `&Player`)
- `commands/fantasy.rs` (`to_scheme_stats(&Player)`)
- `commands/export.rs` (5 markdown shape renderers on `&Player`)
- `commands/rank.rs` (export path uses Player)
- `icelines-site/src/builder.rs` (consumes `&DepthChart` Player-embedded)
- All `tui/screens/*.rs` (10 files reading `app.players: Vec<Player>` /
  `app.goalies: Vec<Goalie>`)
- `icelines-fetch/src/player_builder.rs` (used by 2 L1 tests, deleted in 5c.7)

---

## Decisions to make

### D1 — DepthChart shape (Option B confirmed)

**Problem**: `DepthChart` currently embeds `Player` by value:
```rust
pub struct DepthChart {
    pub forward_lines: Vec<[Option<Player>; 3]>,
    pub defense_pairs: Vec<[Option<Player>; 2]>,
    pub unplaced: Vec<Player>,
    pub below_min_gp: Vec<Player>,
    ...
}
```

**Decision: Option B — `DepthChartSlot` value struct.** Lifetime-parameterized
`DepthChart<'a>` (Option A) would force `App<'a>` since the TUI holds a depth
chart across frames; the lifetime virality would infect every caller. Option C
(keep Player just for DepthChart) doesn't actually solve anything.

```rust
pub struct DepthChartSlot {
    pub player_id: PlayerId,                    // for metrics lookup
    pub full_name: String,                      // for display
    pub name_normalized: String,                // for fixture/test matching
    pub team: TeamAbbr,                         // for badge/logo (NB: D3)
    pub position: Position,                     // for layout
    pub pace_82: Option<f64>,                   // for color/rank
    pub goals_per_82: Option<f64>,              // for tooltips
    pub gp: Option<u32>,                        // tape #2 — render_team_card reads this
    pub headshot_canonical_url: Option<String>,
}

pub struct DepthChart {
    pub team: TeamAbbr,
    pub season: Season,
    pub forward_lines: Vec<[Option<DepthChartSlot>; 3]>,
    pub defense_pairs: Vec<[Option<DepthChartSlot>; 2]>,
    pub unplaced: Vec<DepthChartSlot>,
    pub below_min_gp: Vec<DepthChartSlot>,
}
```

**Renderer audit (confirmed needs)**:
- `render/terminal::render_team_card` — full_name, team, pos, **gp** for
  unplaced/below_min rows, **pace_82/82** as PPG
- `tui/screens/depth.rs` — same plus pace coloring
- `icelines-site/src/builder.rs::render_team_page` — name, team, pos,
  player_id (for cross_team_metrics lookup), headshot URL via
  `team_logo_url`/`player_cell` helpers
- `tonight::run_trade` formatters — full_name only

### D2 — TUI App restructure (with cache invalidation)

**Problem**: `App` stores `Vec<Player>` + `Vec<Goalie>` long-lived.
`PlayerView<'a>` borrows from a repo, can't be stored across frames.

**Decision**: `App` owns `StatsRepository` + `Season` + `SeasonType`.
Per-frame view collection in screens. Caches that survive across frames
(`dashboard_panel`, `league_context`) get explicit invalidation hooks.

```rust
pub struct App {
    repo: StatsRepository,
    active_season: Season,
    active_type: SeasonType,
    selected: usize,
    search_query: String,
    dashboard_panel: DashboardPanel,
    league_context: LeagueContext,
    // ... non-data UI state (tabs, modes, etc.)
}

impl App {
    pub fn views(&self) -> impl Iterator<Item = PlayerView<'_>> {
        self.repo.skaters(self.active_season, self.active_type)
    }
    pub fn goalie_views(&self) -> impl Iterator<Item = PlayerView<'_>> {
        self.repo.goalies(self.active_season, self.active_type)
    }

    pub async fn reload_for_season(&mut self, season: Season, ty: SeasonType)
        -> anyhow::Result<()>
    {
        // Glass #2 + #3: load + atomic swap + cache invalidation in one place.
        let mut new_repo = StatsRepository::new();
        load_into_repo(&mut new_repo, &snapshot_dir, season, ty).await?;
        self.repo.repo_swap(new_repo);  // borrow-checked atomic swap
        self.active_season = season;
        self.active_type = ty;
        self.dashboard_panel.clear_cache();           // glass #2
        self.league_context = LeagueContext::build(&self.repo, season, ty); // glass #2
        Ok(())
    }
}
```

**Screen variants — PlayerId-keyed (Glass #1)**:
```rust
pub enum Screen {
    Players,
    Player(PlayerId),       // was: Player(usize)
    Goalies,
    GoalieDetail(PlayerId), // was: GoalieDetail(usize)
    Comps(PlayerId),        // was: Comps(usize)
    DepthTeam(TeamAbbr),
    Schedule,
    // ... rest unchanged
}
```
List-screen `selected: usize` stays — it indexes into the per-frame
sorted iterator inside that screen's render, not into long-lived state.

**Selection-state contract**: when a list-screen press of `Enter` navigates
to `Screen::Player(pid)`, the screen's `handle_key` resolves the selected
view's `PlayerId` once at navigation time and stores that. The detail
screen looks up by ID per-frame. If the underlying player vanishes from
the repo (rare — only on season switch with extreme roster changes), the
detail screen renders an empty-state "player not found in current season"
panel and Esc returns to the list.

### D3 — `tonight::run_trade` hypothetical (with explicit contract doc)

**Decision**: `DepthChartBuilder::build_views_with_swap`:
```rust
/// Build a hypothetical depth chart for `team` after swapping `swap_out_id`
/// out and `swap_in` in. The returned chart's slot for `swap_in` will have
/// `team == <destination team>`, NOT `swap_in.team()` (which reports the
/// player's actual current team).
///
/// IMPORTANT: any downstream consumer that joins back to the repo via
/// `(player_id, team)` will mismatch on the swap-in slot, because the
/// slot's `team` field is the destination, not the player's real team.
/// Renderers that need ground-truth team membership must read from
/// `repo.view(slot.player_id, season, season_type).team()`, not from
/// `slot.team`.
///
/// (Tape #3.)
pub fn build_views_with_swap(
    team: TeamAbbr,
    season: Season,
    base_views: &[PlayerView<'_>],
    swap_in: PlayerView<'_>,
    swap_out_id: PlayerId,
) -> DepthChart { ... }
```

### D4 — `scouting::render_report` test fixtures (with goalie variant)

**Decision**: introduce `icelines-core::fixtures::test_repo_with` and a
goalie variant.

```rust
// Skater variant.
pub fn test_repo_with(
    identity: PlayerIdentity,
    stats: SeasonStats,
) -> StatsRepository {
    let mut r = StatsRepository::new();
    r.upsert_identity(identity).unwrap();
    r.upsert_stats(stats).unwrap();
    r
}

// Goalie variant (Glass #6).
pub fn test_repo_with_goalie(
    identity: PlayerIdentity,
    stats: SeasonStats,  // stats.goalie populated, stats.totals minimal
) -> StatsRepository {
    let mut r = StatsRepository::new();
    r.upsert_identity(identity).unwrap();
    r.upsert_stats(stats).unwrap();
    r
}
```

(The two variants are identical in body because `StatsRepository` is
shape-agnostic; the goalie-ness lives in `stats.goalie.is_some()`. The
helper exists as a pair so test-readers see intent at the call site, and
to give us a hook if we later need different invariants.)

Returns `(StatsRepository, PlayerId, Season, SeasonType)` from a per-test
helper closure:
```rust
fn fixture_repo_and_view() -> (StatsRepository, PlayerId, Season, SeasonType) {
    let id = fixtures::identity(8478402).build();
    let stats = fixtures::stats(8478402, 20242025, "EDM").build();
    let repo = fixtures::test_repo_with(id, stats);
    (repo, PlayerId(8478402), Season(20242025), SeasonType::Regular)
}
```

### D5 — `fantasy::to_scheme_stats` migration (with cold-start policy)

**Decision**: replace with `to_scheme_stats_view(&PlayerView<'_>)` private
to `fantasy.rs`. **Cold-start mapping** (Tape #4):

```rust
fn to_scheme_stats_view(view: &PlayerView<'_>) -> scheme::SkaterStats {
    let totals = view.season_totals();
    scheme::SkaterStats {
        goals: totals.goals,
        assists: totals.assists,
        // ...
        // Cold-start: Option<u32> realtime fields fall to 0 (preserves legacy
        // behavior — flat_view_legacy used `realtime.map(|r| r.hits).unwrap_or(0)`).
        hits:           view.hits().unwrap_or(0),
        blocked_shots:  view.blocked_shots().unwrap_or(0),
        takeaways:      view.takeaways().unwrap_or(0),
        giveaways:      view.giveaways().unwrap_or(0),
        // ...
    }
}
```

Required parity test in 5c.4 — same player through `to_scheme_stats_view`
on a cold-start (no realtime) repo asserts equal to legacy
`to_scheme_stats(&flat_view_legacy(view))` output. Adapter parity test
runs ONCE in 5c.1 against a synthetic golden, then both legacy and new
adapters produce that golden.

**Goalie analog**: `to_goalie_scheme_stats_view(&PlayerView<'_>)` reads
`view.stats.goalie.as_ref()`.

### D6 — Order of operations (8 sub-phases, was 7)

Each is a separate commit. L2 system tests gate every commit.

1. **Hart.5c.0** — `PlayerFilter` migration (NEW in v0.2). Migrate
   `PlayerFilter::apply` from `&[Player]` to `&[PlayerView<'_>]`. This is
   upstream of `query.rs`, `export.rs`, `fantasy.rs`. Keep
   `apply` (legacy) and `apply_views` side-by-side until 5c.7 deletes
   the legacy variant. Deliverable: `apply_views` exists, all
   `cross_team` view paths route through it; legacy `apply` callers
   unchanged.

2. **Hart.5c.1** — DepthChart redesign. Define `DepthChartSlot` with the
   field set above (incl. `gp: Option<u32>`). Refactor `DepthChart` to
   `Vec<[Option<DepthChartSlot>; 3]>` etc. Update
   `DepthChartBuilder::build` to produce slots from `&[PlayerView]`
   (rename to `build_views`; legacy Player-input `build` removed
   immediately because every depth-chart caller migrates in this commit).
   Add `build_views_with_swap` for `tonight::run_trade`. Migrate
   `render_team_card`, `tui/screens/depth.rs`,
   `icelines-site::render_team_page`, `team.rs`, `tonight::run_trade`,
   `icelines-site::build()`. **Adapter parity golden snapshot** (Tape #6):
   before any consumer migration in this commit, snapshot
   `player_from_view(known_view).hits` etc. and pin to a golden so
   subsequent commits can verify their migrated output matches the
   pre-migration baseline.

3. **Hart.5c.2** — `scouting::render_report` migration. Add
   `fixtures::test_repo_with` + `test_repo_with_goalie` helpers. Rewrite
   scouting tests against the new fixture pattern.

4. **Hart.5c.3** — `query.rs` migration. Subcommands (run_leaders,
   run_player, run_compare, run_similar) + internal helpers
   (leaders_table, print_current_stats, print_percentile,
   position_percentile, find_player, pace_strings, age_from_birth_date,
   draft_str). **`SortMetric` audit table** (Tape #5) embedded in
   commit message to prove every metric was reviewed for cast / None
   policy / sentinel preservation:

   | Metric | Legacy | View | Cast / policy |
   |---|---|---|---|
   | `Toi` | `f32` | `u32` | `as f64` cast — quantization-equal for whole-second values |
   | `Hits` | `u32` (zero on cold-start) | `Option<u32>` | `unwrap_or(0)` — None sorts to bottom of zero-tied cluster |
   | `Blocks` | `u32` | `Option<u32>` | `unwrap_or(0)` |
   | `Takeaways` | `u32` | `Option<u32>` | `unwrap_or(0)` |
   | `Giveaways` | `u32` | `Option<u32>` | `unwrap_or(0)` |
   | `CfPct` | `Option<f32>` w/ `unwrap_or(50.0)` | `Option<f32>` | `unwrap_or(50.0)` PRESERVED — median sentinel for cold-start |
   | `FfPct` | same | same | same |
   | `XgfPct` | same | same | same |
   | (other 22+ metrics) | unchanged shape | direct passthrough | — |

5. **Hart.5c.4** — `fantasy.rs` migration. `to_scheme_stats_view`,
   `to_goalie_scheme_stats_view`, all run_* commands. **REWRITE
   `tests/integration_phase2.rs`** (Bench #1) — rebuild the 12 L1 tests
   on `StatsRepository` + `PlayerView` fixture pattern, preserving
   known-value assertions (Beniers 179.0 / 130.0 / 122.0 / 50.0 / 440.0
   etc). **Cold-start parity test** (Tape #4) for
   `to_scheme_stats_view`.

6. **Hart.5c.5** — `export.rs` migration. 5 markdown shape renderers.
   Test mod's `eligible_pos: vec![pos]` literal disappears with the
   Player struct in 5c.7.

7. **Hart.5c.6** — TUI App restructure. Rewrite `App` to own
   `StatsRepository`. `reload_for_season` calls `load_into_repo` +
   `repo_swap` + cache invalidation (D2 sketch). Switch `Screen` variants
   to PlayerId keying (D2). Migrate all 10 TUI screens to use
   `app.views()` / `app.goalie_views()`. **Required deliverable**: L2
   TUI snapshot test at `icelines-cli/tests/tui_snapshot.rs` using
   `ratatui::backend::TestBackend` — render each top-level screen on a
   bundled-data fixture repo and assert frame-buffer matches a golden
   `.snap` file (Bench #4 + Glass #5). **Final gate** (Glass #4):

   ```bash
   $ Grep "app\.players|app\.goalies" icelines-cli/src/tui/
   # zero hits required to merge.
   ```

8. **Hart.5c.7** — Final delete + L1 replacement. Remove
   `flat_view_legacy`, `flat_view_legacy_goalies`, the 8
   `#[allow(deprecated)]` annotations referencing them (Forge #7),
   `Player`, `Goalie`, `model::GoalieSeasonStats`,
   `icelines-fetch::player_builder` module, the private
   `fn player_from_view` and `fn goalie_from_view` (Forge #6 — these
   are private helpers, not `pub`). Delete `tests/integration_pipeline.rs`
   (its only assertions are PlayerResolver / Cache round-trip already
   covered by `tests/stats_loader.rs`). **Replace** `tests/mock_nhl_api.rs`
   with `tests/mock_nhl_api_loader.rs` (Bench #2 + Tape #8) — exercises
   the same parsers (boxscore, playoff bracket, schedule serde, goalie
   serde) through `load_into_repo`, including diacritic round-trip
   assertion (Slafkovský). Also: **goalie `games_played` audit** (Tape
   #7) — verify `stats_repository.rs:894`'s `games_played: view.stats.totals.gp`
   sources from goalie GP not skater-pool GP; if it's wrong, fix in this
   commit.

**Each commit independently green.** L2 system tests gate every commit.
The test file changes are explicit replacement (5c.4: integration_phase2.rs
rewrite; 5c.7: mock_nhl_api.rs → mock_nhl_api_loader.rs); only
integration_pipeline.rs is fully deleted (its functionality is covered
elsewhere).

---

## Test impact (corrected v0.2)

| File | Change | Hart.5c sub-phase |
|---|---|---|
| `tests/transactions_fixture.rs` | unchanged | — |
| `tests/stats_loader.rs` | unchanged | — |
| `tests/integration_phase2.rs` | **REWRITTEN** against StatsRepository + PlayerView; preserves Beniers known-value asserts | 5c.4 |
| `tests/integration_pipeline.rs` | **DELETED** in 5c.7 — coverage redundant with stats_loader.rs | 5c.7 |
| `tests/mock_nhl_api.rs` | **REPLACED** by `tests/mock_nhl_api_loader.rs` in 5c.7 — same parsers via `load_into_repo` | 5c.7 |
| `tests/transactions_storage.rs` | unchanged | — |
| `tests/system_tests.rs` | unchanged (L2, runs binary) | — |
| `tests/proof_lib_smoke.rs` | unchanged | — |
| `tests/tui_snapshot.rs` | **NEW** (5c.6 deliverable) — ratatui TestBackend snapshots | 5c.6 |
| `commands/scouting.rs` test mod | rewritten in 5c.2 | 5c.2 |
| `commands/players.rs` test mod | unchanged (no Player literals) | — |
| `commands/query.rs` test mod | rewritten in 5c.3 if any Player literals | 5c.3 |
| `commands/fantasy.rs` test mod | rewritten in 5c.4 incl. cold-start parity | 5c.4 |
| `commands/export.rs` test mod | rewritten in 5c.5 | 5c.5 |
| `tui/screens/*.rs` test mods | rewritten in 5c.6 | 5c.6 |

---

## Risks (updated v0.2)

1. **`DepthChartSlot` field set is wrong**. Mitigation: tape #2 already
   widened with `gp`; renderer audit in D1 captures all known consumers.

2. **TUI App refactor breaks key handling.** Some screens may rely on
   `app.players[selected]` indexing in event handlers, not just render.
   Mitigation (Glass #1): Screen variants keyed on PlayerId means
   selection state is per-frame, not stale across season switch.

3. **`PlayerView` accessor surface might be incomplete**. We've added
   `is_rankable / pp_assists / per_82 helpers / pace_82 / pace_sort_key /
   toi_mmss / hits_per_82 / blocked_shots_per_82`. Audit in 5c.3 (query.rs)
   may surface needs; add accessors as needed in that commit.

4. **Test runtime regression** if every test now spins up a small
   StatsRepository. Mitigation: `fixtures::test_repo_with` is one-line
   and 1-2 microseconds; net wash.

5. ~~**L2 system tests cover end-to-end CLI behavior** but not per-screen
   TUI rendering.~~ **PROMOTED to required deliverable** (Bench #4 +
   Glass #5): `tests/tui_snapshot.rs` using `ratatui::backend::TestBackend`
   renders each screen on a bundled-data fixture repo and asserts
   frame-buffer == golden.

6. ~~**Hart.5c.7 deletion of integration_pipeline.rs / mock_nhl_api.rs
   loses test coverage**~~ **RESOLVED** (Bench #2 + Tape #8):
   `mock_nhl_api.rs` is replaced with `mock_nhl_api_loader.rs`;
   `integration_pipeline.rs`'s coverage is verified redundant with
   `stats_loader.rs` before deletion.

7. **`eligible_pos` policy enforcement**: any test that builds a
   multi-element `eligible_pos` is testing a feature that doesn't exist
   on the live path. The greedy depth-chart spill code that read
   `eligible_pos.iter().filter(|p| p.is_forward())` (depth_chart.rs:31)
   collapses to "primary forward position only" in 5c.1. Mitigation:
   document in commit message; multi-position spill behavior was already
   a known limitation.

8. **`load_into_repo` API signature lock-in** (Glass #3). The async
   `App::reload_for_season` path requires `load_into_repo` to take
   `&mut StatsRepository` and produce a `LoadOutcome`. This is what
   Hart.4 already shipped. No change needed; just confirm the call site
   in 5c.6 matches.

---

## What's NOT in this spec

- Hart.6 (playoff bundled data) — deferred per the master plan.
- Renames or naming convention changes.
- Multi-position eligibility (deferred per policy section above).
- API stability beyond the workspace (icelines-core is internal; no
  semver concerns).

## Next step

This is v0.2 — review punch list applied. Ready to implement.

1. User reads v0.2 + signs off OR pushes back on a specific decision.
2. Implement Hart.5c.0 (`PlayerFilter::apply_views`).
3. Forge / tape / bench review on the implementation diff per sub-phase.
4. Repeat for Hart.5c.1 through 5c.7.
5. Hart phase ships with `model.rs::Player` and `model.rs::Goalie`
   deleted. Single-model invariant achieved.
