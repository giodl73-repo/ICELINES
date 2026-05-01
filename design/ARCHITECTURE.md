# IceLines — System Architecture

**Version**: 2.1
**Date**: 2026-05-01
**Status**: Active — replaces the v1 (pre-Hart) architecture doc.
v2.0 → v2.1 incorporates 5-role review (forge / tape / glass / bench / pace).

---

## Mission

A single-user, local-only NHL hockey analytics + fantasy tool. Four surfaces — TUI,
CLI, mkdocs site, HTTP server — all driven by one data engine and one normalized
domain model.

The architectural invariant: **for the same data state, every surface produces the
same output.** The TUI's depth chart, the CLI's `team EDM`, the site's team page,
and the HTTP server's `/api/team/EDM/roster` all call the same `StatsRepository`
and the same `DepthChartBuilder::build_views`. Renderer differences are cosmetic;
data and computation paths converge.

---

## High-level shape

```
                      ┌────────────────────────────────────────┐
                      │  ~/.icelines/config.toml               │
                      │  (season, snapshot_dir, no_live, ...)  │
                      └─────────────┬──────────────────────────┘
                                    │
   ┌─────────────┬──────────────────┼───────────────────┬─────────────┐
   │             │                  │                   │             │
   ▼             ▼                  ▼                   ▼             ▼
┌──────┐    ┌──────────┐      ┌──────────┐        ┌──────────┐  ┌────────┐
│ TUI  │    │ CLI cmds │      │ axum HTTP│        │ mkdocs   │  │ export │
│(rata-│    │ (28 top- │      │ server   │        │ site     │  │ md     │
│ tui) │    │ level)   │      │(fantasy  │        │(build/   │  │        │
│      │    │          │      │ serve)   │        │ serve/   │  │        │
│      │    │          │      │          │        │ deploy)  │  │        │
└──┬───┘    └────┬─────┘      └────┬─────┘        └────┬─────┘  └───┬────┘
   │             │                 │                   │            │
   └─────────────┴────────┬────────┴───────────────────┴────────────┘
                          │  every surface calls into ↓
                          ▼
        ┌─────────────────────────────────────────────────────┐
        │              icelines-core (engine)                  │
        │   StatsRepository  ◄──  PlayerView<'_>               │
        │   PlayerIdentity   ─►   SeasonStats {totals, ...}    │
        │   key: (player_id, season, season_type)              │
        │                                                       │
        │   Pure logic: filter, scoring, projections,          │
        │   depth chart, cross-team metrics, scheme,           │
        │   history, name normalization                        │
        └─────────────────────┬───────────────────────────────┘
                              │ populated by ↓
                              ▼
        ┌──────────────────────────────────────────────────────┐
        │   icelines-fetch::stats_loader::load_into_repo       │
        │   (season, season_type, &SnapshotStore)              │
        │                  │                                   │
        │                  ▼                                   │
        │    Source-by-source fallback (NO single chain):      │
        │    ┌──────────────────────────────────────────┐      │
        │    │ bios + skater stats:                     │      │
        │    │   chunked snapshot → legacy snapshot     │      │
        │    │   → embedded (include_bytes!)            │      │
        │    │                                          │      │
        │    │ goalie stats:                            │      │
        │    │   legacy snapshot → embedded             │      │
        │    │   → installed bundle                     │      │
        │    │                                          │      │
        │    │ transactions:                            │      │
        │    │   legacy snapshot → embedded             │      │
        │    │   → installed bundle                     │      │
        │    │                                          │      │
        │    │ playoffs (bracket):                      │      │
        │    │   installed bundle → embedded            │      │
        │    │                                          │      │
        │    │ realtime / moneypuck / contracts:        │      │
        │    │   snapshot tier ONLY                     │      │
        │    │   absent → MissingSource flag set        │      │
        │    └──────────────────────────────────────────┘      │
        │                                                      │
        │    Live NHL API is NOT a query-time tier — it is     │
        │    the WRITE path for `icelines fetch *`, which      │
        │    lands in the snapshot tier. Queries never fall    │
        │    through to live.                                  │
        └──────────────────────────────────────────────────────┘
```

