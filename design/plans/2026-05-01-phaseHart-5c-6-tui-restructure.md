# Phase Hart.5c.6 — TUI App Restructure (v0.5, post-fourth-round-review)

**Status**: v0.5 — fourth 7-role review caught defects v0.4 introduced.
Two BLOCKERs were "documentation became fiction" (TAPE F1 + BENCH B-5 — I
documented an enum shape and a test file that didn't match reality);
two more (FORGE F1 + EDGE B1) caught D11 added incompletely. v0.5 closes
all 6 round-4 BLOCKERs and 16 FIXITs; spec is now backed by code or
explicitly deferred to a named successor plan.
**Date**: 2026-05-01
**Trophy**: Hart (sub-phase of 5c)
**Predecessor**: design/plans/2026-05-01-phaseHart-5c-final-cleanup.md (v0.3)
**Replaces**: nothing — sub-spec for the largest 5c sub-phase

---

## v0.2 → v0.3 changelog

Second 5-role review round. v0.2 patches verified clean for 35 of 36 v0.1
findings. The remaining issues are new defects v0.2 introduced via over-confident
claims (Risk #9 blanket safety, "1k iterations" understatement, D9 split
optimism). Resolutions:

### BLOCKERs resolved

- **tape NEW-1** — `schedule_team_cache: HashMap<String, _>` keyed by team only,
  but data is `(team, season)`-shaped. After season swap, `Screen::ScheduleTeam(EDM)`
  returns wrong-season schedule. v0.2's Risk #9 falsely claimed this cache was
  safe. **Resolution**: widen the cache key to `(String, Season)` as a 5c.6
  Commit 1 deliverable. Risk #9 replaced with per-cache verification table.

- **forge N3 + glass new-issue** — D9's 3-commit split breaks the build-green
  invariant. Once `App.players: Vec<Player>` flips to `repo: StatsRepository` in
  commit 1, 6 unmigrated screens (player/comps/queries/depth/misc/widgets) fail
  to compile. **Resolution**: collapse to 2 commits. Commit 1 = foundation +
  all-screens atomic (whole TUI builds in one pass). Commit 2 = snapshot test
  + final gate.

- **forge N2** — `LoadState`'s `Arc<Mutex<LoadInner>>` polling pattern won't
  compose with `!Send` repo. **Resolution**: D8 redesigned. Use `tokio::sync::mpsc`
  channel from `spawn_local` task to App's per-tick `try_recv()` poll. No shared
  state, no Mutex needed; fully single-threaded.

### v0.2 mediums + tape still-broken resolved

- forge N4: `skater_count` API doesn't exist. D5 sample uses
  `self.repo.skaters(season, ty).count()`.
- glass: `comps.rs` lines 34/39 added to migration row.
- glass: `depth.rs` lines 93/101/111 added to migration row.
- glass + bench: D9 collapse means commit 2's "doesn't include comps.rs" issue
  goes away — all screens migrate atomically in commit 1.
- glass: 8 omitted screens from snapshot list now have explicit rationale (no
  `app.players`/`app.goalies` reads).
- bench F4 partial → resolved: CI guardrail broadened to detect
  `{CI, GITHUB_ACTIONS, BUILDKITE, JENKINS_URL, GITLAB_CI, CIRCLECI}`.
- bench N5: `render_screen` signature extended with `ui_state` parameter; Search
  fixture documents the fixed query "mcdavid".
- pace NEW-1: D1's per-frame cost reframed honestly — `repo.skaters()` iterates
  the full `stats` HashMap with up to `LRU_CAP × N` entries when historical
  seasons are co-resident, filter-skipping non-matching windows. Still
  sub-millisecond at current scale; scale-threshold table updated.
- tape still-broken: `tx_search_mode` added to D5 invalidation list.
- tape NEW-4: D8 explicitly scopes `spawn_local` requirement to repo-bearing
  tasks; `schedule.rs`'s `tokio::spawn` is fine because it doesn't touch the
  repo.
- bench N4: arch doc updated to 7 Beniers asserts (added 50.0 giveaway-penalty).

### v0.2 NITs resolved

- forge: D8 boxing-escape sentence ("`Box<T>: Send` requires `T: Send`").
- forge: tokio current_thread runtime hint added to D8 code block.
- glass: Search query fixed value documented as fixture pin.
- bench: IceLines.md links Hart.5c.6 symmetrically with Hart.6.

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
    league_context: LeagueContext,
    league_context_window: (Season, SeasonType),  // window the ctx was built for
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
        // O(1) index lookup + O(roster_size ≈ 25) view materialization.
        // Last-stint roster only — see D10 for the all-stints variant.
        self.repo.team_roster(team, self.active_season, self.active_type)
    }
    pub fn team_views_all_stints(&self, team: &TeamAbbr) -> Vec<PlayerView<'_>> {
        // Includes any player who played for `team` at any point in
        // (active_season, active_type). Use this for the team page when
        // mid-season trades should remain visible on both rosters.
        self.repo.team_roster_all_stints(team, self.active_season, self.active_type)
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

