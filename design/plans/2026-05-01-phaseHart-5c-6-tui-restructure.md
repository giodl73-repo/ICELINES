# Phase Hart.5c.6 — TUI App Restructure (v0.1, pre-review)

**Status**: Draft v0.1 — needs forge / glass / bench / tape review
**Date**: 2026-05-01
**Trophy**: Hart (sub-phase of 5c)
**Predecessor**: design/plans/2026-05-01-phaseHart-5c-final-cleanup.md (v0.3)
**Replaces**: nothing — sub-spec for the largest 5c sub-phase

---

## Goal

Restructure the TUI App to own a `StatsRepository` directly (not `Vec<Player>` /
`Vec<Goalie>`), and migrate every screen to read through `PlayerView<'_>`. After
this commit, season switch is a one-call `repo_swap` plus cache invalidation;
nothing reads `app.players` or `app.goalies` in the codebase.

This is the largest single sub-phase in Hart.5c by line count (~2,500 lines in
`app.rs` plus 14 screen files). It needs its own design before implementation
because the App refactor cascades into seven specific architectural decisions, none
of which are pinned by the parent 5c v0.3 spec.

## Why this needs a sub-spec

Hart.5c v0.3 D2 sketches the App restructure at a high level. What it doesn't pin:

1. **App field layout**: cache `Vec<PlayerView>` on App, or recompute per-frame?
2. **`dashboard_panel.compile` API**: current sig takes `&[Player]` + `&Player`;
   needs to take repo + (season, type, player_id). Substantial rewrite.
3. **`LeagueContext` rebuild**: same shape change. Exact API is open.
4. **`loader.rs` contract**: today returns `(Vec<Player>, Vec<Goalie>)` and feeds
   `app.players`/`app.goalies`. Needs to return `LoadOutcome` so App can
   `repo_swap`. Plus the season-switch flow must atomically swap + invalidate.
5. **Per-screen migration table**: 14 screen files, each independent. Spec doesn't
   enumerate each screen's reads; some need new view-helper accessors.
6. **TUI snapshot harness fixture**: signature is pinned in 5c v0.3
   (`render_screen(repo, season, season_type, screen) -> Buffer`), but body is
   open: which fixture repo? Which screens to snapshot? Golden file format?
7. **Sort/filter caching strategy**: ratatui re-renders every frame. Per-frame
   view collection adds a `repo.skaters().collect()` cost — measure or accept?

Hart.5c v0.3 + 4-role review caught 8 BLOCKERs at this scale. Same pattern needed
here.

---

## Pre-conditions

- Hart.5c.0 through 5c.5 complete (every non-TUI consumer migrated). ✓ as of
  commit `f5d53821`.
- Hart.5c v0.3's pinned harness signature must hold:
  `render_screen(repo, season, season_type, screen) -> Buffer`.
- `cross_team::compute_*_views` accept slice of `PlayerView<'_>` (already true
  post-Hart.5b2c).
- `players::load_repo_for_season` returns `(LoadOutcome, Season)`
  (already true post-Hart.5c.3).

---

## Decisions to make

### D1 — App field layout: per-frame view collection

**Problem**: ratatui re-renders the whole frame on every event. If `App` stores a
`Vec<PlayerView<'a>>`, the lifetime `'a` infects every field and propagates into
event handlers — `App<'a>` is unworkable.

**Recommendation**: per-frame view collection. App owns owned data; views are
constructed inside each render frame from `app.repo.skaters(s, t).collect()`.