---

## The data spine in detail

```
┌────────────────────── icelines-core::stats_repository ──────────────────────────┐
│                                                                                  │
│   PlayerIdentity            SeasonStats                                          │
│   ─────────────             ──────────                                           │
│   id: PlayerId       ───►   player_id, season, season_type, position             │
│   full_name                 sweater_number: Option<u32>                          │
│   name_normalized           totals: StatTotals                                   │
│   bio: PlayerBio            team_stints: Vec<TeamStint>  ← traded players kept   │
│   headshot_canonical_url    realtime: Option<RealtimeStats>  ← cold-start = None│
│                             advanced: Option<AdvancedStats> ← MoneyPuck silo'd  │
│                             goalie:   Option<GoalieSeasonStats>                  │
│                                                                                  │
│   StatsRepository                                                                │
│   ─ Internal storage: HashMap-indexed by primary keys                            │
│   ─ LRU cap fires only under memory pressure (never at current scale)            │
│   ─ !Send + !Sync (PhantomData<*const ()>) — single-threaded by construction.   │
│     Background loads must use spawn_local + LocalSet, NOT tokio::spawn.          │
│                                                                                  │
│        ├── upsert_identity(PlayerIdentity) -> Result<(), RepoError>              │
│        ├── upsert_stats(SeasonStats)       -> Result<(), RepoError>              │
│        ├── upsert_contract(PlayerId, PlayerContract)                             │
│        ├── repo_swap(new) -> StatsRepository                                     │
│        │       ↑ returns OLD repo (mem::replace); atomic, borrow-checked         │
│        │                                                                         │
│        ├── view(pid, s, t)  -> Option<PlayerView<'_>>                            │
│        ├── skaters(s, t)    -> impl Iterator<Item = PlayerView<'_>>              │
│        ├── goalies(s, t)    -> impl Iterator<Item = PlayerView<'_>>              │
│        ├── team_roster(team, s, t)             -> Vec<PlayerView<'_>>           │
│        │       ↑ O(1) hashmap lookup via rosters_last_stint index               │
│        └── team_roster_all_stints(team, s, t)  -> Vec<PlayerView<'_>>           │
│                ↑ O(1) hashmap lookup via rosters_all_stints index               │
│                                                                                  │
│   PlayerView<'a>            ← borrowed projection over (identity, stats, ...)    │
│   ─────────────                                                                  │
│   identity: &PlayerIdentity                                                      │
│   stats:    &SeasonStats                                                         │
│   contract: Option<&PlayerContract>                                              │
│                                                                                  │
│   accessor methods: pace_82(), goals_per_82(), gp(), shots(), hits(), team(),    │
│   plus_minus(), toi_mmss(), contract_expiry_year(), is_rankable(), ...          │
└──────────────────────────────────────────────────────────────────────────────────┘
```

Per-Hart-phase invariant: **`(player_id, season, season_type)` is the primary key
axis.** Pre-Hart, the data was implicitly current-season-regular-only. Hart makes
historical + playoff queries possible without schema gymnastics.

---

## Surfaces

