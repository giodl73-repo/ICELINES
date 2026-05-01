# Phase Hart.5c.6 — TUI App Restructure (v0.2, post-review)

**Status**: v0.2 — incorporates 5-role review (forge / glass / bench / tape / pace)
**Date**: 2026-05-01
**Trophy**: Hart (sub-phase of 5c)
**Predecessor**: design/plans/2026-05-01-phaseHart-5c-final-cleanup.md (v0.3)
**Replaces**: nothing — sub-spec for the largest 5c sub-phase

---

## v0.1 → v0.2 changelog

5 parallel role reviews returned **10 BLOCKERs + 17 FIXITs + 8 NITs**. Punch list
applied below.

### BLOCKERs resolved

- **forge B1** — `StatsRepository: !Send + !Sync` (`PhantomData<*const ()>` at
  `stats_repository.rs:81`). Today's TUI loader uses `tokio::spawn`; that won't
  work after migration since `LoadOutcome.repo` isn't `Send`. **Resolution: new
  D8** pinning `spawn_local + LocalSet` as the load model. `tokio::spawn` is out;
  `spawn_blocking` is also out (return must be `Send`).

- **forge B2** — `repo_swap` returns the OLD `StatsRepository` (via `mem::replace`),
  not `()`. Resolution: D5 sample updated to `let _old = self.repo.repo_swap(new);`
  and ARCHITECTURE.md doc fixed.

- **glass B1** — `screens/misc.rs` reads `app.players` at lines 327, 337, 480,
  617, 620 (verified). Resolution: migration table row split into 3 rows
  (render_projections Medium, render_groups/group_members Low, status_footer Low).

- **glass B2** — `Screen::DepthLeague` doesn't exist. Actual variant in
  `screens/mod.rs:45` is `Screen::Depth`. Resolution: snapshot list and golden
  filenames corrected.

- **glass B3 + bench B2** — `tui/mod.rs` (loader callback writes
  `app.players`/`app.goalies` at lines 60, 69, 70, 77, 79), `widgets/mod.rs`
  (`player_cell_text(&Player, ...)` at line 10), `tonight.rs` (160 lines),
  `sparkline.rs` (193 lines) all missing. Resolution: 4 new rows in migration
  table.

- **bench B1** — Test-tier breakdown numbers wrong:
  L1 fetch is **255** (140 lib + 115 across 7 integration files), not 140.
  L2 cli is **140**, not 35 (off by 4×).
  Resolution: ARCHITECTURE.md test strategy section corrected.

- **pace B1** — TUI is event-driven at 100 ms poll cap (10 fps), not 60 fps.
  Resolution: every "60 fps" reference in this spec replaced; budget framed as
  per-event (~16 ms perceptible threshold), not steady-state.

- **pace B2** — `compute_all_views` is O(N · T · K) where T=32, K=~10, total
  ≈320k comparisons. Not O(N²) ≈ 1M. Resolution: ARCHITECTURE.md complexity
  label corrected; this spec's references corrected.

- **tape B1** — There is no single 5-tier fallback chain; each data source has
  its own ordering. Resolution: ARCHITECTURE.md "5-tier" diagram replaced with
  per-source list. Live API isn't a query-time tier.

- **tape B2** — D5's invalidation list is incomplete. Resolution: D5 expanded to
  reset `transactions` envelope, `tx_*` filter state, `playoffs_round`,
  `playoffs_series`, `query_result_scroll`, `tx_selected`.

### FIXITs applied

- **forge F1** — `team_roster` IS O(1)-indexed via `rosters_last_stint` /
  `rosters_all_stints` HashMaps. Architecture doc corrected; this spec
  acknowledges the index in D1.
- **forge F2** — `RepoError`, `IdentityMergeError` added to architecture error
  model.
- **forge F3** — D1 includes explicit Box-rebuttal sentence.
- **forge F4** — D6 PlayerId-Copy property pinned as fact.
- **forge F5** — D5 sample reordered (compute new context, then assign).
- **glass F1** — D2 cache invalidation expanded: keyed by `(PlayerId, Season,
  SeasonType)`; also clears on `dashboards` config change.
