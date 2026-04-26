# IceLines Rust CLI — Foundation Implementation Plan

**Date**: 2026-04-25  
**Phase**: 1 of 3 — Foundation (icelines-core + icelines-fetch + icelines-cli: `team` and `rank`)  
**Spec references**:
- `docs/specs/rust-cli.md` — command surface, crate architecture, data model
- `docs/specs/data-sources.md` — NHL API tiers, shift data, composite scoring roadmap
- `docs/specs/player-analysis.md` — player filtering, draft classes, peer groups (Phase 2+)
- `docs/specs/dashboard-engine.md` — TOML-driven dashboard generation (Phase 2+)

---

## Background

The IceLines pipeline currently exists as a collection of Python scripts. The Rust CLI rewrite
addresses three structural problems: no shared data model, no error surface for failed player
resolution, and no caching between runs. This plan covers the foundation phase: the core data
model, the NHL API fetch layer with caching, and the two terminal-output commands (`team` and
`rank`) that validate end-to-end correctness before the site generator is built.

`icelines build`, `icelines serve`, `icelines deploy`, and `icelines compare` are out of scope
for this plan (Phase 2: site generation).

---

## Goals

- Stand up a Cargo workspace with four crates: `icelines-core`, `icelines-fetch`,
  `icelines-site` (stub), `icelines-cli`
- Implement all data model types in `icelines-core`
- Implement the CSV loader, NHL API client, and cache layer in `icelines-fetch`
- Implement the scoring engine and fit classification in `icelines-core`
- Implement `icelines team <TEAM>` and `icelines rank` in `icelines-cli` with terminal output
- Establish the test suite with known-value assertions and the canonical test fixture

---

## File Map

Files to create, by crate:

### Workspace root

| File | Description |
|------|-------------|
| `Cargo.toml` | Workspace manifest: `[workspace]` with four members, shared dependencies |
| `.cargo/config.toml` | Rustflags: `--deny warnings` in CI, target config |
| `rust-toolchain.toml` | Pin stable Rust version (e.g. `1.78.0`) |

### `icelines-core/`

| File | Description |
|------|-------------|
| `Cargo.toml` | Dependencies: `thiserror`, `serde`, `serde_json` |
| `src/lib.rs` | Re-exports: `pub mod model`, `pub mod scoring`, `pub mod position`, `pub mod error` |
| `src/error.rs` | `icelines_core::Error` enum with thiserror |
| `src/model.rs` | All data model types: `Player`, `TeamAbbr`, `Position`, `Slot`, `FitClass`, `PaceScore`, `DepthChart`, `LineAssignment` |
| `src/position.rs` | `PositionResolver`: parses Yahoo position strings ("C,LW") into primary + all-eligible |
| `src/scoring.rs` | `compute_pace_score()`, `classify_fit()`, `sort_by_rank()` |
| `src/depth_chart.rs` | `DepthChartBuilder`: assigns players to forward lines and defense pairs |
| `src/name.rs` | `normalize_name()`: Unicode NFC normalization, diacritic strip, lowercase |
| `src/teams.rs` | `CANONICAL_TEAMS`: the 32 NHL team abbreviations, team full names, `TeamAbbr::parse()` |

### `icelines-fetch/`

| File | Description |
|------|-------------|
| `Cargo.toml` | Dependencies: `icelines-core`, `thiserror`, `reqwest`, `tokio`, `serde`, `serde_json`, `csv`, `encoding_rs` |
| `src/lib.rs` | Re-exports: `pub mod csv_loader`, `pub mod nhl_api`, `pub mod cache`, `pub mod resolver`, `pub mod error` |
| `src/error.rs` | `icelines_fetch::Error` enum: Http, Cache, CsvParse, PlayerNotFound, SchemaChanged, NameAmbiguous |
| `src/csv_loader.rs` | `load_csv(path: &Path) -> Result<Vec<Player>, Error>`: reads Yahoo CSV, validates columns, builds `Player` stubs |
| `src/nhl_api.rs` | `NhlApiClient`: reqwest Client, `fetch_all_bios(season) -> Vec<PlayerBio>` and `fetch_all_stats(season) -> Vec<SkaterStats>` — both paginate using `limit=100&start={N}` until `start + page_size >= total` |
| `src/schema.rs` | NHL API response types with `#[serde(deny_unknown_fields)]`: `PlayerBioResponse { data: Vec<PlayerBio>, total: u32 }` (from `/skater/bios`), `SkaterStatsResponse { data: Vec<SkaterStats>, total: u32 }` (from `/skater/summary`) |
| `src/cache.rs` | `Cache`: file-based cache, `get(key)`, `put(key, value, ttl)`, `invalidate(key)`, TTL = 24h |
| `src/resolver.rs` | `PlayerResolver`: resolves Player name → NHL player ID using exact match, normalized match, then reports ambiguity |

### `icelines-site/` (stub only in this phase)

| File | Description |
|------|-------------|
| `Cargo.toml` | Dependencies: `icelines-core`, `thiserror`. No Tera yet — stub only. |
| `src/lib.rs` | Empty except for a `// Phase 2: site generation` comment |