```
┌─ TUI (full-screen) ───────────────────┐  ┌─ CLI commands (28 of them) ──────┐
│                                        │  │                                  │
│  App {                                 │  │  fn run(args) -> Result {        │
│    repo: StatsRepository,    ◄────────┼──┤    let (outcome, season) =       │
│    active_season: Season,              │  │        load_repo_for_season();   │
│    active_type:   SeasonType,          │  │    let views = outcome.repo      │
│    ... per-screen UI state             │  │        .skaters(season, type)    │
│  }                                     │  │        .collect();               │
│                                        │  │    // filter / sort / render     │
│  StatsRepository is !Send + !Sync.     │  │  }                               │
│  Background loads use spawn_local +    │  │                                  │
│  LocalSet, NOT tokio::spawn.           │  │  one-shot: load → use → drop     │
│                                        │  │                                  │
│  per render frame:                     │  └──────────────────────────────────┘
│    let views = app.repo.skaters(...);  │
│    screen.render(&views, ...);         │  ┌─ axum HTTP server ──────────────┐
│                                        │  │  (icelines fantasy serve)       │
│  season switch (`y` key):              │  │                                 │
│    new_repo = load_into_repo(...);     │  │  per request handler:           │
│    let _old = self.repo.repo_swap(new);│  │    let (outcome, s) = load();   │
│    dashboard_panel.clear_cache();      │  │    let views = pools_views(...);│
│    league_context = rebuild(&repo);    │  │    score_team(...) -> JSON      │
│                                        │  │  per request handler:           │
└────────────────────────────────────────┘  │    let (outcome, s) = load();   │
                                            │    let views = pools_views(...);│
┌─ mkdocs site builder ─────────────────┐  │    score_team(...) -> JSON      │
│  (icelines build / serve / deploy)    │  │                                 │
│                                        │  │  reads SQLite (fantasy_db)      │
│  one shot:                             │  │  for league/team/roster state. │
│    load_into_repo(current_season)      │  │                                 │
│    sort_views_by_pace                  │  └─────────────────────────────────┘
│    compute_all_views (cross-team)      │
│    for each team:                      │  ┌─ export md (markdown table) ────┐
│      DepthChartBuilder::build_views    │  │                                 │
│      render_team_page → docs/teams/X.md│  │  same load + view path; renders │
│    rewrite mkdocs.yml nav              │  │  GitHub-flavored markdown +     │
│    write index.md                      │  │  YAML front-matter to           │
│                                        │  │  ~/.icelines/reports/{shape}.md │
│  Output: docs/ (deterministic)         │  │                                 │
└────────────────────────────────────────┘  └─────────────────────────────────┘
```

Only the TUI is long-lived — its `App` holds the repo across many render frames.
CLI / site / HTTP-handler are one-shot: load, use, drop. This shapes the cache
invalidation contract: only the TUI needs to think about cache invalidation on
season switch.

---

## Persistence layers (where state lives)

```
~/.icelines/                         ← user data root
│
├── config.toml                      ← season, snapshot_dir, dashboards, no_live
│
├── snapshots/                       ← live-fetched data (icelines fetch)
│   ├── manifest.toml                  active snapshot per tier
│   ├── chunks/                        content-addressed object store (Phase 8h)
│   │   ├── ab/ab1f5c…d7e2              one player record per chunk
│   │   └── ...
│   ├── chunkrefs.json                 refcount table for GC
│   ├── 20252026-2026-04-25-stats/
│   │   ├── snapshot.json               sealed metadata + integrity hashes
│   │   ├── chunked.json                player_id → chunk hash mapping
│   │   ├── stats/{bios,stats}.json     legacy file-per-tier (still supported)
│   │   ├── realtime/realtime.json
│   │   ├── moneypuck/moneypuck.json
│   │   ├── contracts/contracts.json
│   │   └── stats/transactions.json
│   └── ...
│
├── seasons/                         ← installed historical bundles
│   ├── 19931994/bundle-19931994/       tarball-extracted (data install)
│   │   ├── bios.json
│   │   ├── stats.json
│   │   ├── goalie-stats.json
│   │   ├── playoffs.json               bracket + game log (not per-player stats)
│   │   └── transactions.json
│   └── manifest.json                   list of installed season IDs
│
├── icelines.db                      ← SQLite, shared across features
│   ├── groups, group_members          (group-management)
│   ├── saved_queries                  (TUI saved queries)
│   ├── fl_leagues, fl_teams, fl_roster (fantasy-leagues)
│   └── games_attended                 (icelines games command)
│
├── schemes/                         ← user-authored fantasy schemes
│   ├── my-league.toml                  (user wins over built-in same-name)
│   └── ...
│
└── reports/                         ← export md output
    ├── leaders.md
    ├── team-EDM.md
    └── ...

(in-binary, not on disk)
icelines-fetch/src/bundled.rs        ← 5 seasons embedded
                                       via include_bytes!() at compile time
                                       (~4.3 MB binary growth)

(external, optional)
api.nhle.com / api-web.nhle.com      ← live (icelines fetch *)
moneypuck.com (CSV)                  ← optional silo
site.api.espn.com (transactions)     ← optional silo
```