- **glass F2** — D6 empty-state UX changed: auto-pop to parent + 2-second toast,
  not stranded detail view.
- **glass F3** — selection-state on season switch: clamp via
  `selected.min(new_count - 1)`, not reset to 0.
- **glass F4** — D7 explicitly flags `Screen::Depth` and `Screen::Queries` as
  v1.1 caching candidates.
- **glass F5 + bench F2** — Snapshot coverage adds `Screen::GoalieDetail`,
  `Screen::Comps`, `Screen::Search`. List grows from 9 to 12.
- **bench F1** — Migration table covers `app.rs:1606` (selected-rank →
  `app.goalies`), `app.rs:1791` (test-mod assertion), `screens/depth.rs:136/166`
  (`Vec<&Player>` reference — bumped to High). Plus tape's transactions /
  playoff cursor invalidation.
- **bench F3** — Snapshot test scope explicitly excludes style regressions
  (color, modifiers, selected-row highlight); flagged for v1.1 `.snap.styled`.
- **bench F4** — `INSIDE_GOLDEN_UPDATE` env-var: error if `CI=true`.
- **bench N3** — Per-screen test mods reuse `integration_phase2.rs::fixture_repo()`
  pattern; each screen wraps it locally.
- **bench N1** — 6 Beniers known-value asserts (was 5); ARCHITECTURE.md fixed.
- **bench N2** — `tui-admin-overlay` test coverage lands here under `misc.rs`
  rows; remove from v1.1 backlog.
- **tape F1** — `SeasonStats` description in architecture adds `sweater_number`.
- **tape F2** — Persistence-layer asymmetry documented: only bios/stats fall
  back to embedded; realtime/moneypuck/contracts are snapshot-only.
- **tape F3** — `IceLines.md` links `Hart.6` spec explicitly.
- **tape F4** — D7 deferred-cache invalidation key now includes `(season, type)`.
- **glass N4** — Implementation split into 3 commits (low → medium →
  snapshot test) per D9.

### NITs applied / deferred

- pace numbers across spec: every "microseconds" / "~1 ms" tagged "estimated".
- forge N1 (surfaces line) clarified.
- glass N3 (D1 Box rebuttal): incorporated as F3.
- glass N2 (snapshot styled note): captured in bench F3 wording.
- tape N3 ("5 MB" claim): retained but tagged "estimated".

---

## Goal

Restructure the TUI App to own a `StatsRepository` directly (not `Vec<Player>` /
`Vec<Goalie>`), and migrate every screen to read through `PlayerView<'_>`. After
this commit, season switch is one call (`load_into_repo` + `repo_swap`) plus a
known invalidation set; nothing reads `app.players` or `app.goalies` in the
codebase.

This is the largest single sub-phase in Hart.5c by line count. It needs its own
design before implementation because the App refactor cascades into eight
specific architectural decisions, none of which were pinned by the parent 5c v0.3
spec.

## Pre-conditions

- Hart.5c.0 through 5c.5 complete (every non-TUI consumer migrated). ✓
- Hart.5c v0.3's pinned harness signature must hold:
  `render_screen(repo, season, season_type, screen) -> Buffer`.
- `cross_team::compute_*_views` accept `&[PlayerView<'_>]`.
- `players::load_repo_for_season` returns `(LoadOutcome, Season)`.

---

## Decisions made

### D1 — App field layout: per-frame view collection

**Problem**: ratatui re-renders the whole frame on every event. If `App` stores a
`Vec<PlayerView<'a>>`, the lifetime `'a` infects every field and propagates into
event handlers — `App<'a>` is unworkable.

