# Phase Hart.5c — Final Cleanup Spec (v0.1, pre-review)

**Status**: Draft v0.1 — needs forge / glass / bench / tape review
**Date**: 2026-05-01
**Trophy**: Hart (final cleanup sub-phase)
**Predecessor**: design/plans/2026-04-30-phaseHart-normalization.md (the
master Hart plan), design/plans/2026-04-30-phaseHart-4-1-test-foundation.md
**Replaces**: nothing — additive but is the closing phase

---

## Goal

Land the deletions that the master Hart plan promised: legacy `Player`,
`Goalie`, `model::GoalieSeasonStats`, `flat_view_legacy`,
`flat_view_legacy_goalies`, `player_builder` module, and the two L1
test files that exercise the legacy build pipeline
(`integration_pipeline.rs`, `mock_nhl_api.rs`).

After Hart.5c the codebase has exactly one model: the normalized
`PlayerIdentity` + `SeasonStats` + `PlayerView` shape from Hart.1-4.
Every consumer reads the model through `PlayerView` accessors. No
shim, no dual-shape data structures.

## Why this needs a spec

Sessions 1-2 of Hart.5b migrated the simple consumers cleanly. Then I
hit architectural depth (`DepthChart` embeds `Player`, `App` stores
`Vec<Player>` long-lived, `render_report` tests build `Player`
literals) and started introducing tactical shims to keep grinding:

- `DepthChartBuilder::build_views` that converted views→Player internally
- `player_from_view` made `pub` to enable `tonight::run_trade`'s
  hypothetical
- Site builder using the same shim

Those commits (`d8703e93` Hart.5b2d/g + `48328b9c` Hart.5b2k) have
been reverted as of `8c3567ef` + `7bf84d8c`. The state is clean again
but architecturally we're stuck — the next consumer migration WILL
introduce the same shims unless we resolve the underlying questions.

This spec resolves them.

## Scope

In:
- DepthChart redesign + every consumer of DepthChart
- TUI App restructure + every TUI screen
- scouting::render_report + its tests
- fantasy::to_scheme_stats + scheme integration
- tonight::run_trade hypothetical
- query.rs + export.rs migrations
- Test pattern shift for files that build `Player` literals
- Final delete of legacy types + flat_view_legacy* + player_builder

Out:
- Anything not blocking Player/Goalie deletion
- New features
- TUI redesign beyond what the App refactor minimally requires

## Currently migrated (8 surfaces)

These are clean PlayerView consumers and stay as-is:
- `commands/rank.rs` (export path)
- `commands/players.rs`
- `commands/project.rs`
- `commands/mates.rs`
- `commands/analysis.rs` (run_class, run_peers, run_compare, run_group::Show)

Plus the data layer (Hart.0-4.1) and the Hart.5a/5b1 work (load
boundary centralized, `PlayerRepository` + `GoalieRepository`
deleted).

## Currently NOT migrated (the Hart.5c surface)

- `commands/team.rs` — uses `DepthChartBuilder::build`
- `commands/tonight.rs::run_trade` — uses `DepthChartBuilder::build`
  with manual `Player` clone-and-mutate for hypothetical
- `commands/scouting.rs::render_report` — takes `&Player` + `&[Player]`
- `commands/query.rs` — multiple subcommands consuming `&Player`
- `commands/fantasy.rs` — `to_scheme_stats(&Player)`
- `commands/export.rs` — 5 markdown shape renderers all on `&Player`
- `icelines-site/src/builder.rs` — render path consumes
  `&DepthChart` (Player-embedded)
- All `tui/screens/*.rs` (10 files) — read from `app.players: Vec<Player>`
- `icelines-fetch/src/player_builder.rs` (used only by 2 L1 tests)

---

## Decisions to make

### D1 — DepthChart shape

**Problem**: `DepthChart` currently embeds `Player` by value:
```rust
pub struct DepthChart {
    pub forward_lines: Vec<[Option<Player>; 3]>,
    pub defense_pairs: Vec<[Option<Player>; 2]>,
    ...
}
```

