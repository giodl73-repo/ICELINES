# IceLines Rust CLI — Phase 2 Implementation Plan

**Date**: 2026-04-25  
**Phase**: 2 of 3 — Site Generation, Fantasy Schemes, and Player Analysis  
**Spec references**:
- `docs/specs/rust-cli.md` — command surface, crate architecture, migration table
- `docs/specs/fantasy-scheme.md` — scheme TOML format, SkaterWeights, GoalieWeights, compute_fantasy_score()
- `docs/specs/dashboard-engine.md` — TOML dashboard definitions, QuerySpec, DashboardSpec, templates, layouts
- `docs/specs/player-analysis.md` — PlayerFilter, PlayerBio, DraftInfo, SeasonHistory, PlayerGroup, all player commands
- `docs/specs/position-engine.md` — PositionProfile, boxscore API, 20%-threshold eligibility algorithm
- `docs/specs/data-sources.md` — NHL API tiers, cache layout, TTL policy

**Companion plans**:
- `docs/plans/2026-04-25-rust-cli-foundation.md` — Phase 1: workspace, icelines-core, icelines-fetch, `team` and `rank`
- `docs/plans/2026-04-25-phase3-tui-projections.md` — Phase 3: TUI, projections, shift data, tonight

---

## Background

Phase 1 delivered the foundational workspace: `icelines-core` data model, `icelines-fetch` NHL
API client with caching, and two terminal commands (`team`, `rank`) that validate end-to-end
correctness. The Python scripts `fetch_rosters.py` and `fetch_gp.py` can now be retired.

Phase 2 completes the site generation pipeline and builds out the full player analysis command
surface. The central deliverable is `icelines build` — a Rust replacement for `scripts/gen_site.py`
driven by a declarative TOML dashboard engine. Alongside it: the fantasy scheme system, the
position engine (boxscore-derived position profiles), the extended player data model (bio, draft
info, career history, groups), the `PlayerFilter` engine, and seven new CLI commands (`players`,
`class`, `peers`, `compare`, `group`, `history`, `scheme`). A SQLite database (`icelines.db`) is
introduced for persistent storage of `PlayerRecord`, `SeasonHistory`, and `PlayerGroup` records
that are too large or too structured for flat JSON cache files. A GitHub Actions CI workflow
ensures the codebase stays clean on every push.

By the end of Phase 2, the full `icelines build → icelines serve → icelines deploy` pipeline
replaces all remaining Python scripts, and users can perform deep player analysis from the
terminal without touching a browser.

---

## Goals

- Implement `icelines build`, `icelines serve`, and `icelines deploy` — full site generation
  replacing `scripts/gen_site.py`, driven by the TOML dashboard engine
- Implement `icelines fetch positions` — boxscore API client, per-game position aggregation,
  `PositionProfile` derivation using the 20%-threshold algorithm from `position-engine.md`
- Implement `icelines scheme` subcommands: `new`, `from-csv`, `list`, `show`, `edit`
- Implement the fantasy scoring engine in `icelines-core`: `compute_fantasy_score()`,
  `SkaterWeights`, `GoalieWeights`, `FantasyScore` with breakdown map
- Implement the dashboard engine in `icelines-site`: TOML `DashboardSpec` loader, Tera template
  engine init, `QueryExecutor`, page builder, built-in templates and layouts
- Extend `icelines-core` data model with `PlayerBio`, `DraftInfo`, `SeasonHistory`, `SeasonLine`,
  `PlayerGroup`, `GroupMember`, `PositionProfile`, `PlayerFilter`
- Implement `icelines players`, `icelines class`, `icelines peers`, `icelines compare`,
  `icelines group`, `icelines history` CLI commands
- Introduce SQLite (`rusqlite`) for `PlayerRecord`, `SeasonHistory`, and `PlayerGroup`
  persistence in `~/.icelines/db/icelines.db`
- Add `icelines-fetch/src/career.rs` — player landing API client for `seasonTotals`
- Add `icelines-fetch/src/boxscore.rs` — boxscore API client and game-log fetcher
- Ship three built-in read-only scheme TOML files (`yahoo-standard`, `espn-standard`,
  `simple-pts`) and three starter dashboard TOML definitions
- Establish a GitHub Actions CI workflow: `cargo test`, `cargo clippy`, `cargo fmt --check`

---

## File Map

Files to create, by crate. All files in Phase 1 remain; this table lists additions only.

### `icelines-core/`