---

## Query model: scale + indices

```
Current data scale (per loaded season):
  ~960  active skaters (current season, 32 teams × ~30 each)
  ~70   goalies
  ~30   contracts per fantasy league

Aggregate (5 bundled seasons in memory):
  ~4,800 skater records
  ~350   goalie records
  ~5 MB  total memory footprint when fully loaded

Historical (38 seasons via data install):
  ~38,000 skater records max if all installed
  Loaded one-at-a-time on season switch — never all in memory
```

At this scale, **the answer is mostly on-the-fly**. Numbers in this section are
estimates against current scale (N ≈ 1000); no `criterion` benchmarks exist as of
2026-05-01. If/when scale grows, run `cargo bench` before re-tuning. ratatui is
event-driven — a frame redraws only when an event fires (key, resize, or the
`tui/mod.rs:117` 100 ms poll tick). Effective worst-case is ~10 fps on a held-
key; not a fixed 60 fps render loop.

That said, there are real indices and caches. Inventory:

### What IS indexed (by-key O(1) lookup)

```
StatsRepository internal HashMaps:
  identities:           HashMap<PlayerId, PlayerIdentity>                    O(1)
  stats:                HashMap<(PlayerId, Season, SeasonType), SeasonStats> O(1)
  contracts:            HashMap<PlayerId, PlayerContract>                    O(1)
  rosters_last_stint:   HashMap<(Season, SeasonType, TeamAbbr), Vec<PlayerId>> O(1)
  rosters_all_stints:   HashMap<(Season, SeasonType, TeamAbbr), Vec<PlayerId>> O(1)

So `team_roster` and `team_roster_all_stints` are O(1) hashmap lookup +
O(roster_size ≈ 25) view materialization. Indexes are rebuilt incrementally on
every `upsert_stats`.

Plus an LRU bidirectional bijection layer for memory bound (fires only under
pressure; in practice never at current scale).
```

### What IS scanned (linear over the season's view set)

```
Every other operation:

  PlayerFilter::apply_views(views)         O(N)  filter by 19 fields
  sort_views_by_pace(&mut views)           O(N log N)
  compute_all_views(views)                 outer O(N); inner O(T·K)
                                            where T = teams (~32), K = max
                                            position bucket (~10).
                                            Total ≈ N·T·K ≈ 320k comparisons.
                                            Estimated <1 ms; unmeasured.
  fuzzy_find_view_in(views, query)         O(N)  substring match on name
  position_percentile(all, target, metric) O(N) per call
```

### What IS cached (computed once, invalidated explicitly)

```
TUI App (per Hart.5c.6 design):

  dashboard_panel: CompiledPanel
      contents: per-player scout-card sparkline + percentile bars
      built:    lazily on player-card open
      cleared:  on season switch (reload_for_season → clear_cache)

  league_context: LeagueContext
      contents: per-position pace_82 sorted vectors (for percentile lookups)
      built:    after initial load, after season switch
      cleared:  on season switch (rebuilt from new repo)

Site builder, CLI commands, HTTP handlers (one-shot, process exits or per-request):
  No long-lived caches. Reload outcome on each invocation.
```

### Why this is sound