Three options:

**Option A** — `DepthChart<'a>` lifetime-parameterized:
```rust
pub struct DepthChart<'a> {
    pub forward_lines: Vec<[Option<PlayerView<'a>>; 3]>,
    ...
}
```
Renderers also become `<'a>`. Maximum information available at render
time (any PlayerView accessor). **Cost**: lifetime-parameter virality.
Every function that touches DepthChart now has an `'a`.

**Option B** — `DepthChartSlot` value struct:
```rust
pub struct DepthChartSlot {
    pub player_id: PlayerId,
    pub full_name: String,
    pub team: TeamAbbr,
    pub position: Position,
    pub pace_82: Option<f64>,
    pub headshot_url: Option<String>,
    // … only the fields renderers actually display
}

pub struct DepthChart {
    pub forward_lines: Vec<[Option<DepthChartSlot>; 3]>,
    ...
}
```
DepthChart owns its data — no lifetime, decoupled from any model.
Renderers consume Slot. Builder copies the displayed fields out of
each PlayerView.

**Option C** — keep DepthChart Player-embedded, accept it as the
single allowed `Player` consumer. Delete Player elsewhere; keep it
specifically for DepthChart. Doesn't actually solve anything — the
Player type still exists.

**Recommendation: B.** Renderers don't need full PlayerView access;
they need name, team, pos, pace_82, headshot for the lineup-card
display. DepthChartSlot is a small owned-data type that survives any
data-model evolution. Lifetime virality (Option A) infects every
caller, including the TUI which holds a depth chart across frames.
Option C postpones the problem.

**Open question for review**: does any current renderer of DepthChart
need a field beyond the Slot subset? Audit:
- `render/terminal::render_team_card` — uses full_name, team, pos
- `tui/screens/depth.rs` — same plus pace coloring
- `icelines-site/src/builder.rs::render_team_page` — name, team, pos,
  nhl_id (for cross_team_metrics lookup), plus headshot URL via
  `team_logo_url`/`player_cell` helpers
- `tonight::run_trade` formatters — full_name only

Slot fields needed:
```rust
pub struct DepthChartSlot {
    pub player_id: PlayerId,           // for metrics lookup
    pub full_name: String,              // for display
    pub name_normalized: String,        // for fixture/test matching
    pub team: TeamAbbr,                 // for badge/logo
    pub position: Position,             // for layout
    pub pace_82: Option<f64>,           // for color/rank
    pub goals_per_82: Option<f64>,      // for tooltips
    pub headshot_canonical_url: Option<String>,
}
```

### D2 — TUI App restructure

**Problem**: `App` stores `Vec<Player>` long-lived. `PlayerView<'a>`
borrows from a repo, can't be stored across frames.

**Recommendation**: `App` owns `StatsRepository` + `Season` +
`SeasonType`. Each screen's `render` collects views per-frame inside
its borrow. App methods that today filter `app.players` (e.g.
`app.search_query` filters → screens search/comps) move to view-iter
helpers on a frame-scoped `Frame { repo: &StatsRepository, season,
season_type }` struct.

```rust
pub struct App {
    repo: StatsRepository,
    active_season: Season,
    active_type: SeasonType,
    selected: usize,
    search_query: String,
    // ... non-data UI state (tabs, modes, etc.)
}

impl App {
    pub fn views(&self) -> impl Iterator<Item = PlayerView<'_>> {
        self.repo.skaters(self.active_season, self.active_type)
    }
    pub fn goalie_views(&self) -> impl Iterator<Item = PlayerView<'_>> {
        self.repo.goalies(self.active_season, self.active_type)
    }
}
```

**Season switch**: `App::reload_for_season(season_id)` calls
`load_into_repo` and uses `repo.repo_swap(new_repo)` to atomically
swap. This was the design intent of `repo_swap` from Hart.2.