| File | Description |
|------|-------------|
| `src/scheme.rs` | `Scheme`, `SchemeSource`, `SkaterWeights`, `GoalieWeights`, `FantasyScore`; `compute_fantasy_score(stats, weights, gp) -> Option<FantasyScore>`; invariants DI-20 through DI-23 |
| `src/filter.rs` | `PlayerFilter` builder struct with all filter dimensions from `player-analysis.md` §Filtering Engine; `PlayerFilter::apply(&self, players: &[PlayerRecord]) -> Vec<&PlayerRecord>` |
| `src/bio.rs` | `PlayerBio`, `DraftInfo`, `Region` enum (NorthAmerica, Scandinavia, CentralEurope, Russia, Other), `Hand` enum (L, R), `LeagueBackground` enum (NCAA, OHL, WHL, QMJHL, SHL, Liiga, KHL, Other) |
| `src/history.rs` | `SeasonHistory { player_id: u32, seasons: Vec<SeasonLine> }`; `SeasonLine` with season id, team, gp, goals, assists, ppg, toi_pg; `compute_career_ppg(history: &SeasonHistory) -> Option<f64>` (weighted mean, regular season only) |
| `src/group.rs` | `PlayerGroup { id: Uuid, name: String, description: Option<String>, created_at: DateTime<Utc>, members: Vec<u32>, tags: Vec<String> }`; `GroupMember { player_id: u32, added_at: DateTime<Utc>, note: Option<String> }` |
| `src/position_profile.rs` | `PositionProfile { player_id, season, primary_position, eligible_positions, appearance_counts: HashMap<Position,u32>, games_processed, is_fallback }`; `compute_position_profile(counts: &HashMap<Position,u32>, bio_position: Position, gp: u32) -> PositionProfile` implementing the 20%-threshold and tie-breaking rules from `position-engine.md` §4–§5 |
| `src/query/mod.rs` | `QuerySpec`, `QuerySource` enum (Players, Teams, Groups, Class, Leaderboard, Buried, Peers, DepthChart, History), `SortField`, `SortDir`, `DisplaySpec` |
| `src/query/executor.rs` | `run_query(spec: &QuerySpec, store: &PlayerStore) -> Vec<Record>`; dispatch on `QuerySource` variant; `Record { fields: HashMap<String, TemplateValue> }` |
| `src/query/filters.rs` | Internal filter helpers for each `QuerySource`: `filter_players()`, `filter_class()`, `filter_leaderboard()`, `filter_buried()`, `filter_peers()` |
| `src/dashboard/mod.rs` | `DashboardSpec { dashboard: DashboardMeta, query: QuerySpec, display: DisplaySpec, tabs: Option<Vec<TabSpec>> }`; `load_dashboard_spec(path: &Path) -> Result<DashboardSpec, Error>` |
| `src/dashboard/builder.rs` | `DashboardBuilder::build(spec: &DashboardSpec, store: &PlayerStore, out_dir: &Path) -> Result<(), Error>` — runs query, renders each record through Tera template, applies layout, writes output files |
| `src/dashboard/nav.rs` | `collect_nav_entries(dashboard_dir: &Path) -> Vec<NavEntry>` — reads all `.toml` files, returns entries for mkdocs `nav:` key |
| `src/lib.rs` | Add `pub mod scheme`, `pub mod filter`, `pub mod bio`, `pub mod history`, `pub mod group`, `pub mod position_profile`, `pub mod query`, `pub mod dashboard` |

### `icelines-fetch/`

| File | Description |
|------|-------------|
| `src/boxscore.rs` | `fetch_game_log(player_id, season, client) -> Result<Vec<u64>, Error>` — player landing game-log endpoint; `fetch_boxscore(game_id, client, cache) -> Result<BoxscoreResponse, Error>` — boxscore endpoint with shared game-level deduplication; `BoxscoreResponse` serde types matching the `playerByGameStats` structure from `position-engine.md` §2.2; deduplicated fetch: if a game ID is already in boxscore cache it is read from disk |
| `src/career.rs` | `fetch_player_landing(player_id, client, cache) -> Result<PlayerLanding, Error>` — `api-web.nhle.com/v1/player/{ID}/landing`; `PlayerLanding { season_totals: Vec<SeasonTotalsEntry> }`; 7-day cache TTL; excludes playoff seasons (`gameTypeId != 2`) |
| `src/lib.rs` | Add `pub mod boxscore`, `pub mod career` |

### `icelines-site/`

| File | Description |
|------|-------------|
| `Cargo.toml` | Promote from stub: add `tera`, `serde`, `serde_json`, `walkdir`, `toml`; retain `icelines-core` and `thiserror` |
| `src/lib.rs` | `init_tera(template_dir: &Path) -> Result<Tera, Error>` — registers built-in templates from embedded directory; `pub mod dashboard`, `pub mod render` |
| `src/dashboard.rs` | `DashboardPageBuilder` — wraps `DashboardBuilder` from `icelines-core`, owns the `Tera` instance, writes rendered HTML/Markdown to `out_dir`; handles tabbed layout by rendering one page with multiple tab sections |
| `src/render.rs` | `render_team_page(depth_chart: &DepthChart, tera: &Tera) -> Result<String, Error>` — renders the team lineup card markdown using `team.md.tera`; `render_index(teams: &[TeamSummary], tera: &Tera) -> Result<String, Error>` |
| `src/templates/lineup-cell.html` | Tera template: player name, team, position, pace projection, GP badge, CSS fit class |
| `src/templates/player-row.html` | Tera template: ranked table row — rank, name, team, pos, age, gp, ppg_pace, goals_pace, fit badge |
| `src/templates/player-card.html` | Tera template: larger card with photo URL, full bio line, draft info, nationality flag, team logo, pace stats |
| `src/templates/class-card.html` | Tera template: draft pick card — pick number badge, name, team, pace stats, class rank |
| `src/templates/team-summary-card.html` | Tera template: team tracker card — logo, rank badge, position bars (C/LW/RW/D), E/S/B/X counts, total score |
| `src/templates/leaderboard-row.html` | Tera template: position + name + team + stat value + trend arrow |
| `src/templates/group-member-row.html` | Tera template: rank within group, name, pace stats, group-relative percentile |
| `src/templates/team.md.tera` | Tera template: full team lineup card markdown page, 4×3 forward grid + 3×2 defense grid with mkdocs-material card grid layout and CSS fit class names |
| `src/templates/index.md.tera` | Tera template: index page with all 32 teams sorted by Elite count, tier distribution per team |
| `src/layouts/two-column-grid.html` | Layout: 32 items in 2 columns of 16 |
| `src/layouts/four-column-grid.html` | Layout: cards in 4 columns, good for 30+ cards |
| `src/layouts/ranked-table.html` | Layout: sortable HTML table for leaderboards and filtered lists |
| `src/layouts/single-page.html` | Layout: one record filling the page (scouting reports, player profiles) |
| `src/layouts/tabbed.html` | Layout: multiple queries in tabs on one page (e.g., C / LW / RW / D) |
| `src/layouts/comparison.html` | Layout: two queries side by side for head-to-head views |