**Decision**: per-frame view collection. App owns owned data (the
`StatsRepository`); views are constructed inside each render frame from
`app.repo.skaters(s, t)`.

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
    pub fn team_views(&self, team: &TeamAbbr) -> Vec<PlayerView<'_>> {
        // Uses StatsRepository's rosters_last_stint HashMap index — O(1).
        self.repo.team_roster(team, self.active_season, self.active_type)
    }
}
```

**Box-the-repo rebuttal**: Boxing `StatsRepository` inside `App`
(`repo: Box<StatsRepository>`) does NOT escape the lifetime tie. Views still
borrow from `*box`'s contents, and any cached `Vec<PlayerView<'a>>` field still
pins `'a = 'self`. Self-referential storage requires `ouroboros` / `yoke` and
isn't worth a dep at N≈1000.

**Cost**: per-frame `repo.skaters().collect()` is ~1k iterations + a Vec alloc.
TUI redraws on event only (`tui/mod.rs:117` polls every 100 ms); a typed key
produces one frame. Sub-millisecond at this scale (estimated; unmeasured).

**`team_views` is cheap**: `team_roster` uses the `rosters_last_stint` HashMap
index — O(1) lookup + O(roster_size ≈ 25) view materialization. No need to
filter the full skater pool by team string.

### D2 — `dashboard_panel.compile` API + cache invalidation

**Decision**: take `(repo, season, season_type, player_id)`. Cache key includes
the (season, type) tuple, not just `PlayerId`. Cache clears on:
1. Season switch (`reload_for_season`).
2. `dashboards` config change (toggle of `--no-dashboards` or edit of
   `~/.icelines/config.toml`).

```rust
pub struct CompiledPanel {
    cache: HashMap<(PlayerId, Season, SeasonType), CompiledOutput>,
}

impl CompiledPanel {
    pub fn compile(
        &mut self,
        repo: &StatsRepository,
        season: Season,
        season_type: SeasonType,
        player_id: PlayerId,
    ) -> Result<&CompiledOutput, DashboardError> {
        let key = (player_id, season, season_type);
        if !self.cache.contains_key(&key) {
            let view = repo.view(player_id, season, season_type)
                .ok_or(DashboardError::PlayerNotInRepo)?;
            self.cache.insert(key, build_panel(&view)?);
        }
        Ok(&self.cache[&key])
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}
```

Why the (season, type) in the key: percentile bars derive from `LeagueContext`,
which is rebuilt on season switch. A pre-swap compiled panel reopened post-swap
would render stale bars without this keying. F1 from glass review.

### D3 — `LeagueContext` rebuild

**Problem**: `LeagueContext` holds per-position pace_82 sorted vectors for
percentile lookups. Must rebuild on season switch.