### `icelines-cli/`

| File | Description |
|------|-------------|
| `Cargo.toml` | Dependencies: all three library crates, `clap`, `anyhow`, `tokio`, `comfy-table`, `owo-colors` or `termcolor` |
| `src/main.rs` | `#[tokio::main]`, clap Cli struct, subcommand dispatch |
| `src/cli.rs` | Clap derive: `Cli`, `Commands` enum with `Fetch`, `Build`, `Serve`, `Deploy`, `Team`, `Rank`, `Compare` |
| `src/commands/fetch.rs` | `run_fetch(args, csv_path)`: fetch handler |
| `src/commands/team.rs` | `run_team(args, team_abbr)`: loads CSV, fetches/caches GP, builds DepthChart, renders terminal card |
| `src/commands/rank.rs` | `run_rank(args)`: loads CSV, fetches/caches GP, computes pace rankings, renders terminal table |
| `src/render/terminal.rs` | Terminal rendering: colored rows via owo-colors, tabular output via comfy-table |
| `src/config.rs` | Config loading: `~/.icelines/config.toml`, project-local `.icelines.toml` |
| `src/error.rs` | CLI error formatting: translate crate errors to user messages |

### `tests/`

| File | Description |
|------|-------------|
| `tests/fixtures/sample.csv` | Canonical test CSV (see BENCH role: 9 player archetypes) |
| `tests/fixtures/api/` | Mock API response JSON files for known player IDs |
| `tests/integration_scoring.rs` | Integration test: load sample.csv, mock GP data, verify pace projections and fit classes |
| `tests/integration_team.rs` | Integration test: verify Colorado Avalanche lineup card structure (4×3 forward grid, 3×2 D grid) |

---

## Phase Breakdown

### Phase 1 — Workspace Setup and icelines-core Data Models

- [ ] Create `Cargo.toml` at workspace root with `[workspace]` and four members
- [ ] Create `rust-toolchain.toml` with stable toolchain pin
- [ ] Create `icelines-core/Cargo.toml` with thiserror, serde, serde_json
- [ ] Implement `src/error.rs`: `icelines_core::Error` with thiserror variants
- [ ] Implement `src/model.rs`: all types — `Player`, `TeamAbbr`, `Position`, `Slot`, `FitClass`, `PaceScore`, `DepthChart`, `LineAssignment`
- [ ] Implement `src/teams.rs`: `CANONICAL_TEAMS` constant, `TeamAbbr::parse()` validates against list
- [ ] Implement `src/position.rs`: `PositionResolver::parse("C,LW")` → `(Center, [Center, LeftWing])`
- [ ] Implement `src/name.rs`: `normalize_name()` — NFC normalization, diacritic strip, lowercase
- [ ] Verify: `cargo check -p icelines-core` passes with no warnings

### Phase 2 — icelines-fetch: NHL API Client

- [ ] Create `icelines-fetch/Cargo.toml` with all dependencies
- [ ] Implement `src/schema.rs`: NHL API response types with `deny_unknown_fields`
- [ ] Implement `src/cache.rs`: file cache in `~/.icelines/cache/`, `get`/`put`/`invalidate`, 24h TTL
- [ ] Implement `src/nhl_api.rs`: `NhlApiClient` with reqwest, `fetch_all_bios(season)` and `fetch_all_stats(season)` (bulk pagination), exponential backoff on 429/503
- [ ] Implement `src/resolver.rs`: `PlayerResolver` — exact name match, normalized match, ambiguity detection (Sebastian Aho case)
- [ ] Implement `src/csv_loader.rs`: `load_csv()` — validate expected columns by name, parse rows, report missing fields
- [ ] Write unit tests in `src/csv_loader.rs` module: empty rows, missing columns, BOM input, accented names
- [ ] Write unit tests in `src/resolver.rs` module: Slafkovský normalization, Sebastian Aho disambiguation
- [ ] Verify: `cargo test -p icelines-fetch --lib` passes

### Phase 3 — Scoring Engine in icelines-core

- [ ] Implement `src/scoring.rs`:
  - `compute_pace_score(points: u32, gp: u32) -> Option<PaceScore>` — returns None if gp < MIN_GP
  - `classify_fit(pace_score: &PaceScore, position: Position) -> FitClass` — applies forward/defense thresholds
  - `sort_by_rank(players: &mut Vec<Player>)` — pace_82 desc, goals_per_game desc, name asc
- [ ] Implement `src/depth_chart.rs`: `DepthChartBuilder::build(team: &TeamAbbr, players: Vec<Player>) -> DepthChart`
  - Assigns LW/C/RW slots for up to 4 forward lines
  - Assigns LD/RD slots for up to 3 defense pairs
  - Populates `unplaced` and `below_min_gp` vecs