### `icelines-cli/`

| File | Description |
|------|-------------|
| `src/db.rs` | SQLite via `rusqlite`: `open_db(path: &Path) -> Result<Connection, Error>`; migration runner `run_migrations(conn: &Connection) -> Result<(), Error>`; migration SQL embedded as `const` strings; schema: `player_records`, `season_history`, `player_groups`, `group_members` tables; `upsert_player_record()`, `upsert_season_history()`, `save_group()`, `load_groups()`, `delete_group()` |
| `src/commands/build.rs` | `run_build(args, config)` — reads CSV, loads cached data, builds `PlayerStore`; walks `dashboards/` for all `.toml` files; calls `DashboardPageBuilder` for each; generates `docs/teams/` pages (one per team) and `docs/index.md`; optionally calls `mkdocs build` via `std::process::Command`; `--dashboard <ID>` flag limits to one dashboard |
| `src/commands/serve.rs` | `run_serve(args, config)` — calls `run_build` then spawns `mkdocs serve --dev-addr 127.0.0.1:<PORT>` via `std::process::Command` |
| `src/commands/deploy.rs` | `run_deploy(args, config)` — calls `run_build` then spawns `mkdocs gh-deploy --remote-name <remote>` |
| `src/commands/scheme.rs` | Scheme subcommands: `run_scheme_new()` interactive wizard (prompts per stat with current default, saves TOML); `run_scheme_from_csv()` detects columns, generates template; `run_scheme_list()` reads `~/.icelines/schemes/`; `run_scheme_show()` pretty-prints weights; `run_scheme_edit()` opens `$EDITOR`; validates TOML on save |
| `src/commands/players.rs` | `run_players(args, config)` — builds `PlayerFilter` from CLI flags, calls `PlayerFilter::apply`, renders terminal table with comfy-table; `--json` emits serde_json; reused by `class`, `peers` internals |
| `src/commands/class.rs` | `run_class(year, args, config)` — loads player records, filters by `draft_year`, optionally compares two classes side-by-side; renders class table with pick number, PPG, class median and hit count |
| `src/commands/peers.rs` | `run_peers(player, args, config)` — resolves player, selects peer method (draft-class ±1 year, age ±1 year, pick-range ±15), renders peer table with vs. column and percentile line |
| `src/commands/compare.rs` | `run_compare(p1, p2, args, config)` — resolves both players, renders head-to-head table; `--history N` fetches career data via `career.rs` and shows N-season comparison block |
| `src/commands/group.rs` | Group subcommands — all backed by `db.rs` group CRUD: `run_group_create()`, `run_group_add()`, `run_group_remove()`, `run_group_list()`, `run_group_show()` (full stats table), `run_group_delete()`, `run_group_export()` (JSON/CSV), `run_group_compare()` (side-by-side), `run_group_auto()` (auto-populate from draft class + position via `PlayerFilter`) |
| `src/commands/history.rs` | `run_history(player, args, config)` — resolves player, fetches `SeasonHistory` from `career.rs`, renders multi-season table normalized to 82-game pace if `--pace` flag set |
| `src/commands/fetch.rs` | Extend Phase 1 stub — add `positions` subcommand that calls `boxscore.rs` fetch pipeline, deduplicates game IDs, computes `PositionProfile` for each player, writes to cache |
| `src/cli.rs` | Extend clap `Commands` enum: add `Build`, `Serve`, `Deploy`, `Scheme`, `Players`, `Class`, `Peers`, `Compare`, `Group`, `History`; add `Fetch::Positions` subcommand variant |

### `data/schemes/`

| File | Description |
|------|-------------|
| `data/schemes/yahoo-standard.toml` | Built-in read-only scheme: G=3.0, A=2.0, PPG=1.0, PPA=0.5, SHG=2.0, SHA=1.0, GWG=0.5, HIT=0.5, BLK=0.5, W=5.0, L=-2.0, SV=0.15, GA=-1.0, SHO=4.0 |
| `data/schemes/espn-standard.toml` | Built-in read-only scheme: G=6.0, A=4.0, plus_minus=2.0, pp_goals=2.0, pp_assists=2.0, shots_on_goal=1.0, hits=1.0 |
| `data/schemes/simple-pts.toml` | Built-in read-only scheme: G=1.0, A=1.0 (pure hockey points, no bonuses) |

### `dashboards/`