**Decision**: associated function `LeagueContext::build(&repo, season, type)`.
Owned data; no lifetime param.

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

    pub fn empty() -> Self {
        LeagueContext { pace_82_by_pos: HashMap::new() }
    }
}
```

### D4 — `loader.rs` contract

**Decision**: loader returns `LoadOutcome` directly (already exists in
`icelines-fetch::stats_loader`). The TUI `LoadState` evolves:

```rust
pub enum LoadState {
    Idle,
    Loading,
    Loaded(LoadOutcome),
    Error(String),
}
```

Loader callback in `tui/mod.rs` no longer mutates `app.players`/`app.goalies`
directly (today's lines 60, 69, 70, 77, 79); instead the App's update loop
applies the `LoadOutcome`:

```rust
// In App::update, after the load completes:
if let LoadState::Loaded(outcome) = std::mem::replace(&mut self.load_state, LoadState::Idle) {
    let _old = self.repo.repo_swap(outcome.repo);
    self.league_context = LeagueContext::build(&self.repo, self.active_season, self.active_type);
    self.dashboard_panel.clear_cache();
}
```

### D5 — Season switch (`reload_for_season`) — full invalidation list

**Tape B2**: every (season, type)-coupled field on `App` must reset on swap, not
just `dashboard_panel` and `league_context`. Audit of `app.rs`:

```rust
pub async fn reload_for_season(
    &mut self,
    season: Season,
    ty: SeasonType,
) -> anyhow::Result<()> {
    let store = SnapshotStore::new(&self.config.snapshot_dir());
    let outcome = load_into_repo(season, ty, &store)
        .map_err(|e| anyhow::anyhow!("loading repo: {e}"))?;

    // 1. Atomic repo swap (borrow-checked; in-flight PlayerView fail to compile).
    let _old = self.repo.repo_swap(outcome.repo);
    self.active_season = season;
    self.active_type = ty;

    // 2. (Season, type)-keyed caches.
    self.dashboard_panel.clear_cache();
    let new_ctx = LeagueContext::build(&self.repo, season, ty);
    self.league_context = new_ctx;

    // 3. Transactions — re-fetch envelope for the new season.
    let (rows, fetched_at, stale) = load_transactions_with_fallback(&self.config, season)
        .unwrap_or_default();
    self.transactions = rows;
    self.transactions_fetched_at = fetched_at;
    self.transactions_stale = stale;
    self.tx_selected = 0;
    self.tx_team_filter = None;
    self.tx_kind_filter = None;
    self.tx_search_query.clear();

    // 4. Playoff cursors — bracket data may be different shape.
    self.playoffs_round = 0;
    self.playoffs_series = 0;

    // 5. Selection state — clamp to new view set, don't reset.
    let new_count = self.repo.skater_count(season, ty);
    self.selected = self.selected.min(new_count.saturating_sub(1));

    // 6. Saved-query scroll — results computed against OLD repo's view set.
    self.query_result_scroll = 0;

    Ok(())
}
```

`new_ctx` builds before assignment so the immutable `&self.repo` reborrow doesn't
overlap with the mutable `repo_swap` (NLL handles it but the temporal split is
clearer). Forge F5.

### D6 — Screen variant migration (PlayerId-keyed) + empty-state UX

**Problem**: `Screen::Player(usize)`, `Screen::Comps(usize)`, `Screen::GoalieDetail(usize)`
encode indexes into `app.players` / `app.goalies`. After per-frame view collection,
indexes have no stable meaning.

**Decision**: re-key on `PlayerId`. `PlayerId` is `Copy + Eq + Hash` (verified at
`core/identity.rs`), so the variants stay `Copy + 'static`; `Screen` itself
remains `Copy`-derivable. Forge F4.

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

**Empty-state UX (revised — glass F2)**: when navigating to
`Screen::Player(missing_pid)` after a season switch, **auto-pop to the parent
list screen** (Players / Goalies / DepthTeam) and set
`app.status = "Connor McDavid not in 1993-94 roster"` for 2 seconds. Less
disruptive than landing on a blank card; preserves user orientation.

```rust
fn render_player(&self, frame: &mut Frame, pid: PlayerId) {
    let view = match self.repo.view(pid, self.active_season, self.active_type) {
        Some(v) => v,
        None => {
            // Auto-pop to parent list + flash status toast.
            self.screen = self.parent_for(Screen::Player(pid));
            self.status = format!("{} not in {}",
                self.repo.identity(pid).map(|i| i.full_name.as_str()).unwrap_or("Player"),
                self.active_season.label(),
            );
            self.status_expires = Instant::now() + Duration::from_secs(2);
            return;
        }
    };
    // … render the card …
}
```

### D7 — Sort/filter caching (deferred to v1.1, key shape pinned)

**Decision**: defer screen-level query caching to v1.1. Two screens are flagged
as the most-likely-first caching candidates per glass F4:

- `Screen::Depth` — calls `compute_team_strength_views` and `compute_all_views`
  per frame. ~320k comparisons (estimated <1 ms). Held-arrow scrolling could
  show measurable CPU; not user-perceivable today but watch for regressions.
- `Screen::Queries` — runs the saved query spec per frame (multi-field filter +
  sort). Sub-millisecond at N≈1000 (estimated).

If a future profile shows real cost, add a per-screen
`Option<CachedQueryResult>` field.

**Pinned cache invalidation key (tape F4)**: any future cache MUST key on
`(active_season, active_type, search_query, query_fields_hash)`. Otherwise season
switch with the same query keeps stale results visible.

### D8 — `!Send + !Sync` constraint and the load model