```rust
pub struct App {
    repo: StatsRepository,
    active_season: Season,
    active_type: SeasonType,
    selected: usize,
    search_query: String,
    // ... non-data UI state (tabs, modes, caches)
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

**Cost**: per-frame `repo.skaters().collect()` is ~1k iterations + a Vec alloc.
At 60 fps this is ~60k iterations/sec — negligible. Measured on a release build
of the depth screen, the full frame budget is dominated by ratatui rendering, not
view collection.

**Open question for review**: any screen that today caches sorted/filtered subsets
(query results, depth rankings) — is per-frame recompute acceptable, or do we need
a per-screen scratch cache that invalidates on `app.search_query` change?

### D2 — `dashboard_panel.compile` API

**Problem**: `dashboard_panel.compile(all_players: &[Player], target: &Player)`
needs to migrate. The compile produces a sparkline + percentile-bar render pinned
to one player.

**Recommendation**: take `(repo, season, season_type, player_id)`:

```rust
impl CompiledPanel {
    pub fn compile(
        &mut self,
        repo: &StatsRepository,
        season: Season,
        season_type: SeasonType,
        player_id: PlayerId,
    ) -> Result<(), DashboardError> {
        let view = repo.view(player_id, season, season_type)
            .ok_or(DashboardError::PlayerNotInRepo)?;
        // … sparkline build over view.identity + view.stats.totals.pace_82 …
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}
```

The cache (per-player compiled output) clears on season switch.

### D3 — `LeagueContext` rebuild

**Problem**: `LeagueContext` holds `pace_82_by_pos: HashMap<Position, Vec<f64>>` —
sorted-descending pace_82 per position, used for percentile lookups in dashboard
panels. Needs to rebuild on season switch.

**Recommendation**: associated function `LeagueContext::build`:

```rust
impl LeagueContext {
    pub fn build(
        repo: &StatsRepository,
        season: Season,
        season_type: SeasonType,
    ) -> Self {
        let mut pace_82_by_pos: HashMap<Position, Vec<f64>> = HashMap::new();
        for view in repo.skaters(season, season_type) {
            if let Some(p) = view.pace_82() {
                pace_82_by_pos
                    .entry(view.position())
                    .or_default()
                    .push(p);
            }
        }
        for v in pace_82_by_pos.values_mut() {
            v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        }
        LeagueContext { pace_82_by_pos }
    }