| File | Description |
|------|-------------|
| `dashboards/u23-centers.toml` | U23 Centers leaderboard: source=players, pos=C, age_max=23, gp_min=15, sort_by=ppg_pace, template=player-row, layout=ranked-table |
| `dashboards/class-2022.toml` | 2022 Draft Class: source=class, draft_year=2022, gp_min=10, sort_by=draft_pick, template=class-card, layout=four-column-grid |
| `dashboards/buried-assets.toml` | Buried trade assets: source=buried, min_fpts=80, min_delta=0.75, sort_by=delta, template=player-row, layout=ranked-table |

### `.github/workflows/`

| File | Description |
|------|-------------|
| `.github/workflows/ci.yml` | GitHub Actions CI: triggers on push and pull_request to master; jobs: `test` (cargo test --workspace), `clippy` (cargo clippy --workspace -- -D warnings), `fmt` (cargo fmt --check); uses `actions/checkout@v4`, `dtolnay/rust-toolchain@stable` matching `rust-toolchain.toml` version |

---

## Phase Breakdown

### Phase 1 — Data Model Extensions in icelines-core

- [ ] Add `uuid` and `chrono` to `icelines-core/Cargo.toml`
- [ ] Implement `src/bio.rs`: `PlayerBio`, `DraftInfo`, `Region`, `Hand`, `LeagueBackground` — all types derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [ ] Implement `src/history.rs`: `SeasonHistory`, `SeasonLine`; implement `compute_career_ppg()` — weighted mean over prior regular seasons (`gameTypeId = 2`), excludes current season, returns `None` for zero-GP history (rookie case)
- [ ] Implement `src/group.rs`: `PlayerGroup`, `GroupMember`; derive `Serialize`, `Deserialize`; `PlayerGroup::new(name, description) -> Self` generates UUID via `uuid::Uuid::new_v4()`
- [ ] Implement `src/position_profile.rs`: `PositionProfile` struct; `compute_position_profile(counts, bio_position, total_gp)` — threshold = `ceil(0.20 × total_gp)`, primary = argmax(counts) with tie-breaking (prefer bio_position, then C > LW > RW > D), fallback when total_gp < 5 games
- [ ] Write unit tests in `src/position_profile.rs`:
  - Draisaitl case: `{C:45, L:25}`, 70 GP → primary=C, eligible=[C, LW] (25 >= ceil(14) = 14)
  - Ehlers case: `{L:40, R:10, C:5}`, 55 GP → primary=LW, eligible=[LW, RW] (10 >= 11? No — 10 < 11, so eligible=[LW] only)
  - Fallback case: 4 games → `is_fallback = true`, primary = bio_position
  - Tie-break case: `{C:10, LW:10}`, 50 GP → primary=C (bio_position=C wins tie)
- [ ] Implement `src/filter.rs`: `PlayerFilter` struct with all fields from spec; `apply()` method chains predicates — each filter dimension is `Option<_>` and is skipped when `None`; implement `Hand`, update `src/lib.rs` re-exports
- [ ] Write unit tests in `src/filter.rs`: filter by age_max, nationality, draft_year, undrafted, ppg_range; verify empty filter returns all players; verify combined filters chain correctly (AND semantics)
- [ ] Verify: `cargo test -p icelines-core` passes with no warnings

### Phase 2 — Fantasy Scheme Engine

- [ ] Implement `src/scheme.rs` in `icelines-core`:
  - `SkaterWeights` struct — all 16 stat fields from `fantasy-scheme.md` data model, all `f32`
  - `GoalieWeights` struct — 6 fields (wins, losses, saves, goals_against, shutouts, save_pct), all `f32`
  - `SchemeSource` enum — Yahoo, Espn, Cbs, Custom; derives `Serialize`, `Deserialize`
  - `Scheme` struct — name, description, version, source, skater: SkaterWeights, goalie: GoalieWeights
  - `FantasyScore { total: f32, per_game: f32, breakdown: HashMap<String, f32> }`
  - `compute_fantasy_score(stats: &SkaterStats, weights: &SkaterWeights, gp: u32) -> Option<FantasyScore>` — returns `None` when `gp < MIN_GP`; `build_breakdown()` inner function that verifies sum within 0.001 of total (invariant DI-23)
- [ ] Write unit tests in `src/scheme.rs`:
  - `compute_fantasy_score` with yahoo-standard weights and known stat line: Beniers 20G 30A 6PPG 5PPA 1GWG 31HIT 69BLK → total=179.0, per_game=2.18 (82 GP)
  - `None` returned when `gp=9` (below MIN_GP=10)
  - `breakdown` keys sum to within 0.001 of `total`
  - Zero weights: a stat with weight 0.0 contributes 0.0 to breakdown, not omitted
- [ ] Create `data/schemes/yahoo-standard.toml`, `data/schemes/espn-standard.toml`, `data/schemes/simple-pts.toml` per spec; mark with `[scheme] readonly = true`
- [ ] Implement scheme loading in `icelines-cli/src/commands/scheme.rs`:
  - `load_builtin_schemes() -> Vec<Scheme>` — reads from `data/schemes/` (bundled with binary via `include_str!` or runtime path resolution)
  - `load_user_schemes(dir: &Path) -> Vec<Scheme>` — reads `~/.icelines/schemes/`
  - `run_scheme_list()` — renders both built-in and user schemes in a single table
  - `run_scheme_show(name)` — finds scheme by name (case-insensitive), pretty-prints weights in three columns, lists unscored stats
  - `run_scheme_new(args)` — interactive prompt loop over each stat key with current default value; saves to `~/.icelines/schemes/{slug}.toml`; `--copy <NAME>` pre-fills from existing scheme
  - `run_scheme_from_csv(path, args)` — reads CSV column headers, maps to stat keys per the detection table in `fantasy-scheme.md`, writes template with all detected stats set to 0.0, prints instructions
  - `run_scheme_edit(name)` — reads `$EDITOR`, opens TOML file; on return from editor, validates by attempting `toml::from_str::<Scheme>()` and reports parse errors