**Open question for review**: do any screens cache derived data
(sorted lists, computed metrics) across frames? If yes, where does
that cache live now and how does it stay in sync with the repo? My
read: today most TUI screens recompute per-frame anyway (the data is
small; ratatui re-renders the whole frame on every event). Likely
no caching breaks.

### D3 — `tonight::run_trade` hypothetical

**Problem**: BEFORE chart is built from real views; AFTER chart needs
a player added to a team they're not currently on.

**Option A** — `DepthChartBuilder::build_views_with_swap`:
```rust
pub fn build_views_with_swap(
    team: TeamAbbr,
    season: Season,
    base_views: &[PlayerView<'_>],
    swap_in: PlayerView<'_>,
    swap_out_id: PlayerId,
) -> DepthChart {
    // Build slots from base, drop swap_out, insert swap_in with team override.
}
```
Explicit hypothetical contract. The "team override" lives inside
build_views_with_swap as a Slot field assignment, not a Player
mutation.

**Option B** — let consumers construct a `Vec<DepthChartSlot>`
manually with the slot they want, then a `build_from_slots` API.
More general but more rope.

**Recommendation: A.** The hypothetical is the only known use case;
explicit fn captures intent and prevents general slot-mutation
mischief.

### D4 — `scouting::render_report` test fixtures

**Problem**: tests build `Player` struct literals. Migrating
`render_report` to take `&PlayerView` requires a different test
fixture pattern.

**Recommendation**: introduce a test helper at
`icelines-core/src/fixtures.rs::test_repo_with(identity, stats)`:
```rust
pub fn test_repo_with(
    identity: PlayerIdentity,
    stats: SeasonStats,
) -> StatsRepository {
    let mut r = StatsRepository::new();
    r.upsert_identity(identity).unwrap();
    r.upsert_stats(stats).unwrap();
    r
}
```

Then `scouting::render_report` tests:
```rust
fn fixture_repo_and_view() -> (StatsRepository, PlayerId, Season, SeasonType) {
    let id = fixtures::identity(8478402).build();
    let stats = fixtures::stats(8478402, 20242025, "EDM").build();
    let repo = fixtures::test_repo_with(id, stats);
    (repo, PlayerId(8478402), Season(20242025), SeasonType::Regular)
}

#[test]
fn l0_format_terminal_includes_all_eight_sections() {
    let (repo, pid, s, t) = fixture_repo_and_view();
    let view = repo.view(pid, s, t).unwrap();
    let out = render_report(&view, std::slice::from_ref(&view), None, &[], "terminal");
    // ... assertions
}
```

**Open question for review**: should `render_report` keep its current
8-section signature, or is now a good time to split into per-section
functions (one per render concern)? My read: keep as-is; this
spec is about migrating off Player, not redesigning scouting.

### D5 — `fantasy::to_scheme_stats` migration

**Problem**: `to_scheme_stats(&Player)` produces `scheme::SkaterStats`
for `compute_fantasy_score`. Currently invoked in fantasy.rs's
`score_team`, `run_standings`, etc.

**Recommendation**: replace with `to_scheme_stats_view(&PlayerView<'_>)`
in fantasy.rs (private to fantasy.rs since the conversion is local
display logic). All field reads are direct accessor or stats.totals
mappings — same shape as `fantasy_score_view` we already added
to icelines-core::cross_team. Hart.5c can lift this into icelines-core
if it grows a second consumer, but for now keep local.

**Goalie analog**: `to_goalie_scheme_stats(&Goalie)` becomes
`to_goalie_scheme_stats_view(&PlayerView<'_>)`. Reads from
`view.stats.goalie.as_ref()` instead of `g.stats`.

### D6 — Order of operations

Hart.5c needs to land in this order (each is a separate commit):

1. **Hart.5c.1**: Define `DepthChartSlot`. Refactor `DepthChart` to
   `Vec<[Option<DepthChartSlot>; 3]>` etc. Update
   `DepthChartBuilder::build` to produce slots. Add
   `DepthChartBuilder::build_views`. Migrate `render_team_card`,
   `tui/screens/depth.rs`, `icelines-site::render_team_page` to consume
   slots. Migrate `team.rs`, `tonight::run_trade` (with
   `build_views_with_swap`), `icelines-site::build()` to use
   `build_views`/`build_views_with_swap`.