    pub fn empty() -> Self { LeagueContext { pace_82_by_pos: HashMap::new() } }
}
```

Owned data; no lifetime param. Rebuilt on season switch.

### D4 — `loader.rs` contract

**Problem**: today's TUI loader returns `(Vec<Player>, Vec<Goalie>)`. After this
sub-phase, App needs to `repo_swap` a freshly loaded `StatsRepository`.

**Recommendation**: loader returns `LoadOutcome` directly (it already exists in
icelines-fetch::stats_loader). The TUI `LoadState` type evolves from
`Loaded(Vec<Player>, Vec<Goalie>)` to `Loaded(LoadOutcome)`.

```rust
pub enum LoadState {
    Idle,
    Loading,
    Loaded(LoadOutcome),
    Error(String),
}

// In App initialization (loader callback):
match load_outcome {
    Ok(outcome) => {
        self.repo = outcome.repo;
        self.league_context = LeagueContext::build(&self.repo, season, ty);
    }
    Err(e) => self.load_state = LoadState::Error(e.to_string()),
}
```

### D5 — Season switch (`reload_for_season`)

**Problem**: `y` season picker → `reload_for_season(season, ty)` must:
1. Load new outcome (potentially blocking ~50ms; runs on background task today).
2. Atomically swap the repo.
3. Invalidate caches that depend on (season, type).
4. Reset `selected` indices that may now be out of bounds.

**Recommendation**:

```rust
pub async fn reload_for_season(
    &mut self,
    season: Season,
    ty: SeasonType,
) -> anyhow::Result<()> {
    let store = SnapshotStore::new(&self.config.snapshot_dir());
    let outcome = load_into_repo(season, ty, &store)
        .map_err(|e| anyhow::anyhow!("loading repo: {e}"))?;
    self.repo.repo_swap(outcome.repo);            // atomic, borrow-checked
    self.active_season = season;
    self.active_type = ty;
    self.dashboard_panel.clear_cache();           // glass #2
    self.league_context = LeagueContext::build(&self.repo, season, ty);
    self.selected = 0;                            // safe reset
    Ok(())
}
```

**Borrow-check note**: `repo_swap` is the documented atomic-swap method on
`StatsRepository`. The compile_fail doctest at `stats_repository.rs:513` proves
borrows of the OLD repo cannot survive the swap. Any held `PlayerView` (e.g.,
inside an in-progress render frame) would fail to compile.

### D6 — Screen variant migration (PlayerId-keyed)

**Problem**: `Screen::Player(usize)`, `Screen::Comps(usize)`, `Screen::GoalieDetail(usize)`
encode indexes into `app.players` / `app.goalies`. After per-frame view collection,
indexes have no stable meaning. Glass #1 in 5c v0.3 — re-key on `PlayerId`.

**Migration**:

```rust
pub enum Screen {
    Home,
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

Selection state in list screens (`app.selected: usize`) stays — it's per-frame
indexing into the per-frame sorted iterator, not into long-lived state.

**Selection-state contract**: when a list-screen press of `Enter` navigates to
`Screen::Player(pid)`, the screen's `handle_key` resolves the selected view's
`PlayerId` once at navigation time and stores that. The detail screen looks up by
ID per-frame. If the underlying player vanishes from the repo (rare — only on
season switch with extreme roster changes), the detail screen renders an empty-
state "player not found in current season" panel and Esc returns to the list.

### D7 — Sort/filter caching

**Problem**: 4 screens currently compute filter+sort against `app.players` per
frame. Per-frame view collection adds the collect cost; for the filter+sort path,
that's negligible. But screens like `queries.rs` apply the user's saved query
spec (multi-field filter) and could benefit from caching when the query
hasn't changed.

**Recommendation**: defer query result caching to v1.1. For 5c.6, all screens
recompute per-frame. At N≈1000 records, full filter+sort is sub-millisecond on a
modern CPU; the user-facing budget (60 fps = 16 ms/frame) is never saturated by
view operations.

If a future profile shows real cost: add a per-screen `Option<CachedQueryResult>`
field with a hash of (search_query, query_fields) as the invalidation key.

---

## Per-screen migration table

14 files in `tui/screens/` plus the 7 cross-cutting `tui/*.rs` files. Each row
captures: current reads (what the file accesses on the App), the migration shape,
and complexity.

| File | Today reads | Migration | Complexity |
|---|---|---|---|
| `screens/home.rs` | `app.players` for league rankings | `app.views().collect()` + sort | Low |
| `screens/team.rs` | `app.players` filtered by team | `app.views().filter(team)` | Low |
| `screens/depth.rs` | `app.players` for greedy spill + cross-team metrics | per-frame views + `compute_all_views` | Medium — re-tests the spill algorithm |
| `screens/player.rs` | `app.players[idx]` from `Screen::Player(idx)` | `app.repo.view(pid, s, t)` from `Screen::Player(pid)` | Medium — empty-state handling |
| `screens/comps.rs` | similar to player.rs | view-based comps | Medium |
| `screens/goalies.rs` | `app.goalies` for leaderboard | `app.goalie_views()` | Low |
| `screens/queries.rs` | `app.players` + saved query spec | `apply_views(views)` + sort | Medium — preserves saved-query JSON shape |
| `screens/search.rs` | `app.players` substring match | `views.find(name_normalized.contains(q))` | Low |
| `screens/schedule.rs` | NHL schedule (independent of players) | unchanged | None |
| `screens/playoffs.rs` | Playoffs bundle (independent of players) | unchanged | None |
| `screens/transactions.rs` | `app.transactions` (independent) | unchanged | None |
| `screens/game_detail.rs` | NHL boxscore (independent) | unchanged | None |
| `screens/misc.rs` | static admin overlay | unchanged | None |
| `screens/mod.rs` | dispatch | screen variants update for D6 | Low |
| `app.rs` | App struct + handlers | full restructure | High — the core of this commit |
| `dashboard_panel.rs` | `compile(&[Player], &Player)` | per D2 | Medium |
| `loader.rs` | returns `(Vec<Player>, Vec<Goalie>)` | per D4 | Low |
| `event.rs` | event dispatch | unchanged signatures | None |
| `headshot.rs` | nhl_id keyed cache | unchanged | None |
| `playoffs.rs` | Playoffs cache | unchanged | None |
| `schedule.rs` | Schedule cache | unchanged | None |

Net: ~2 high-complexity files (`app.rs`, `screens/depth.rs`), ~5 medium, ~7 low,
~7 untouched.

---

## L2 TUI snapshot test (required deliverable)

Per Hart.5c v0.3, a new test file `icelines-cli/tests/tui_snapshot.rs` lands in
this commit using `ratatui::backend::TestBackend`.

### Pinned signature

```rust
fn render_screen(
    repo: &StatsRepository,
    season: Season,
    season_type: SeasonType,
    screen: Screen,
) -> ratatui::buffer::Buffer
```

### Fixture and goldens

**Fixture repo**: bundled current season (`CURRENT_SEASON`, `Regular`). Loads
deterministically from the binary's embedded data. ~1000 skater + ~70 goalie
records. No live network, no snapshot dependency.

**Screens to snapshot** (8 top-level + 1 representative drill-down):

| Screen | Coverage |
|---|---|
| `Screen::Home` | League rankings list |
| `Screen::Players` | Stats tab default sub-view |
| `Screen::DepthLeague` | Cross-team rankings table |
| `Screen::DepthTeam(EDM)` | Team depth grid (representative) |
| `Screen::Goalies` | Goalie leaderboard |
| `Screen::Schedule` | Today's schedule (uses bundled date math; no live API) |
| `Screen::Playoffs` | Playoffs bracket (1993-94 historical fixture) |
| `Screen::Transactions` | Transactions feed (current season bundled) |
| `Screen::Player(McDavidId)` | Drill-down detail card |

**Golden format**: one `.snap` file per (screen, season, type) tuple at
`icelines-cli/tests/tui_snapshot/{screen}__{season}__{type}.snap`. Each file
contains the buffer's rendered text representation (rows of cells flattened to
strings; styles ignored in v1).

```
$ ls icelines-cli/tests/tui_snapshot/
home__20252026__regular.snap
players__20252026__regular.snap
depth_league__20252026__regular.snap
depth_team_EDM__20252026__regular.snap
goalies__20252026__regular.snap
schedule__20252026__regular.snap
playoffs__20252026__regular.snap
transactions__20252026__regular.snap
player_8478402__20252026__regular.snap
```

Test body asserts `render_screen(...).rendered_text() == read(snapfile)`.
Updating goldens: `INSIDE_GOLDEN_UPDATE=1 cargo test tui_snapshot` writes new
snapshots; review and commit. (Pattern adopted from `insta`-style snapshot tests
without taking on the dep.)

### Final gate

```
$ Grep "app\.players|app\.goalies" icelines-cli/src/tui/
# zero hits required to merge.
```

Spec gate per Glass #4 in 5c v0.3.

---

## Test impact

| File | Change | Notes |
|---|---|---|
| `tests/tui_snapshot.rs` | NEW | 9 screen goldens + harness |
| `app.rs` test mod | rewritten | League context build, repo_swap invariant |
| `screens/*.rs` test mods | rewritten in 5c.6 | per-screen render with view fixtures |
| `loader.rs` test mod | LoadState::Loaded(LoadOutcome) | |
| `dashboard_panel.rs` test mod | new compile signature | |

---

## Risks

1. **TestBackend rendering doesn't match production** — different terminal
   emulators, font metrics, etc. Mitigation: snapshot the buffer (cells), not
   the visual output. Style differences are ignored in v1; layout is byte-stable.

2. **Bundled fixture changes between releases** — if the bundled `bios.json` /
   `stats.json` updates (CI re-bakes each release), goldens must update. This
   is by design — CI catches accidental output regressions when the fixture
   updates. Updating goldens is a deliberate human step.

3. **dashboard_panel cache invalidation race** — if a season switch fires while
   a render frame is in flight, the old panel's cache entries point to a swapped
   repo. Mitigation: `repo_swap` is borrow-checked; in-flight `PlayerView`s would
   fail to compile. The cache stores compiled output (owned), not views — safe.

4. **`Screen::Player(PlayerId)` after season switch** — if the user is on a
   player detail screen and switches to a season where that player doesn't exist,
   the screen renders an empty state. UX-acceptable; alternative (kick to home)
   would be more disruptive.

5. **Selection state after season switch** — `app.selected = 0` resets cursors.
   User loses position in long lists. Mitigation: probably fine; alternative is
   "preserve by player_id where possible" which adds complexity for marginal UX.

6. **Per-frame view collection cost at scale** — if data scale ever grows past
   ~10× (e.g., per-game shift logs), per-frame `collect()` becomes measurable.
   Mitigation: add per-screen scratch cache on the v1.1 path. Not blocking 5c.6.

7. **Per-screen migration error surface area** — 14 files, 5+ medium-complexity
   migrations. High blast radius for a single commit. Mitigation: TUI snapshot
   test catches output regressions; manual smoke test before merge.

---

## What I'm asking the reviewers

**forge** (Rust soundness + lifetime design):
- D1 — per-frame view collection avoids `App<'a>` lifetime virality. Is this the
  right trade-off vs caching views on App with a `'static` lifetime via `Box`-ed
  StatsRepository?
- D5 — `repo_swap` borrow-check semantics correctly described?
- D6 — `Screen::Player(PlayerId)` re-key sound for borrow-check (PlayerId is Copy)?

**glass** (TUI architecture):
- D2 — `dashboard_panel.compile(repo, s, t, pid)` — is the cache invalidation
  contract complete? Should panels also invalidate on `dashboards` config change?
- D6 — empty-state handling for `Screen::Player(missing_pid)` — render in-place
  or auto-pop to parent? Spec says in-place; gut-check.
- L2 snapshot test deliverable: 9 screens enough? Other screens that warrant a
  golden?

**bench** (test discipline):
- TUI snapshot harness golden format — buffer text vs full buffer with styles.
  Acceptable simplification?
- Per-screen migration table — does it capture all reads? Any screen reading
  `app.players` indirectly (e.g., through a helper) that I missed?

**tape** (data integrity through migration):
- D5 — season switch invalidates `dashboard_panel` cache + rebuilds
  `league_context`. Anything else cached on App that depends on (season, type)?
- D7 — sort/filter caching deferred. If queries.rs caches are added later,
  invalidation contract is `app.search_query` + `app.query_fields` change?
  Confirm there are no other invalidation triggers.

---

## What's NOT in this spec

- Hart.5c.7 (final delete of legacy types) — separate sub-phase.
- Hart.6 (playoff data) — separate spec.
- Per-screen UI redesigns — only data-path migration.
- Performance profiling — assume current scale; add caching only if profile
  shows real cost (post-v1.0).
- Snapshot test for screens that don't depend on player data
  (`schedule.rs`, `playoffs.rs`, `transactions.rs`, `game_detail.rs`,
  `misc.rs`) — those screens are unchanged in 5c.6 and don't need new goldens.

## Next step

If approved as v0.2 after review:
1. Implement App restructure (`app.rs` + `loader.rs` + `dashboard_panel.rs` +
   `LeagueContext`).
2. Migrate screens: low-complexity batch first (home, team, goalies, search),
   then medium (player, comps, queries, depth), then verify final gate
   (`grep app.players|app.goalies`).
3. Land the TUI snapshot test as the final commit step.
4. Manual smoke test: launch TUI, navigate every tab, switch season via `y`,
   confirm dashboards re-render, confirm selection state is sane.