- [ ] Verify: `cargo test -p icelines-core` and `cargo build -p icelines-cli` pass

### Phase 3 — Position Engine and icelines-fetch Extensions

- [ ] Add `toml` to `icelines-fetch/Cargo.toml`
- [ ] Implement `src/boxscore.rs` in `icelines-fetch`:
  - `BoxscoreResponse` serde struct matching the `playerByGameStats` structure from `position-engine.md` §2.2: `awayTeam` and `homeTeam` each contain `forwards: Vec<BoxscorePlayer>`, `defense: Vec<BoxscorePlayer>`, `goalies: Vec<BoxscorePlayer>`
  - `BoxscorePlayer { player_id: u32, name: BoxscoreName, position: String, toi: String, shifts: u32 }`
  - `fetch_game_log(player_id: u32, season: u32, client: &NhlApiClient) -> Result<Vec<u64>, Error>` — `GET api-web.nhle.com/v1/player/{ID}/game-log/{SEASON}/2`; caches at `~/.icelines/cache/landing/{PLAYER_ID}_gamelog_{SEASON}.json`
  - `fetch_boxscore(game_id: u64, client: &NhlApiClient, cache: &Cache) -> Result<BoxscoreResponse, Error>` — `GET api-web.nhle.com/v1/gamecenter/{GAME_ID}/boxscore`; caches at `~/.icelines/cache/boxscores/{SEASON}/{GAME_ID}.json`; completed games never re-fetched (no TTL); today's and future games not cached
  - `fetch_positions_for_all_players(player_ids: &[u32], season: u32, client, cache) -> Result<HashMap<u32, PositionProfile>, Error>` — deduplicates game IDs across all players (fetch each boxscore once), aggregates `appearance_counts` per player, calls `compute_position_profile()` for each
- [ ] Implement `src/career.rs` in `icelines-fetch`:
  - `PlayerLanding { player_id: u32, season_totals: Vec<SeasonTotalsEntry> }`
  - `SeasonTotalsEntry { season: u32, game_type_id: u8, team_abbrev: String, games_played: u32, goals: u32, assists: u32, points: u32 }`
  - `fetch_player_landing(player_id: u32, client: &NhlApiClient, cache: &Cache) -> Result<PlayerLanding, Error>` — 7-day TTL; cache at `~/.icelines/cache/landing/{PLAYER_ID}.json`
  - `to_season_history(landing: &PlayerLanding) -> SeasonHistory` — filters `game_type_id == 2`, maps to `SeasonLine`, computes `ppg = points / games_played as f32` (0.0 if GP=0)
- [ ] Write unit tests in `src/boxscore.rs`: parse a known fixture `tests/fixtures/api/boxscore_sample.json`, assert player IDs and positions extracted correctly
- [ ] Write unit tests in `src/career.rs`: parse `tests/fixtures/api/landing_draisaitl.json`, assert `compute_career_ppg` result matches documented expected value from spec example (Draisaitl 620 GP)
- [ ] Extend `icelines-cli/src/commands/fetch.rs`: add `positions` subcommand — load player IDs from bios cache, call `fetch_positions_for_all_players`, write each `PositionProfile` to `~/.icelines/cache/positions/{SEASON}/{PLAYER_ID}.json`, report players that failed
- [ ] Add test fixtures: `tests/fixtures/api/boxscore_sample.json`, `tests/fixtures/api/landing_draisaitl.json`
- [ ] Verify: `cargo test -p icelines-fetch` and `cargo test -p icelines-cli` pass

### Phase 4 — SQLite Database Layer

- [ ] Add `rusqlite` (with `bundled` feature) and `uuid` to `icelines-cli/Cargo.toml`
- [ ] Implement `src/db.rs` in `icelines-cli`:
  - `open_db(path: &Path) -> Result<Connection, anyhow::Error>` — creates parent dirs if needed; opens with `PRAGMA journal_mode=WAL`
  - Migration runner `run_migrations(conn: &Connection) -> Result<(), anyhow::Error>` — applies migrations in order, uses a `_migrations` table to track applied versions; migrations are embedded `const` strings
  - Migration 001: create `player_records` table — `(player_id INTEGER PRIMARY KEY, name TEXT, name_normalized TEXT, team TEXT, position TEXT, season_gp INTEGER, season_goals INTEGER, season_assists INTEGER, season_points INTEGER, pace_82 REAL, fit_class TEXT, updated_at INTEGER)`
  - Migration 002: create `season_history` table — `(player_id INTEGER, season INTEGER, team TEXT, gp INTEGER, goals INTEGER, assists INTEGER, ppg REAL, toi_pg REAL, PRIMARY KEY (player_id, season))`
  - Migration 003: create `player_groups` table — `(id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT, created_at INTEGER, tags TEXT)` and `group_members` table — `(group_id TEXT, player_id INTEGER, added_at INTEGER, note TEXT, PRIMARY KEY (group_id, player_id))`
  - `upsert_player_record(conn, record: &PlayerRecord) -> Result<(), anyhow::Error>`
  - `upsert_season_history(conn, history: &SeasonHistory) -> Result<(), anyhow::Error>` — replaces all rows for the player
  - `save_group(conn, group: &PlayerGroup) -> Result<(), anyhow::Error>`
  - `load_all_groups(conn) -> Result<Vec<PlayerGroup>, anyhow::Error>`
  - `delete_group(conn, group_id: &str) -> Result<(), anyhow::Error>`
  - `load_group_members(conn, group_id: &str) -> Result<Vec<u32>, anyhow::Error>`