```
N (per season) ≈ 1,000 records.
Filter + sort + render at this N is sub-millisecond (estimated; unmeasured).

The "real" cost is:
  1. Loading the repo (disk I/O + JSON parse) — ~50 ms cold (estimated)
  2. compute_all_views cross-team ranking — ~320k comparisons (estimated <1 ms)
  3. Headshot fetch (network) — ~200 ms async

None of these are query-time costs. They're load-time costs, amortized once per
season switch.

Adding indices for filter axes (position, age, nationality) would:
  • Save microseconds per query
  • Add invalidation complexity on repo_swap
  • Bloat memory by ~2× per axis
  • Buy nothing observable

Architectural rule (post-Hart): by-key lookups are O(1) (HashMap-indexed in the
repo, including team_roster). Everything else is on-the-fly over per-season view
iterators. Caches exist only where re-computation is genuinely expensive
(`league_context`'s sorted-by-position pace vectors) or genuinely user-perceived
(`dashboard_panel`'s sparkline rendering).

Scale thresholds for revisiting (per operation):
  • compute_all_views — first hot path: at N=10k, ~32M ops (~30 ms perceptible)
  • filter + sort       — stays sub-5ms until N≈100k
  • team_roster        — already O(1); scales freely
  • Per-frame collect   — at N=10k, ~50 µs (still imperceptible)

If scale grows past ~10× (e.g., per-game shift logs at ~100k rows),
`compute_all_views` becomes the first thing to cache (per active screen, keyed by
`(season, type, mode)`). Filter/sort can remain on-the-fly considerably longer.
Add an index per operation, not globally.
```

---

## Cross-cutting concerns

### Configuration

```
Config::load() reads in precedence order:
  1. CLI flags (--season, --no-live, --no-dashboards)
  2. ICELINES_* env vars (ICELINES_NO_LIVE, ICELINES_DASHBOARDS)
  3. ~/.icelines/config.toml
  4. defaults

Keys: season, snapshot_dir, no_live, dashboards
```

### Error model

```
Library crates:           thiserror enums, never panic in production paths
  icelines_core::IcelinesError
  icelines_core::stats_repository::RepoError      ← upsert violations, LRU
  icelines_core::identity::IdentityMergeError     ← name_normalized conflicts
  icelines_fetch::FetchError
  icelines_fetch::LoadError                       ← #[from] RepoError
  icelines_fetch::SnapshotError
  icelines_site::SiteError

CLI binary (icelines-cli): anyhow::Error
  Wraps + provides user-facing context strings.
  Translates "no contracts data" to "Run icelines fetch contracts."
```

### Time-travel as a primary axis (post-Hart)

```
active_season: Season       ── TUI App field
active_type:   SeasonType   ── --season flag (CLI), per-call

Every load_into_repo / repo.skaters / view / team_roster takes both.
Time-travel = repo_swap to a freshly loaded repo for new (s, t).
Caches that depend on (s, t) (dashboard_panel, league_context) clear on swap.
```

### Test strategy (1,020 tests as of 2026-05-01)

```
L0 unit (~308 core lib, ~315 cli main)
   inline #[cfg(test)], pure logic, microseconds

L1 integration (~140 fetch lib + ~115 across 7 fetch integration files = ~255)
   integration_phase2.rs (17), integration_pipeline.rs (10), mock_nhl_api.rs (35),
   stats_loader.rs (22), transactions_storage.rs (17), transactions_mock.rs (10),
   transactions_fixture.rs (4)
   StatsRepository + PlayerView fixtures, no live network, httpmock for NHL API
   integration_phase2.rs preserves 6 Beniers known-value asserts:
     179.0 / 50.0 / 130.0 / 122.0 / 195.0 / 440.0
   across yahoo / espn / simple / custom scheme variants

L2 system (~140 in cli tests/system_tests.rs + 1 proof_lib_smoke.rs)
   subprocess invocation; covers every top-level command

cargo test --workspace runs all tiers; CI gates green on every commit.
```

---

## Crate dependency DAG