**forge B1 / fix**: `StatsRepository` plants `PhantomData<*const ()>`
(`stats_repository.rs:81`) and asserts not Send, not Sync (`:84`). After 5c.6,
`LoadOutcome` (which contains a repo) is also `!Send`. Today's loader uses
`tokio::spawn(async move { ... })` which requires `Send` futures — won't compile.

**Decision**: switch the TUI's load model to `spawn_local` + `LocalSet`.

- `tokio::spawn` — out (requires `Send`).
- `tokio::task::spawn_blocking` — out (return value must be `Send`).
- `tokio::task::spawn_local` + `tokio::task::LocalSet` — **chosen**. Single-
  threaded by construction; `!Send` is fine.
- Synchronous on UI thread — rejected; ~50 ms stall on cold load is too long.

```rust
// In TUI bootstrap (one place):
let local = tokio::task::LocalSet::new();
local.run_until(async {
    tokio::task::spawn_local(initial_load_task(...));
    run_event_loop(...).await;
}).await;
```

`reload_for_season` runs in the local set. Background fetches and installs that
exist today already work this way; this just formalizes it.

### D9 — Implementation split (3 commits)

Per glass N4. Each commit independently green; final gate at the end of commit 3.

```
Commit 1 (foundation)
  app.rs restructure (App fields), loader.rs LoadOutcome,
  dashboard_panel.compile API + key shape, LeagueContext::build,
  reload_for_season full invalidation. Low-complexity screens migrated:
  home, team, goalies, search.

Commit 2 (medium-complexity screens)
  player, comps, queries, depth, misc.rs (split into 3 fns),
  widgets/mod.rs::player_cell_text signature, tonight.rs, sparkline.rs.
  Bumped depth.rs to High because of Vec<&Player> references that
  won't compile post-5c.7.

Commit 3 (snapshot test)
  tests/tui_snapshot.rs harness + 12 goldens.
  Final gate verification:
    grep -rn "app\.players\|app\.goalies" icelines-cli/src/tui/
  Must return zero hits to merge.
```

---

## Per-screen migration table (revised)

24 files in `tui/`. Bench / glass review found 4 missing rows + 5 missed reads.

| File | Today reads / writes | Migration | Complexity |
|---|---|---|---|
| `app.rs` | App struct + handlers; `app.players`/`app.goalies` at lines 105, 108, 1303, 1340, 1355, 1365, 1487, 1494, 1528, 1546, 1564, 1606, 1614, 1620, 1791 | full restructure per D1–D8 | **High** |
| `tui/mod.rs` | loader callback writes `app.players`/`app.goalies` at lines 60, 69, 70, 77, 79 | apply `LoadOutcome` per D4; remove field writes | Medium |
| `loader.rs` | returns `(Vec<Player>, Vec<Goalie>)` | returns `LoadOutcome` per D4 | Low |
| `dashboard_panel.rs` | `compile(&[Player], &Player)` | per D2 (key shape change) | Medium |
| `widgets/mod.rs` | `player_cell_text(&Player, ...)` at line 10 | take `&PlayerView`; rename if helpful | Medium |
| `tonight.rs` | (verify reads of `app.players`) | likely unchanged; spec audit at impl time | Low (tentative) |
| `sparkline.rs` | (verify reads) | likely unchanged | None (tentative) |
| `event.rs` | event dispatch | unchanged signatures | None |
| `headshot.rs` | nhl_id keyed cache | unchanged | None |
| `playoffs.rs` (cross-cutting) | playoffs cache | reset cursors on season switch (D5) | Low |
| `schedule.rs` (cross-cutting) | schedule cache | unchanged | None |
| `screens/home.rs` | `app.players` for league rankings | `app.views().collect()` + sort | Low |
| `screens/team.rs` | `app.players` filtered by team | `app.team_views(&team)` (O(1) hashmap) | Low |
| `screens/depth.rs` | `app.players` at lines 26, 31, 136, 166; `Vec<&Player>` references | per-frame views + `compute_all_views` + `compute_team_strength_views`. Bumped to High because Vec<&Player> won't compile after 5c.7. | **High** |
| `screens/player.rs` | `app.players[idx]` from `Screen::Player(idx)` | `app.repo.view(pid, s, t)` from `Screen::Player(pid)`; auto-pop on missing per D6 | Medium |
| `screens/comps.rs` | similar to player.rs | view-based comps | Medium |
| `screens/goalies.rs` | `app.goalies` for leaderboard | `app.goalie_views()` | Low |
| `screens/queries.rs` | `app.players` + saved query spec at lines 282–287 | `apply_views(views)` + sort; preserves saved-query JSON shape | Medium |
| `screens/search.rs` | `app.players` substring match | `views.find(name_normalized.contains(q))` | Low |
| `screens/schedule.rs` | NHL schedule (independent) | unchanged | None |
| `screens/playoffs.rs` | Playoffs bundle (independent) | unchanged | None |
| `screens/transactions.rs` | `app.transactions` (independent of player views) | unchanged in 5c.6; `app.transactions` reset by D5 | Low |
| `screens/game_detail.rs` | NHL boxscore (independent) | unchanged | None |
| `screens/misc.rs::render_projections` | `app.players` at lines 327, 337 | per-frame views + sort | Medium |
| `screens/misc.rs::render_groups`/`render_group_members` | `app.players` at line 480 | substring match on views | Low |
| `screens/misc.rs::status_footer` | `app.players` at lines 617, 620 (`{} players loaded`) | `app.repo.skater_count(s, t)` | Low |
| `screens/mod.rs` | dispatch | screen variants update for D6 | Low |