2. **Hart.5c.2**: Migrate `scouting::render_report` to take
   `&PlayerView` + `&[PlayerView]`. Add `fixtures::test_repo_with`
   helper. Rewrite scouting tests against the new fixture pattern.

3. **Hart.5c.3**: Migrate `query.rs` subcommands (run_leaders,
   run_player, run_compare, run_similar) to PlayerView. Internal
   helpers (`leaders_table`, `print_current_stats`, `print_percentile`,
   `position_percentile`, `find_player`, `pace_strings`,
   `age_from_birth_date`, `draft_str`) all migrate.

4. **Hart.5c.4**: Migrate `fantasy.rs` (`to_scheme_stats_view`,
   `to_goalie_scheme_stats_view`, all run_* commands).

5. **Hart.5c.5**: Migrate `export.rs` (5 markdown shape renderers).

6. **Hart.5c.6**: TUI App restructure. Rewrite `App` to own
   `StatsRepository`. `reload_for_season` calls `load_into_repo` +
   `repo_swap`. Per-frame view collection in screens. This commit
   touches all 10 TUI screens since they all read `app.players`.

7. **Hart.5c.7**: Final delete. Remove `flat_view_legacy`,
   `flat_view_legacy_goalies`, `Player`, `Goalie`,
   `model::GoalieSeasonStats`, `icelines-fetch::player_builder` module,
   `tests/integration_pipeline.rs`, `tests/mock_nhl_api.rs`. Remove
   `pub fn player_from_view` and `pub fn goalie_from_view` (no longer
   needed). Remove `static_assertions`-style allow-deprecated annotations
   that reference the deleted types.

**Each commit independently green.** L2 system tests gate every
commit. The deleted L1 test files in 5c.7 are the only test
deletion; everything else replaced or kept.

---

## Test impact

| File | Change | Hart.5c sub-phase |
|---|---|---|
| `tests/transactions_fixture.rs` | unchanged | — |
| `tests/stats_loader.rs` | unchanged | — |
| `tests/integration_phase2.rs` | might break — uses `make_player` fixture; check whether the local `make_player` builds a `Player` struct or a `PlayerView`. If Player, deleted in 5c.7 | 5c.7 |
| `tests/integration_pipeline.rs` | DELETED in 5c.7 (uses player_builder) | 5c.7 |
| `tests/mock_nhl_api.rs` | DELETED in 5c.7 (uses player_builder) | 5c.7 |
| `tests/transactions_storage.rs` | unchanged | — |
| `tests/system_tests.rs` | unchanged (L2, runs binary) | — |
| `tests/proof_lib_smoke.rs` | unchanged | — |
| `commands/scouting.rs` test mod | rewritten in 5c.2 | 5c.2 |
| `commands/players.rs` test mod | unchanged (no Player literals) | — |
| `commands/query.rs` test mod | rewritten in 5c.3 if any Player literals | 5c.3 |
| `tui/screens/*.rs` test mods | rewritten in 5c.6 | 5c.6 |

---

## Risks

1. **`DepthChartSlot` field set is wrong**. Add a missing field after
   migration → either widen Slot or refactor renderer to compute it.
   Mitigation: audit every renderer of DepthChart in the spec review.

2. **TUI App refactor breaks key handling.** Some screens may rely on
   `app.players[selected]` indexing in event handlers, not just
   render. Need to audit event flow across screens.

3. **`PlayerView` accessor surface might be incomplete**. We've
   added `is_rankable / pp_assists / per_82 helpers / pace_82 /
   pace_sort_key / toi_mmss / hits_per_82 / blocked_shots_per_82`.
   Audit query.rs for any other Player method that needs an analog.

4. **Test runtime regression** if every test now spins up a small
   StatsRepository. Mitigation: `fixtures::test_repo_with` is one-line
   and 1-2 microseconds; net wash.