### D10 — `team_views` stint shape per screen (HART v3 FIXIT-2)

**Problem**: `team_roster` returns last-stint roster; `team_roster_all_stints`
returns any-stint. Mid-season trade case: Bo Horvat 2022-23 shows on NYI under
`team_roster`, on both VAN and NYI under `team_roster_all_stints`. The v0.3
spec defaulted every `team_views` call to last-stint without naming the
decision per screen.

**Decision**:

| Screen / consumer | Variant | Why |
|---|---|---|
| `Screen::DepthTeam(team)` | `team_views` (last-stint) | Depth chart reflects current roster — a player traded in is on this team's lines, traded out is not. |
| `screens/team.rs` team page | `team_views` (last-stint) | Team page = current roster. Historical trades belong on player pages, not team pages. |
| Cross-team metrics (`compute_all_views`) | per-call iteration over `repo.skaters` | Operates on the full active-window skater set; not roster-scoped. |
| Future: "team season summary" | `team_views_all_stints` | If a screen ever needs full-season team production, it must opt in explicitly. |

D10 makes the default explicit (last-stint) and exposes the all-stints accessor
on App so future screens can opt in. Per-call site picks deliberately; no
silent stint shape changes.

### D11 — `LeagueContext` single-window invariant (HART v3 FIXIT-3)

**Problem**: `LeagueContext` keys percentile vectors by `Position` only. The
`dashboard_panel.compile` cache keys on `(PlayerId, Season, SeasonType)` and
sources percentile bars from `LeagueContext`. If a future caller computes a
panel for a non-active window (e.g. side-by-side season compare),
`LeagueContext` still holds the active-window vectors — bars would be stale by
axis.

**Decision**: pin the single-window invariant in `compile`. `LeagueContext` is
explicitly active-window-only; cross-window panel compilation is rejected at
the boundary with a clear error.

```rust
impl CompiledPanel {
    pub fn compile(
        &mut self,
        repo: &StatsRepository,
        season: Season,
        season_type: SeasonType,
        player_id: PlayerId,
        ctx: &LeagueContext,
        ctx_window: (Season, SeasonType),  // window the ctx was built for
    ) -> Result<&CompiledOutput, DashboardError> {
        if ctx_window != (season, season_type) {
            return Err(DashboardError::CrossWindowCompile {
                requested: (season, season_type),
                ctx_for: ctx_window,
            });
        }
        // ... unchanged
    }
}
```