Net: **3 high-complexity files** (`app.rs`, `screens/depth.rs`, plus possibly
the snapshot harness as a fourth standalone), ~7 medium, ~9 low, ~5 untouched.

---

## L2 TUI snapshot test (required deliverable)

Per Hart.5c v0.3, `icelines-cli/tests/tui_snapshot.rs` lands using
`ratatui::backend::TestBackend`.

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
records. No live network, no snapshot dependency. Reuses
`integration_phase2.rs`-style fixture builders (per bench N3).

**12 screens to snapshot** (was 9 in v0.1; +3 per glass F5 / bench F2):

| Screen | Coverage |
|---|---|
| `Screen::Home` | League rankings list |
| `Screen::Players` | Stats tab default sub-view |
| `Screen::Depth` | Cross-team rankings table (was `DepthLeague` in v0.1 — wrong) |
| `Screen::DepthTeam(EDM)` | Team depth grid (representative) |
| `Screen::Goalies` | Goalie leaderboard |
| `Screen::GoalieDetail(SkinnerId)` | Goalie drill-down — D6 PlayerId re-key proof |
| `Screen::Comps(McDavidId)` | Comps engine over view fixtures |
| `Screen::Search` | Substring match path with fixed query "mcdavid" |
| `Screen::Schedule` | Today's schedule (uses bundled date math; no live API) |
| `Screen::Playoffs` | Playoffs bracket (1993-94 historical fixture) |
| `Screen::Transactions` | Transactions feed (current season bundled) |
| `Screen::Player(McDavidId)` | Player detail card + dashboard panel sparkline |

**Golden format**:
- One `.snap` file per (screen, season, type) tuple at
  `icelines-cli/tests/tui_snapshot/{screen}__{season}__{type}.snap`.
- Each file contains the buffer's rendered text representation (rows of cell
  characters flattened to strings).
- **Out-of-scope (bench F3)**: style regressions (color, modifiers, selected-row
  highlight) are NOT covered by these goldens. A regression that swaps green and
  red would not be caught here. Tracked separately under glass-role manual smoke
  checklist; v1.1 may add `.snap.styled` companion files.

### Update workflow

```
INSIDE_GOLDEN_UPDATE=1 cargo test tui_snapshot
```
Writes new snapshots; print `wrote N goldens — review and commit before push`
to stderr.