- [ ] Write unit tests in `src/scoring.rs`:
  - `compute_pace_score(100, 75) == PaceScore { pace_82: 109.333... }` (McDavid 2023-24)
  - `compute_pace_score(50, 70) == PaceScore { pace_82: 58.571... }`
  - `compute_pace_score(_, 0) == None` (GP=0 case)
  - `compute_pace_score(_, 9) == None` (below MIN_GP)
  - `compute_pace_score(_, 10)` is Some (exactly at MIN_GP)
  - `classify_fit` at each threshold boundary (forward: 64.9 → Solid, 65.0 → Elite)
  - `classify_fit` uses defense thresholds for `Position::Defense`
- [ ] Verify: `cargo test -p icelines-core` passes, all assertions use documented expected values

### Phase 4 — icelines-cli: `team` and `rank` Commands

- [ ] Create `icelines-cli/Cargo.toml`
- [ ] Create stub `icelines-site/Cargo.toml` and `src/lib.rs`
- [ ] Implement `src/main.rs` and `src/cli.rs` with clap derive: Cli, Commands enum
- [ ] Implement `src/config.rs`: load `.icelines.toml` from working directory, fall back to `~/.icelines/config.toml`
- [ ] Implement `src/render/terminal.rs`: `render_team_card()`, `render_rank_table()` using comfy-table + owo-colors
- [ ] Implement `src/commands/team.rs`:
  - Parse team abbreviation or partial name
  - Load CSV (`load_csv`)
  - Filter players for team
  - Fetch/cache GP for each player (`NhlApiClient`)
  - Compute pace scores (`scoring::compute_pace_score`)
  - Classify fits
  - Build depth chart
  - Render terminal card
- [ ] Implement `src/commands/rank.rs`:
  - Load CSV
  - Fetch/cache GP for all players
  - Compute pace scores
  - Sort by rank
  - Render terminal table (top N, position filter, team filter)
- [ ] Implement `src/commands/fetch.rs` (drives the fetch layer independently of build)
- [ ] Implement `src/error.rs`: error-to-message translation for all crate error variants
- [ ] Manual smoke test: `cargo run -- team COL` renders Colorado Avalanche lineup card
- [ ] Manual smoke test: `cargo run -- rank --top 20` renders top-20 skaters with fit colors

### Phase 5 — Tests

- [ ] Create `tests/fixtures/sample.csv` with 9 player archetypes (see BENCH role for full list)
- [ ] Create `tests/fixtures/api/` with mock JSON responses for each sample CSV player
- [ ] Write `tests/integration_scoring.rs`:
  - Load sample.csv, mock GP data
  - Assert exact pace projections for each player (documented expected values)
  - Assert correct fit classification for each player
  - Assert GP=0 player produces None pace score and is not in ranked output
  - Assert player at exactly MIN_GP=10 is included in ranking
- [ ] Write `tests/integration_team.rs`:
  - Build DepthChart for a team from sample.csv + mock data
  - Assert forward_lines[0] = [expected_LW, expected_C, expected_RW]
  - Assert below_min_gp contains the GP=0 player
- [ ] Run `cargo test` — all tests must pass
- [ ] Run `cargo clippy -- -D warnings` — zero warnings
- [ ] Run `cargo fmt --check` — no formatting diffs

---

## Success Criteria

The following conditions must all be true before this plan is considered complete:

1. `cargo build --release` produces a single binary at `target/release/icelines` with no errors
2. `cargo test` passes with zero failures
3. `cargo clippy -- -D warnings` produces zero warnings
4. `icelines team COL` renders a 4×3 forward grid and 3×2 defense grid with fit-classified colors
5. `icelines rank --top 20` renders a ranked table with pace projections and correct fit colors
6. `icelines fetch --dry-run` reports player resolution results without making API calls
7. The canonical test fixture (`tests/fixtures/sample.csv`) produces documented expected outputs
8. Every test assertion documents its expected value with a calculation comment
9. No `unwrap()` in `icelines-core`, `icelines-fetch`, or `icelines-site` library code
10. `icelines-core` has zero dependencies that perform I/O (verified by `cargo tree -p icelines-core`)

---

## Out of Scope for This Plan

**Phase 2 — Site Generation + Player Analysis:**
- `icelines build` — TOML dashboard engine, Tera templates (`dashboard-engine.md`)
- `icelines serve` and `icelines deploy` — mkdocs integration
- `icelines players`, `icelines class`, `icelines peers`, `icelines compare` (`player-analysis.md`)
- `icelines group` — persistent watchlists
- Tera template implementation in `icelines-site`
- GitHub Actions CI workflow
- `PlayerBio`, `DraftInfo`, `SeasonHistory` model extensions (`player-analysis.md`)
- `PlayerFilter` engine for composable filtering

**Phase 3 — Shift Data + Social + Distribution:**
- `icelines mates` — shift-based linemate analysis (`data-sources.md` Tier 3)
- `icelines scouting` — full scouting report
- Tier 4–6 data (advanced stats, social signals, beat media)
- Composite scoring: PPG × TOI, xGF integration (`data-sources.md`)
- `cargo install` packaging, Windows binary release pipeline