Future side-by-side compare must build a per-window `LeagueContext` for each
side. The error variant is a forcing function: callers cannot accidentally
mix windows. Alternative (widening LeagueContext key to
`HashMap<(Season, SeasonType, Position), Vec<f64>>`) was rejected — it
permits cross-window mixing without surfacing the cost; the invariant is
better as a hard boundary.

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
    /// See D11 for the `ctx_window` argument and the cross-window
    /// rejection rule. The arguments below are the canonical signature;
    /// D11 elaborates the invariant only.
    pub fn compile(
        &mut self,
        repo: &StatsRepository,
        season: Season,
        season_type: SeasonType,
        player_id: PlayerId,
        ctx: &LeagueContext,
        ctx_window: (Season, SeasonType),
    ) -> Result<&CompiledOutput, DashboardError> {
        if ctx_window != (season, season_type) {
            return Err(DashboardError::CrossWindowCompile {
                requested: (season, season_type),
                ctx_for: ctx_window,
            });
        }
        let key = (player_id, season, season_type);
        if !self.cache.contains_key(&key) {
            let view = repo.view(player_id, season, season_type)
                .ok_or(DashboardError::PlayerNotInRepo)?;
            self.cache.insert(key, build_panel(&view, ctx)?);
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

Callsites (in App): every `compile` invocation passes
`(self.active_season, self.active_type, pid, &self.league_context,
self.league_context_window)`. The `league_context_window` field is updated
in lockstep with `league_context` itself — see D5 for the full
update sequence. Without that field, the cross-window check at the top of
`compile` is tautological (HART/EDGE v4 catch).

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
            // partial_cmp returns None only for NaN; pace_82() filters NaN
            // (BelowThreshold returns None, never f64::NAN), so Equal is
            // unreachable in practice. Defensive default keeps sort total.
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
    self.league_context_window = (self.active_season, self.active_type);  // D11
    self.dashboard_panel.clear_cache();
    if !outcome.missing.is_empty() {
        // KEEL v4 F3: missing-source banner. Don't drop the partial-fetch signal.
        self.status = format_missing_sources(&outcome.missing);
        self.status_expires = Instant::now() + Duration::from_secs(5);
    }
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
    self.league_context_window = (season, ty);  // D11 forcing function

    // 2b. Surface partial-fetch warnings from the new outcome to the user.
    //     poll_load drains outcome.missing into a status banner; here we
    //     only handle the synchronous reload path.
    if !outcome.missing.is_empty() {
        self.status = format_missing_sources(&outcome.missing);
        self.status_expires = Instant::now() + Duration::from_secs(5);
    }

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
    self.tx_search_mode = false;   // tape v1.1 still-broken: close any open search bar

    // 4. Playoff cursors — bracket data may be different shape.
    self.playoffs_round = 0;
    self.playoffs_series = 0;

    // 5. Selection state — clamp to new view set, don't reset.
    //    forge v1.1 N4: `skater_count` doesn't exist; use the iterator's count().
    let new_count = self.repo.skaters(season, ty).count();
    self.selected = self.selected.min(new_count.saturating_sub(1));

    // 6. Saved-query scroll — results computed against OLD repo's view set.
    self.query_result_scroll = 0;

    // 7. schedule_team_cache: NOT cleared here because Commit 1 widens its key
    //    to (String, Season). Pre-fix, the cache returns wrong-season schedules
    //    after a swap (tape v1.1 NEW-1). Post-fix, distinct seasons map to
    //    distinct entries; old entries can stay (LRU eventually evicts).

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
fn render_player(&mut self, frame: &mut Frame, pid: PlayerId) {
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

**Render-handler convention (HART v4 F1)**: `render_*` methods that mutate
`self.screen` / `self.status` / `self.status_expires` (auto-pop UX) take
`&mut self`. Pure-render handlers that only read App state and write to the
frame buffer take `&self`. The auto-pop path is the only mutating render
case in 5c.6; subsequent renders happen on the next event-loop tick after
the pop has been applied.

**Comps anchor missing case (HART v4 F2)**: `Screen::Comps(pid)` follows the
same pattern as `Screen::Player(pid)` — the renderer first calls
`self.repo.view(pid, s, t)` to resolve the anchor view; on `None` it
auto-pops to `parent_for(Screen::Comps(pid))` (the Players list) and emits
the same toast. The comps engine in `icelines-core` consumes
`&[PlayerView<'_>]` only after the anchor is resolved.

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

### D8 — `!Send + !Sync` constraint, load model, and `LoadState` redesign

**forge v1.0 B1 + v1.1 N2**: `StatsRepository` plants `PhantomData<*const ()>`
(`stats_repository.rs:81`) and asserts not Send, not Sync (`:84`). After 5c.6,
`LoadOutcome` (which contains a repo) is also `!Send`. Two cascading consequences
that the v0.2 spec partially missed:

1. `tokio::spawn(async move { ... })` requires `Send` futures — won't compile
   with a `LoadOutcome`-yielding task.
2. Today's `LoadState` is shared via `Arc<Mutex<LoadInner>>`. `Mutex<T>: Send`
   requires `T: Send`. After migration, `LoadInner` holds a `LoadOutcome` (and
   thus a `StatsRepository`), which is `!Send` — the entire `Arc<Mutex<...>>`
   pattern stops compiling.

**Decision (v0.3)**: redesign around `LocalSet` + a one-shot `mpsc` channel.
No shared state, no Mutex, fully single-threaded.

#### Alternatives considered

- `tokio::spawn` — **out**. Requires `Send` future; `LoadOutcome` is `!Send`.
- `tokio::task::spawn_blocking` — **out**. Return value `T` must be `Send`.
  Boxing doesn't help: `Box<T>: Send` requires `T: Send`. The constraint is
  the payload's marker traits, not the indirection.
- `tokio::task::spawn_local` + `tokio::task::LocalSet` — **chosen**.
  Single-threaded by construction; `!Send` is fine.
- Synchronous on UI thread — rejected; ~50 ms stall on cold load freezes the
  event loop and breaks live-scores polling.

#### `LoadState` shape post-redesign

```rust
pub enum LoadState {
    Idle,
    Loading,
    Loaded,                          // outcome already drained into App.repo
    Error(String),
}

// On the App, alongside LoadState:
load_rx: Option<tokio::sync::mpsc::UnboundedReceiver<LoadResult>>,

// Where:
type LoadResult = Result<LoadOutcome, String>;
```

The receiver is created when a load is spawned and dropped after the result is
drained. On every App tick:

```rust
fn poll_load(&mut self) {
    if let Some(rx) = self.load_rx.as_mut() {
        if let Ok(result) = rx.try_recv() {
            match result {
                Ok(outcome) => {
                    let _old = self.repo.repo_swap(outcome.repo);
                    self.league_context = LeagueContext::build(
                        &self.repo, self.active_season, self.active_type);
                    self.league_context_window =
                        (self.active_season, self.active_type);  // D11
                    self.dashboard_panel.clear_cache();
                    // KEEL v4 F3: surface MissingSource entries; never silent.
                    if !outcome.missing.is_empty() {
                        self.status = format_missing_sources(&outcome.missing);
                        self.status_expires =
                            Instant::now() + Duration::from_secs(5);
                    }
                    self.load_state = LoadState::Loaded;
                }
                Err(msg) => self.load_state = LoadState::Error(msg),
            }
            self.load_rx = None;
        }
    }
}
```

Spawn site:

```rust
fn spawn_load(&mut self, season: Season, ty: SeasonType) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    self.load_rx = Some(rx);
    self.load_state = LoadState::Loading;
    let snapshot_dir = self.config.snapshot_dir();
    tokio::task::spawn_local(async move {
        let store = SnapshotStore::new(&snapshot_dir);
        let result = load_into_repo(season, ty, &store).map_err(|e| e.to_string());
        let _ = tx.send(result);   // App may have moved on (rare); ignore.
    });
}
```

#### Bootstrap

```rust
// In main(): the TUI runtime must be current-thread + LocalSet.
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let local = tokio::task::LocalSet::new();
    local.run_until(async {
        let mut app = App::new(...);
        app.spawn_load(initial_season, initial_type);
        run_event_loop(&mut app).await
    }).await
}
```

`flavor = "current_thread"` is required: spawning multi-thread runtime + then
constraining to LocalSet works but is a foot-gun (`spawn_local` outside a
LocalSet panics). Pin it.

#### Scope of the constraint

`spawn_local` applies ONLY to tasks that touch the `StatsRepository`. The
existing `tokio::spawn` calls in `schedule.rs`, `tonight.rs`, etc., that fetch
boxscores, schedule data, and live scores remain `tokio::spawn` — those tasks
pass `String`/`u64`/`Vec<ScheduledGame>` (all `Send`). Don't blanket-convert.
A future refactor that closes those tasks over `app.repo` would silently break;
add a clippy lint (`-D non_send_in_local`) once that kind of refactor lands.

#### Background fetches and installs (today's flow)

Today, `tui/loader.rs` and `tui/screens/misc.rs` already use a similar
fire-and-forget pattern with `tokio::spawn` + `Arc<Mutex<...>>`. After 5c.6
those that produce a `LoadOutcome` switch to `spawn_local` + mpsc. Those that
produce non-repo data (`InstallPhase`, fetch progress) continue to use the
existing pattern with no changes.

### D9 — Implementation split (2 commits, atomic)

**v0.3 collapse**: v0.2 proposed 3 commits but commit 1's App field flip
(`Vec<Player>` → `repo: StatsRepository`) orphans 6 unmigrated screens that
still read `app.players` (player/comps/queries/depth/misc/widgets) — those
files fail to compile until commit 2 lands. Build-green invariant violated.

The cleaner fix is to make commit 1 atomic. The migration set is medium-large
but coherent (one concept: stop reading `app.players`). The shim alternative
(`#[deprecated] pub fn players(&self) -> Vec<Player>`) adds rotting code in
exchange for a smaller commit; not worth it.

```
Commit 1 (atomic foundation + all screens)
  Single coherent change. After this commit:
    • App owns repo: StatsRepository (D1)
    • LoadState mpsc shape (D8); LocalSet bootstrap pinned to current_thread
    • dashboard_panel.compile new signature with (PlayerId, Season, SeasonType) key (D2)
    • LeagueContext::build associated function (D3)
    • Loader returns LoadOutcome via mpsc (D4)
    • reload_for_season clears 8 fields atomically (D5)
    • Screen variants re-keyed on PlayerId (D6)
    • schedule_team_cache key widened to (String, Season) (tape v1.1 NEW-1)
    • All 14 affected screen files migrated to read views
    • widgets/mod.rs::player_cell_text signature flips to &PlayerView
    • Verification step: cargo build --workspace must succeed.

Commit 2 (snapshot test + final gate)
  tests/tui_snapshot.rs harness with 12 goldens.
  Final gate verification:
    grep -rn "app\.players\|app\.goalies" icelines-cli/src/tui/
  Must return zero hits to merge.
  cargo test --workspace must be green.
```

**Why this is OK as a single commit**: ~15 files, all touching the same
conceptual layer (TUI App + screens reading it). No cross-layer surgery (no
icelines-core changes; no icelines-fetch changes). Reviewable in one PR. The
snapshot test in commit 2 catches output regressions; manual smoke test
(launch TUI, navigate every tab, switch season via `y`) is the human gate
between commits 1 and 2.

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
| `screens/depth.rs` | `app.players` at lines 26, 31, **93, 101, 111**, 136, 166 (7 reads); `Vec<&Player>` references | per-frame views + `compute_all_views` + `compute_team_strength_views`. High because Vec<&Player> won't compile after 5c.7. | **High** |
| `screens/player.rs` | `app.players[idx]` from `Screen::Player(idx)` | `app.repo.view(pid, s, t)` from `Screen::Player(pid)`; auto-pop on missing per D6 | Medium |
| `screens/comps.rs` | `app.players` at lines 34, 39 | view-based comps; `Screen::Comps(PlayerId)` per D6 | Medium |
| `tui/schedule.rs` | (no `app.players` read; cache key fix) | widen `schedule_team_cache: HashMap<String, _>` → `HashMap<(String, Season), _>` and update `maybe_fetch_team` callsite to key by both. tape v1.1 NEW-1. | Low |
| `screens/goalies.rs` | `app.goalies` for leaderboard | `app.goalie_views()` | Low |
| `screens/queries.rs` | `app.players` + saved query spec at lines 282–287 | `apply_views(views)` + sort; preserves saved-query JSON shape | Medium |
| `screens/search.rs` | `app.players` substring match | `views.find(name_normalized.contains(q))` | Low |
| `screens/schedule.rs` | NHL schedule (independent) | unchanged | None |
| `screens/playoffs.rs` | Playoffs bundle (independent) | unchanged | None |
| `screens/transactions.rs` | `app.transactions` (independent of player views) | unchanged in 5c.6; `app.transactions` reset by D5 | Low |
| `screens/game_detail.rs` | NHL boxscore (independent) | unchanged | None |
| `screens/misc.rs::render_projections` | `app.players` at lines 327, 337 | per-frame views + sort | Medium |
| `screens/misc.rs::render_groups`/`render_group_members` | `app.players` at line 480 | substring match on views | Low |
| `screens/misc.rs::status_footer` | `app.players` at lines 617, 620 (`{} players loaded`) | `app.repo.skaters(s, t).count()` (no `skater_count` API exists; D5 / forge v1.1 N4) | Low |
| `screens/mod.rs` | dispatch | screen variants update for D6 | Low |

Net: **3 high-complexity files** (`app.rs`, `screens/depth.rs`, plus possibly
the snapshot harness as a fourth standalone), ~7 medium, ~9 low, ~5 untouched.

---

## L2 TUI snapshot test (required deliverable)

Per Hart.5c v0.3, `icelines-cli/tests/tui_snapshot.rs` lands using
`ratatui::backend::TestBackend`.

### Pinned signature

```rust
/// UI state passed alongside the screen to make non-default goldens
/// reproducible (search queries, list cursors, query field state).
/// Default-constructible via UiState::default() — covers most goldens.
pub struct UiState {
    pub search_query: String,            // for Screen::Search
    pub selected:     usize,             // for Players/Goalies/Transactions list rows
    pub query_fields: Vec<QueryField>,   // for Screen::Queries; default = empty
    pub depth_mode:   ScoringMode,       // for Screen::Depth (Pace vs Fantasy)
}

fn render_screen(
    repo:        &StatsRepository,
    season:      Season,
    season_type: SeasonType,
    screen:      Screen,
    ui:          &UiState,
) -> ratatui::buffer::Buffer
```

bench v1.1 N5: v0.2 signature didn't take UI state, leaving `Screen::Search`'s
query origin and list-screen selection cursors unspecified. v0.3 adds an
explicit `UiState` parameter. Most goldens use `UiState::default()`; the Search
golden pins `search_query: "mcdavid".to_string()` in its test body.

### Fixture and goldens

**Fixture repo**: bundled current season (`CURRENT_SEASON`, `Regular`). Loads
deterministically from the binary's embedded data. ~1000 skater + ~70 goalie
records. No live network, no snapshot dependency. Reuses
`integration_phase2.rs::fixture_repo` pattern (per bench N3) — pure-data,
calls `repo.upsert_identity` + `repo.upsert_stats` against
`StatsRepository::new()`.

**Search query is fixture-pinned**: the `Screen::Search` golden uses the
constant `"mcdavid"` because it stably matches "Connor McDavid" in the bundled
season. If McDavid ever leaves the league or his name normalizes differently,
the golden fails — that's an intentional canary for naming changes, not a
fragility.

**Snapshot determinism (bench v3)**: `StatsRepository.stats` is a `HashMap`
with non-deterministic iteration order. Every screen renderer that consumes
`repo.skaters(s, t)` MUST sort the result before rendering. Required sort
keys per snapshotted screen:

| Screen | Sort key | Tiebreak |
|---|---|---|
| `Home`        | pace_82 desc, **None last** | full_name asc |
| `Players`     | pace_82 desc, **None last** | full_name asc |
| `Projections` | pace_82 desc, **None last** | full_name asc — same shape as Players (BENCH v4 B-1) |
| `Depth`       | team asc, position asc, pace_82 desc (None last) | full_name asc |
| `DepthTeam(t)`| line slot asc | full_name asc |
| `Goalies`     | goalie.gaa asc, None last | full_name asc |
| `Search`      | name_normalized.contains(q), then pace_82 desc (None last) | full_name asc |

**None-last clause (PACE v4 F1)**: `view.pace_82()` returns `Option<f64>`
(`None` for `gp_status == BelowThreshold` per A4). Default `Option<f64>`
ordering puts `None` first under `desc`. Renderers MUST partition into
`Some` / `None` groups, sort `Some` group by inner f64 desc, then
concatenate Some-then-None. Equivalent: sort key
`(pace_82.is_none(), Reverse(pace_82.unwrap_or(f64::NEG_INFINITY)))`.

Stats tab default sub-view is **Projections**, not Players (per
`docs/guides/06-tui.md:85-90`). The Projections golden uses the same sort
shape as Players, so a single golden covers both. If Projections gets a
bespoke layout in a follow-up, it earns its own golden then.

Any unsorted iteration over `repo.skaters` in renderer code is a snapshot
flake source and a CI hard-block target.

**Identity round-trip carry-forward (tape v3 + bench v3)**: Hart.5c.6 doesn't
touch identity loading, but the Slafkovský diacritic round-trip assert at
`icelines-fetch/tests/stats_loader.rs::l1_player_view_accessors_against_real_bundled_data`
(diacritic block at lines 512-527 — finds any non-ASCII player name and
asserts diacritic preservation + name_normalized ASCII-strip) is required to
remain green throughout the migration. v0.4 referenced a non-existent
`mock_nhl_api_loader.rs` (TAPE v4 F2 catch); v0.5 points at the real test.

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

**Screens deliberately not snapshotted** (glass v1.1 + bench v1.1):
- `Tonight` / `GameDetail` — independent of `app.players` / `app.goalies`
- `Projections` (sub-view of Stats tab — covered by `Players` golden)
- `Groups` / `GroupDetail` — fully independent of player data spine
- `ScheduleTeam(team)` / `ScheduleMatchup` / `SeriesDetail` — schedule data
  spine, no app.players read
- `Fetch` / admin overlay (covered indirectly via misc.rs's status footer)

These are skipped because they don't read `app.players` or `app.goalies`. Their
data path is unchanged in 5c.6, so snapshot regressions are out-of-scope for
this commit. They get goldens in a follow-up if the screens themselves change.

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

**CI guardrail (bench v1.1 F4)**: if `INSIDE_GOLDEN_UPDATE` is set AND any of
`{CI, GITHUB_ACTIONS, BUILDKITE, JENKINS_URL, GITLAB_CI, CIRCLECI}` is set, the
test harness errors out. Prevents accidental green runs on any major CI vendor
that overwrite goldens. v0.2 only checked `CI=true`, which is unset on Docker
devcontainers, self-hosted runners, and some vendors — silently bypassed the
guard. The `any-of` set above covers ~95% of CI surfaces; for the rest, set
`CI=true` manually in the runner config.

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
| `dashboard_panel.rs` test mod | new compile signature, (PlayerId, Season, SeasonType) cache key, plus D11 cross-window rejection assert (compile() with `ctx_window != (s,t)` returns CrossWindowCompile error) | |
| `tui/schedule.rs` test mod | NEW: two-season `schedule_team_cache` regression — fetch EDM @ 20242025 then EDM @ 20252026, assert two distinct entries (bench v3 FIXIT) | |
| `icelines-fetch/tests/stats_loader.rs::l1_player_view_accessors_against_real_bundled_data` (existing, diacritic block at lines 512-527) | unchanged in 5c.6; must remain green throughout the migration (L1 hard-block gate) | |
| `icelines-core/src/stats_repository.rs` test mod | NEW: `team_views_all_stints` returns Bo Horvat (or any 2-stint synthetic fixture) on both pre-trade and post-trade team for the same (season, type); compared against `team_roster` returning him on last-stint team only (EDGE v4 F1) | |
| `icelines-core/src/fixtures.rs` test mod | NEW: `upsert_stats_rejects_empty_team_stints` — upserting a SeasonStats with `team_stints: vec![]` returns `RepoError::EmptyStints`. Loader-level WARN+skip path tested via mocked SeasonStats with empty stints (HART v4 F3 + EDGE v4 F2) | |
| `icelines-cli/tests/data_install.rs` (existing or NEW) | concurrent `data install` collision: two install processes targeting same season — second loses cleanly with a "season already installing" error or "last-writer wins, prior result discarded" outcome (EDGE v4 F3) | |

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

6. **Per-frame view collection + cross-team cost** — pace v1.1 NEW-1 reframe:
   `repo.skaters(s, t)` iterates the full `stats: HashMap<(PlayerId, Season,
   SeasonType), SeasonStats>` and filter-skips non-matching `(s, t)` windows,
   plus an `identities` and `contracts` HashMap probe per match. Worst-case
   work after multi-season time-travel is `LRU_CAP × N` ≈ 8 × 1000 = 8k
   iterations + 2k probes. Note that LRU_CAP=8 windows can be 4 seasons × 2
   types (Regular + Playoff) — a single "season" can contribute two windows
   to the resident set after Hart.6 lands. Still sub-millisecond at current scale (estimated;
   unmeasured). The architectural cost is on `Screen::Depth`'s
   `compute_all_views` (~320k comparisons, <1 ms estimated); the per-frame
   collect itself is cheaper. At N=10k active, multi-season-resident:
   ~80k iterations + 160k probes (still <5 ms estimated) plus
   compute_all_views ~32M ops (~30 ms — perceptible). If scale grows, cache
   `compute_all_views` result on `Screen::Depth` keyed by `(season, type,
   mode)`; invalidate on `repo_swap`. Not blocking 5c.6.

7. **`!Send + !Sync` constraint** — handled by D8's `spawn_local + LocalSet`.
   Foreseeable failure: someone adds a new background task with `tokio::spawn`
   in the future. Mitigation: pin a CI clippy lint (`-D non_send_in_local`)
   if needed; for now, code review.

8. **Per-screen migration error surface** — 14 files, 7+ medium-complexity
   migrations. High blast radius for a single atomic commit (D9). Mitigation:
   TUI snapshot test catches output regressions; manual smoke test between
   Commit 1 (atomic foundation + screens) and Commit 2 (snapshot test +
   final gate).

9. **App caches that survive `repo_swap`** — verified per cache (tape v1.1
   NEW-1 caught a blanket "verified safe" claim that was wrong for one):

   | Cache | Key shape | Season-coupled? | Action |
   |---|---|---|---|
   | `tonight_cache` | `HashMap<String, _>` keyed by date `YYYY-MM-DD` | No (date is absolute) | safe |
   | `boxscore_cache` | `HashMap<u64, _>` keyed by `game_id` | No (game IDs are unique league-wide) | safe |
   | `schedule_week_cache` | `HashMap<String, _>` keyed by Monday date | No (date is absolute) | safe |
   | `schedule_team_cache` | `HashMap<String, _>` keyed by team only | **YES — fixed in Commit 1** | widen key to `(String, Season)` |
   | `playoffs_cache` | `HashMap<u16, _>` keyed by playoff_year | No (year derived from season → distinct entries) | safe |
   | `headshot_cache` | `HashMap<u32, _>` keyed by nhl_id | No (player ID stable across seasons) | survives swap intentionally — see note below |

   Only `schedule_team_cache` was unsafe; v0.3 fixes the cache key shape.

   **`headshot_cache` cross-correlation (keel v3 BLOCKER)**: post-swap, an
   `nhl_id` can have a cached headshot but no `PlayerIdentity` in the new repo
   (e.g., 1993-94 player with cached headshot is no longer in the 2025-26
   repo). The cache survives the swap intentionally — headshots are stable
   per nhl_id across all seasons, and pre-fetched bytes shouldn't be
   discarded. The D6 auto-pop UX (`Screen::Player(missing_pid)` →
   parent list + 2-second toast) is the load-bearing mitigation: the user
   never reaches a card-render path with cached headshot but no identity. Do
   not add identity-presence checks to the headshot fetch path; rely on D6.

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