5. **L2 system tests cover end-to-end CLI behavior** but not
   per-screen TUI rendering. Hart.5c.6 (TUI restructure) carries
   visible regression risk with no L2 fence. Mitigation: smoke-test
   the TUI manually after 5c.6 lands; consider adding an L2 TUI
   snapshot test.

6. **Hart.5c.7 deletion of integration_pipeline.rs / mock_nhl_api.rs
   loses test coverage** that was exercising specific build paths
   (NHL API mock → bios+stats → Player). Audit: do those tests
   exercise anything not covered by `tests/stats_loader.rs` (which
   tests `load_into_repo`)? My read: they test the fetcher-side path
   (NHL API → SkaterBio), which `load_into_repo` doesn't test. We may
   need to write `tests/mock_nhl_api_loader.rs` that runs the mock
   API through `load_into_repo` instead of through the deleted
   `player_builder`.

---

## What I'm asking the reviewers

**forge** (Rust soundness + lifetime design + pub-API discipline):
- D1 Option B vs A — is the lifetime virality of A actually a problem
  given DepthChart is short-lived in CLI/site contexts? In TUI where
  it's stored across frames, A would force `App<'a>` which infects
  the universe. Confirm B is the right call for that reason.
- D2 — `App` owns `StatsRepository`, screens borrow per-frame. Is
  there a Send/Sync concern? StatsRepository is `!Send + !Sync` by
  design; ratatui is single-threaded; no issue. Confirm.
- D3 — `build_views_with_swap` shape sound?
- The `pub player_from_view` from the reverted commits is gone. After
  5c the function (and its goalie analog) goes away entirely. Confirm
  that's the right disposition.

**glass** (TUI architecture):
- D2 — App restructure. Per-frame view collection in screens. Any
  screen that needs to maintain selection state across frames — does
  the new design preserve that?
- Season-switch keystroke (`y`) calls `reload_for_season`. Does
  `repo_swap` invalidate any held borrows? The compile_fail doctest
  on `repo_swap` proves this is borrow-checked, but the human flow
  (event handler → reload → next frame's render) needs design.
- D6 step ordering — should TUI screens migrate before or after CLI
  consumers? My read: after, because TUI is the riskiest. Confirm.

**bench** (test discipline):
- D4 — `fixtures::test_repo_with` helper. Is this the right shape?
  Should it return `(StatsRepository, PlayerId, Season, SeasonType)`
  or just the repo + a method?
- Risk #6 — replace integration_pipeline.rs / mock_nhl_api.rs with
  a new `tests/mock_nhl_api_loader.rs`? Or accept the coverage loss?
  Audit what those tests actually fence.
- TUI L2 snapshot test for Hart.5c.6 — viable?

**tape** (data integrity through the migration):
- D1 Option B — `DepthChartSlot` carries `pace_82` and `goals_per_82`
  but not the underlying `PaceScore.gp` or `raw_points`. Are any
  downstream consumers reaching into PaceScore.gp through a
  DepthChart slot? Audit.
- D3 — `build_views_with_swap` overrides the swap-in player's team
  to the destination team. Is there a TAPE concern here (the AFTER
  chart is now showing a player on a team they're not actually on
  per the data) — yes, that's the whole point of the hypothetical.
  Confirm the documentation makes this explicit.

---

## What's NOT in this spec

- Hart.6 (playoff bundled data) — deferred per the master plan.
- Renames or naming convention changes.
- New PlayerView accessors beyond what's already in icelines-core
  (audit task #3 above may surface needs).
- API stability beyond the workspace (icelines-core is internal; no
  semver concerns).

## Next step after this spec is reviewed

If approved as v0.2 after review punch list:
1. Implement Hart.5c.1 (DepthChartSlot + consumers).
2. Run forge / tape / bench review on the implementation diff.
3. Repeat for Hart.5c.2 through 5c.7.
4. Hart phase ships with `model.rs::Player` and `model.rs::Goalie`
   deleted. Single-model invariant achieved.