**CI guardrail (bench F4)**: if `INSIDE_GOLDEN_UPDATE` is set AND `CI=true`, the
test harness errors out. Prevents accidental green CI runs that overwrite
goldens.

### Final gate (Glass #4 from 5c v0.3)

```bash
$ Grep "app\.players|app\.goalies" icelines-cli/src/tui/
# zero hits required to merge.
```

This is the canonical signoff.

---

## Test impact

| File | Change | Notes |
|---|---|---|
| `tests/tui_snapshot.rs` | NEW | 12 screen goldens + harness with INSIDE_GOLDEN_UPDATE |
| `app.rs` test mod | rewritten | LeagueContext build, repo_swap invariant, reload_for_season full invalidation |
| `screens/*.rs` test mods | rewritten | per-screen render with view fixtures (reuses fixture_repo pattern) |
| `loader.rs` test mod | LoadState::Loaded(LoadOutcome) | |
| `dashboard_panel.rs` test mod | new compile signature, (PlayerId, Season, SeasonType) cache key | |

---

## Risks (revised v0.2)

1. **TestBackend buffer-text format may change between ratatui versions** —
   pinning the dep. Mitigation: pinned in `Cargo.toml`; major-version bumps
   trigger golden refresh.

2. **Bundled fixture changes between releases** — CI re-bakes each release.
   Goldens must update. **By design** — CI catches accidental output regressions
   when the fixture updates. Updating goldens is a deliberate human step.

3. **dashboard_panel cache invalidation race** — fully prevented by `repo_swap`'s
   borrow-check (compile_fail doctest at `stats_repository.rs:513`). In-flight
   `PlayerView` references can't survive the swap. Race is impossible at
   compile time; not a runtime concern.

4. **`Screen::Player(missing_pid)` after season switch** — auto-pops to parent
   with toast (D6, glass F2). UX-acceptable.

5. **Selection state after season switch** — clamps via
   `selected.min(new_count.saturating_sub(1))` (D5, glass F3). User keeps cursor
   when both seasons have ≥ N players.

6. **Per-frame `compute_all_views` cost on `Screen::Depth`** — at N=1000, ~320k
   comparisons (<1 ms estimated). At N=10k (per-game shift logs), becomes
   ~32M ops (~30 ms perceptible). If scale grows, cache `compute_all_views`
   result on `Screen::Depth` keyed by `(season, type, mode)`; invalidate on
   `repo_swap`. Not blocking 5c.6.

7. **`!Send + !Sync` constraint** — handled by D8's `spawn_local + LocalSet`.
   Foreseeable failure: someone adds a new background task with `tokio::spawn`
   in the future. Mitigation: pin a CI clippy lint (`-D non_send_in_local`)
   if needed; for now, code review.

8. **Per-screen migration error surface** — 14 files, 7+ medium-complexity
   migrations. High blast radius for a single 3-commit batch. Mitigation: TUI
   snapshot test catches output regressions; manual smoke test in commit 3.

9. **Boxscore /v1/score data fetched into App** — `tonight_cache`,
   `boxscore_cache`, `schedule_week_cache`, `schedule_team_cache`,
   `playoffs_cache` are independent of player views. They survive `repo_swap`
   without invalidation. **Verified safe**: these caches are keyed by
   game_id / date / season_id / bracket_year, all independent of the
   StatsRepository. No D5 reset needed for them.

---

## What's NOT in this spec

- Hart.5c.7 (final delete of legacy types) — separate sub-phase.
- Hart.6 (playoff data) — separate spec.
- Per-screen UI redesigns — only data-path migration.
- `compute_all_views` caching — deferred to v1.1 unless profile shows real cost.
- Style regression coverage in snapshot tests — v1.1 `.snap.styled` if needed.

## Next step

1. Implement Commit 1 (foundation).
2. Implement Commit 2 (medium screens).
3. Implement Commit 3 (snapshot test + final gate).
4. Manual smoke: launch TUI, navigate every tab, switch season via `y`, confirm
   dashboards re-render, confirm selection clamps, confirm transactions reset.