```
┌────────────────────────────────────────────────────┐
│                  icelines-cli                       │
│  src/main.rs · 28 commands · TUI · axum · render   │
│  Depends on all three. No business logic — only    │
│  argument parsing, screen rendering, I/O dispatch. │
└─────────────────────┬──────────────────────────────┘
                      │
        ┌─────────────┴──────────────┐
        ▼                            ▼
┌──────────────────┐         ┌──────────────────┐
│  icelines-site   │         │  icelines-fetch  │
│  (mkdocs render) │         │  (NHL API + I/O) │
└────────┬─────────┘         └────────┬─────────┘
         │                            │
         └────────────┬───────────────┘
                      ▼
        ┌──────────────────────────┐
        │     icelines-core         │
        │  (pure logic: model,      │
        │   filter, scoring,        │
        │   projections, depth      │
        │   chart, scheme, ...)     │
        │  No I/O. No async.        │
        └───────────────────────────┘
```

**Where to add new code**:

| New thing | Crate | Why |
|---|---|---|
| Data type, scoring, projection, filter | `icelines-core` | Pure logic |
| NHL API endpoint, snapshot, bundled data | `icelines-fetch` | I/O |
| mkdocs / markdown generation | `icelines-site` | Site only |
| CLI command, TUI screen, HTTP handler | `icelines-cli` | Thin UI |

Business logic must NOT live in `icelines-cli`. CLI commands call library functions;
they don't compute.

---

## Cold-start lifecycle

```
1. cargo install icelines  (or download pre-built binary)
   Binary contains 5 bundled seasons (~4.3 MB).

2. icelines query leaders --top 10
   - Config::load() finds no config → defaults
   - load_repo_for_season(None) → CURRENT_SEASON
   - load_into_repo: snapshot empty → falls through to bundled tier
   - repo populated from include_bytes!() bytes
   - view path renders → output

3. icelines fetch all
   - hits NHL API, writes to ~/.icelines/snapshots/<new-active>/
   - subsequent loads prefer the snapshot over bundled

4. icelines data install 19931994 (optional)
   - downloads tarball from GitHub Releases
   - extracts to ~/.icelines/seasons/19931994/
   - `y` in TUI lists it as installed
   - Note: today's `load_into_repo` only falls back to embedded for bios/stats,
     not to ~/.icelines/seasons/. Goalies, transactions, and playoffs DO have
     installed-bundle fallback. This asymmetry is a known gap (filed under
     plans/INDEX.md backlog "uniform installed-bundle fallback").

5. icelines tui
   - launches ratatui, App owns repo
   - `y` season picker uses listed seasons
   - `F` admin overlay shows install status
```

---

## What this clarifies

- **Why Hart matters**: pre-Hart, `(season, type)` wasn't a key — every load was
  implicitly `current` + `Regular`. Hart makes it the primary axis, which is what
  makes time-travel + playoff data + multi-source coexistence possible.
- **Why TUI 5c.6 is the hard piece**: the App is the only surface that holds the
  repo across many render frames. CLI / site / HTTP server all do one-shot loads.
  TUI needs `repo_swap` + cache invalidation on season switch.
- **Why "single user, local only" is in non-goals everywhere**: zero auth, zero
  cloud, zero multi-tenant. Every spec assumes one user, one machine, one
  `~/.icelines/`.
- **Why Tier 4-6 in `data-sources.md` never landed**: the architecture has slots
  for them (MoneyPuck silo, ESPN silo are real), but Natural Stat Trick scraping /
  social signals / RSS were never wired through `load_into_repo`.

---

## Pointers

- **What + who + where**: `IceLines.md` (app plan).
- **How**: this document.
- **Features**: `design/specs/*.md` — 30 feature specs, each scoped to one
  capability. `design/specs/INDEX.md` is the catalog.
- **Active work**: `design/plans/INDEX.md` — current plans.
- **Trophies**: `design/phases.md` — naming convention, scope discipline.
- **Pitfalls**: `design/PITFALLS.md` — known sharp edges.
- **Invariants**: `design/INVARIANTS.md` — load-bearing rules across crates.