- [ ] Write unit tests in `src/db.rs` using an in-memory SQLite connection (`:memory:`):
  - Verify migrations run cleanly from a fresh database
  - Verify upsert player_record is idempotent (second upsert overwrites, not duplicates)
  - Verify save_group, load_all_groups, delete_group round-trip correctly
  - Verify season_history upsert replaces all rows for a player
- [ ] Wire database initialization into `src/main.rs`: open `~/.icelines/db/icelines.db`, run migrations, pass `Connection` to command handlers that need it (group commands, history commands)
- [ ] Verify: `cargo test` passes including db unit tests

### Phase 5 — Player Analysis Commands

- [ ] Implement `src/commands/class.rs`:
  - `run_class(year: u16, args)` — filters players by `draft_year == year` (from `PlayerBio.draft.year`), sorts by draft pick ascending, computes per-class median PPG and hit count (players with pace_82 ≥ 65 for forwards, ≥ 45 for defense)
  - `--compare <YEAR>` flag: renders two draft classes side-by-side in paired columns
  - `--years-in <N>` flag: filters to players whose `rookie_season` is exactly `N` seasons before the current season
  - `--pos <POS>` and `--round <N>` delegate to `PlayerFilter`
- [ ] Implement `src/commands/peers.rs`:
  - `run_peers(player, args)` — resolves player by name; builds peer group using selected method: `--by draft-class` (same `draft_year ± 1`, same position group), `--by age` (±1 birth year, same position), `--by pick-range` (within 15 overall picks, same draft)
  - Renders peer table with `vs. <player>` column: `+0.03 ahead` or `-0.04 behind` relative to subject player's PPG pace
  - Shows subject player highlighted (indicated with `→` prefix in rank column)
  - Footer: `<player> rank: N of M peers | PPG percentile in group: P%`
- [ ] Implement `src/commands/compare.rs`:
  - `run_compare(p1, p2, args)` — resolves both players; renders head-to-head table with rows: Draft, GP, PPG, G/82, A/82, ES TOI/G, Zone Start %, Avg line (32 teams), Fit on own team
  - `--history <N>` flag: fetches career data for both players via `career.rs`, appends N-season per-player career table below the comparison
  - Footer: `Edge: <PLAYER> by <X> PPG; <OTHER_PLAYER> by <Y>% Zone Start`
- [ ] Implement `src/commands/players.rs`:
  - `run_players(args)` — builds `PlayerFilter` from all CLI flags (pos, age, nationality, region, draft_year, draft_round, pick_max, undrafted, rookie, ppg_range, gp_min, toi_min, handedness)
  - Applies filter, sorts by `--sort` field (ppg default, goals, assists, age, draft)
  - Renders terminal table with comfy-table; first row header shows filter summary
  - `--json` outputs `serde_json::to_string_pretty` of filtered `Vec<PlayerRecord>`
  - `--csv` outputs CSV with header row matching the terminal table columns
- [ ] Implement `src/commands/history.rs`:
  - `run_history(player, args)` — resolves player by name via `PlayerResolver`; fetches `PlayerLanding` from `career.rs`; converts to `SeasonHistory`
  - `--pace` flag: normalizes all raw stats to 82-game pace in the table
  - `--seasons <N>`: limits to the N most recent seasons
  - Footer: career pace average PPG, peak season
- [ ] Implement all `icelines group` subcommands in `src/commands/group.rs`:
  - `create` — prompts for name if not provided, saves empty group to db
  - `add <GROUP> <PLAYER>` — resolves player by name, adds player_id to group in db
  - `remove <GROUP> <PLAYER>` — removes player_id from group in db
  - `list` — reads all groups from db, renders table (name, member count, created date)
  - `show <GROUP>` — loads group, fetches current stats for all member player_ids, renders ranked stat table with median footer
  - `delete <GROUP>` — confirms interactively, deletes group and members from db
  - `export <GROUP>` — writes JSON or CSV of group members with current stats
  - `compare <G1> <G2>` — renders two group stat summaries side-by-side
  - `auto <GROUP> --draft-year <Y> --pos <P>` — builds `PlayerFilter` for the class+position, auto-populates group with matching players
- [ ] Verify: all new commands build and produce correct terminal output; `cargo test -p icelines-cli` passes

### Phase 6 — Site Generation and Dashboard Engine

- [ ] Promote `icelines-site` from stub: add `tera`, `serde_json`, `walkdir`, `toml` to `Cargo.toml`
- [ ] Implement `icelines-site/src/lib.rs`:
  - `init_tera(template_dir: &Path) -> Result<Tera, Error>` — scans template directory, registers each `.html` and `.tera` file by basename
  - Register built-in templates from `src/templates/` directory (embedded at compile time using `include_str!` macros, written to a temp dir at first run, or loaded from the binary's install prefix)
- [ ] Implement all Tera templates in `src/templates/`: `lineup-cell.html`, `player-row.html`, `player-card.html`, `class-card.html`, `team-summary-card.html`, `leaderboard-row.html`, `group-member-row.html`, `team.md.tera`, `index.md.tera`
- [ ] Implement all layout HTML files in `src/layouts/`: `two-column-grid.html`, `four-column-grid.html`, `ranked-table.html`, `single-page.html`, `tabbed.html`, `comparison.html`
- [ ] Implement `icelines-site/src/dashboard.rs`: `DashboardPageBuilder` — owns `Tera` instance; `build_page(spec: &DashboardSpec, store: &PlayerStore, out_dir: &Path) -> Result<(), Error>` runs query via `icelines_core::query::executor::run_query`, renders each record through the spec's template, wraps results in the spec's layout, writes final file to `out_dir / spec.dashboard.id / index.md`
- [ ] Implement `icelines-site/src/render.rs`: `render_team_page`, `render_index`
- [ ] Implement `src/commands/build.rs` in `icelines-cli`:
  - `run_build(args, config)` — loads CSV, fetches or reads cached stats and position profiles, builds `PlayerStore`; iterates `dashboards/` directory with `walkdir`, loads each `.toml` as `DashboardSpec`, calls `DashboardPageBuilder::build_page`; writes 32 team pages and `docs/index.md`; reports per-dashboard errors without aborting the full build; optionally spawns `mkdocs build`
  - `--dashboard <ID>` flag: builds only the specified dashboard TOML file
  - `--no-site` flag: skips the mkdocs subprocess
- [ ] Implement `src/commands/serve.rs`: `run_serve` — calls `run_build` then `mkdocs serve`
- [ ] Implement `src/commands/deploy.rs`: `run_deploy` — calls `run_build` then `mkdocs gh-deploy`
- [ ] Create `dashboards/u23-centers.toml`, `dashboards/class-2022.toml`, `dashboards/buried-assets.toml`
- [ ] Manual smoke test: `cargo run -- build --no-site` generates `docs/teams/COL.md` and `docs/index.md` without error
- [ ] Manual smoke test: `cargo run -- build --dashboard u23-centers --no-site` generates `docs/dashboards/u23-centers/index.md`

### Phase 7 — CI Workflow, Integration Tests, and Cleanup

- [ ] Create `.github/workflows/ci.yml`:
  - Trigger: `push` and `pull_request` targeting `master`
  - Job `test`: `cargo test --workspace` with `cargo` caching via `Swatinem/rust-cache@v2`
  - Job `clippy`: `cargo clippy --workspace -- -D warnings`
  - Job `fmt`: `cargo fmt --all --check`
  - All jobs use `dtolnay/rust-toolchain@stable` pinned to the version in `rust-toolchain.toml`
  - `CARGO_TERM_COLOR: always` env var so CI output is readable in GitHub Actions log
- [ ] Add test fixtures: `tests/fixtures/api/boxscore_sample.json` with realistic structure matching `position-engine.md` §2.2, covering both `awayTeam` and `homeTeam`; `tests/fixtures/api/landing_draisaitl.json` with 8 seasons of `seasonTotals`
- [ ] Write `tests/integration_scheme.rs`:
  - Load `yahoo-standard.toml`, deserialize to `Scheme`
  - Compute `compute_fantasy_score` with Beniers stat line (20G, 30A, 6PPG, 5PPA, 1GWG, 31HIT, 69BLK, 82GP) → assert total=179.0, per_game=2.18
  - Assert `breakdown["goals"] == 60.0`
  - Assert `breakdown` sums to within 0.001 of `total`
  - Assert `None` returned for GP=9
- [ ] Write `tests/integration_positions.rs`:
  - Parse `boxscore_sample.json`, extract appearance counts for known player IDs
  - Assert `compute_position_profile({C:45, L:25}, bio=C, gp=70)` → primary=C, eligible=[C, LW]
  - Assert fallback triggered for player with 3 games → `is_fallback=true`
- [ ] Write `tests/integration_build.rs` (optional, if build pipeline is testable without mkdocs):
  - Build a `PlayerStore` from `sample.csv` and mock API fixtures
  - Build the `u23-centers` dashboard to a temp directory
  - Assert output file exists and contains expected player names
- [ ] Run `cargo test --workspace` — all tests pass
- [ ] Run `cargo clippy --workspace -- -D warnings` — zero warnings
- [ ] Run `cargo fmt --all --check` — no formatting diffs
- [ ] Audit all library code (`icelines-core`, `icelines-fetch`, `icelines-site`) for `unwrap()` — replace with `?` or `expect("documented invariant")`
- [ ] Update `context/waves/WAVE02-APPMANAGER-ADOPTION.md` with Phase 2 completion status

---

## Success Criteria

The following conditions must all be true before this plan is considered complete:

1. `cargo build --release` produces a single `icelines` binary with no errors or warnings
2. `cargo test --workspace` passes with zero failures across all four crates
3. `cargo clippy --workspace -- -D warnings` produces zero warnings
4. `icelines build --no-site` generates 32 team markdown pages and `docs/index.md` from cached data
5. `icelines build --dashboard u23-centers --no-site` generates the U23 Centers dashboard page
6. `icelines fetch positions --dry-run` reports player count and game count without making API calls
7. `icelines scheme list` shows at least three built-in schemes (yahoo-standard, espn-standard, simple-pts)
8. `icelines scheme show yahoo-standard` renders the correct weights table matching `fantasy-scheme.md`
9. `icelines players --pos C --age-max 23` produces a non-empty ranked table for known players in test fixtures
10. `icelines class 2022` produces a table with pick numbers, PPG pace, and class median line
11. `icelines peers "Matty Beniers"` resolves the player and renders a peer table with a rank and percentile footer
12. `icelines compare "Matty Beniers" "Logan Cooley"` produces a head-to-head table with an Edge line
13. `icelines group create "Test"` persists a group to `~/.icelines/db/icelines.db` and `icelines group list` returns it
14. `icelines history "Leon Draisaitl" --pace` renders a season-by-season 82-game pace table
15. `icelines rank --scheme yahoo-standard` computes and ranks by fantasy points instead of pace_82
16. GitHub Actions CI passes on a clean push to master: all three jobs (test, clippy, fmt) are green
17. No `unwrap()` in `icelines-core`, `icelines-fetch`, or `icelines-site` library code
18. All test assertions document expected values with calculation comments
19. `icelines-core` L0 coverage remains ≥ 95% (verified by `cargo llvm-cov`)
20. Every L2 command test exits 0 with correct stdout shape

---

## Test Coverage Requirements

See `docs/specs/test-strategy.md` for full L0/L1/L2 definitions and BENCH archetypes.

### L0 — Unit Tests (≥ 95% icelines-core)

| Test | Expected | Why |
|------|----------|-----|
| `compute_fantasy_score(beniers, yahoo_standard, 82)` | 179.0 | Spec-documented baseline |
| `breakdown_sums_to_total` | within 0.001 | DI-23 invariant |
| `fantasy_score_none_below_min_gp` | None | DI-22 invariant |
| `position_profile_draisaitl(C=45, L=25)` | multi=[C,LW] | 36% ≥ 20% threshold |
| `position_profile_mcmann(L=78)` | primary=LW, single | 0% secondary |
| `position_profile_tie_break(C=40, R=40)` | primary=C | Alpha order |
| `player_filter_combined(C, CAN, ppg≥0.60)` | intersection | Filter composition |
| `player_filter_empty_result` | `[]` not error | No matches valid |
| `draft_info_round_from_overall_pick_1` | round=1 | Draft math correct |

**Property test** (proptest required):
```rust
proptest! {
    fn fantasy_score_scales_linearly_with_goals(g in 0u32..60) {
        let s1 = fantasy_pts_for_goals(g, 3.0);
        let s2 = fantasy_pts_for_goals(g * 2, 3.0);
        assert!((s2 - s1 * 2.0).abs() < 0.001);
    }
}
```

### L1 — Integration Tests

| Test | Verifies |
|------|---------|
| `scheme_from_csv_detects_15_stats` | CSV header → correct column map |
| `scheme_round_trip_toml` | serialize → deserialize → identical |
| `fantasy_pipeline_9_archetypes` | fixture CSV + scheme → exact scores |
| `position_profile_from_boxscore` | 3 fixture boxscores → correct profile |
| `dashboard_toml_loads_query` | u23_centers.toml → QuerySpec(pos=C, age≤23) |
| `group_persist_round_trip` | create/add/show/delete in-memory DB |
| `site_build_32_files` | build → 32 team markdown files exist |
| `scheme_built_in_immutable` | write yahoo-standard → Error::ReadOnly |

### L2 — System Tests (every Phase 2 command)

| Command | Assertion |
|---------|-----------|
| `icelines scheme from-csv tests/fixtures/sample_skaters.csv` | Exit 0, file created |
| `icelines scheme list` | Exit 0, shows scheme |
| `icelines scheme show yahoo-standard` | Exit 0, 11 scored stats visible |
| `icelines players --pos C --age-max 23` | Exit 0, all rows C ≤ 23 |
| `icelines class 2022 --pos C` | Exit 0, contains 2022 picks |
| `icelines rank --scheme yahoo-standard --top 10` | Exit 0, 10 rows |
| `icelines build --no-site` | Exit 0, docs/teams/COL.md created |
| `icelines group create TestGroup` | Exit 0, group in db |
| `icelines fetch positions --dry-run` | Exit 0, no HTTP calls |

---

## Out of Scope

**Phase 3 — TUI, Projections, Shift Data, Tonight:**
- `icelines tui` / `icelines` (no args) — ratatui full-screen TUI with 8 screens (`tui.md`)
- `icelines project` — rest-of-season projections, age curve, schedule difficulty factor (`projection-engine.md`)
- `icelines tonight` — schedule API + projected lines from boxscores
- `icelines schedule` — forward calendar view
- `icelines trade` — depth chart diff for trade analysis
- `icelines mates` — shift-based linemate analysis (requires Tier 3 shift data)
- `icelines scouting` — full scouting report (Tier 3 — shift data required for linemate section)
- Shift data fetch: `icelines fetch shifts` — shiftchart API client and game log fetcher
- `ProjectionMode`, `ProjectionResult`, `compute_projection()` in `icelines-core`
- `GameSchedule`, `RemainingGames` types and schedule API client
- `Shift`, `ShiftProfile`, `compute_linemates()` types and shift aggregation
- Age curve and schedule difficulty factor computation
- `cargo install` packaging and release pipeline (cross-platform binary builds)
- `.github/workflows/release.yml` — Windows, macOS, Linux binary build and GitHub Release publishing
