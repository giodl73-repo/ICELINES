# IceLines — Codebase Guide

**Where to write code.** Use this before adding any new file or module.

---

## Decision tree

```
Adding a new data type or domain concept?
  └── icelines-core/src/

Adding NHL API fetch logic, cache, manifest, or data loading?
  └── icelines-fetch/src/

Changing report/export generation?
  └── icelines-cli/src/commands/export*.rs plus ViewModel/report builders

Adding a new CLI command or TUI screen?
  └── icelines-cli/src/commands/ or icelines-cli/src/tui/

Adding a new web HTML/API route?
  └── icelines-web/src/handlers/ plus shared ViewModel code where possible
```

**If you're unsure:** put it in the lowest crate that needs it. Logic that doesn't touch I/O belongs in `icelines-core`. Logic that doesn't touch the CLI belongs in `icelines-fetch` or `icelines-core`.

`docs/vtrace/` is the controlling specification baseline. Use it with
`design/specs/platform-contracts.md`, `design/specs/surface-parity.md`, and
`design/specs/viewmodels.md` before adding or moving surface behavior.

---

## `icelines-core` — pure domain logic

No I/O. No network. No files. Just types and algorithms.

```
src/icelines-core/src/
├── lib.rs                 # Re-exports + CURRENT_SEASON constant
├── model.rs               # Player, Position, GpStatus, PaceScore, Season, ...
├── filter.rs              # PlayerFilter (composable, all-AND logic)
├── scheme.rs              # SkaterWeights, compute_fantasy_score, Scheme
├── scoring.rs             # compute_pace_score, sort_by_pace
├── projection.rs          # ProjectionMode, compute_projection, age_factor
├── history.rs             # SeasonLine, CareerSummary
├── depth_chart.rs         # DepthChartBuilder, LineAssignment
├── cross_team.rs          # CrossTeamMetrics, WebFitClass
├── position.rs            # PositionResolver (parses "C,LW,Util" strings)
├── name.rs                # normalize_name (diacritics, lowercase)
├── teams.rs               # Team abbreviations, full names
└── error.rs               # IcelinesError
```

**Add here when:** new stat formula, new filter type, new Player field, new scoring scheme.

---

## `icelines-fetch` — data loading and I/O

NHL API clients, snapshot store, bundled historical data, optional data sources.

```
src/icelines-fetch/src/
├── lib.rs                 # Re-exports
├── schema.rs              # SkaterBio, SkaterStats, SkaterRealtime, PlayerContract, RosterResponse
├── nhl_api.rs             # NhlApiClient (fetch_all_bios, fetch_all_stats, fetch_all_realtime, ...)
├── stats_loader.rs        # load_into_repo(), source-state-producing repository load boundary
├── datastore.rs           # DataStore manifest/cache/bundle read and explicit fetch/write boundary
├── player_builder.rs      # BuildInputs, make_player, build_players, build_players_from_bios
├── bundled.rs             # 5 embedded seasons via include_bytes!(), load_bios_with_fallback
├── aggregate.rs           # load_aggregate_players(n), load_improvement_map()
├── career.rs              # load_career() — multi-season history from bundled data
├── snapshot.rs            # SnapshotStore, SnapshotTier, SnapshotMeta
├── moneypuck.rs           # SILO: MoneyPuck xG/CF%/FF% (optional, removable)
├── boxscore_client.rs     # Position eligibility from boxscore data
├── cache.rs               # HTTP response cache
├── csv_loader.rs          # CSV parsing utilities
├── resolver.rs            # PlayerResolver (fuzzy name matching)
├── shift_profile.rs       # Linemate pair analysis
└── error.rs               # FetchError
```

**Add here when:** new NHL API endpoint, new data source (keep silo'd if optional), new snapshot tier.

**Key rule:** analytical reads route through `stats_loader::load_into_repo(...)`
or a typed provider/ViewModel boundary that preserves `LoadOutcome.missing` and
source state. `DataStore` is the manifest/cache/bundle boundary; browser GET
paths must not open it just to create missing local state.

**Adding a new optional data source** (like MoneyPuck):
1. Create `src/new_source.rs` — isolated module, all types self-contained
2. Add `Option<T>` fields to `Player` in icelines-core
3. Add a typed read/write boundary that returns missing/unavailable state rather
   than silent empty success.
4. Thread through the relevant loader/provider and ViewModel.
5. Add manifest/snapshot metadata where the source becomes durable local state.

---

## `icelines-site` — deferred static-site generation

The crate still exists for mkdocs/static-site generation support, but the active
CLI entry points were removed. Durable Markdown/JSON/CSV exports are the
current report artifact path.

```
src/icelines-site/src/
└── lib.rs                 # generate_site(), team pages, index
```

**Add here when:** intentionally touching the deferred site generator. New report
or export behavior normally belongs in ViewModels/report projections and
`icelines-cli/src/commands/export*.rs`.

---

## `icelines-web` — axum HTML and JSON surface

Web handlers are thin request adapters over shared ViewModels/providers.

```
src/icelines-web/src/
├── handlers/              # route families and JSON twins
├── templates.rs           # template wiring
├── state.rs               # server/request state
└── static/ + templates/   # assets and HTML templates
```

**Add here when:** changing an HTML/API route, bookmarkable URL state, safe
POST-backed mutation, or no-JS/recovery rendering. GET handlers are read-only;
mutations require POST-backed routes or explicit CLI/TUI deferral.

---

## `icelines-cli` — thin UI layer

Commands parse args and call library functions. **Zero business logic here.**

```
src/icelines-cli/src/
├── main.rs                # tokio::main, dispatch()
├── cli.rs                 # Clap structs: Cli, Commands, all Subcommand enums
├── config.rs              # Config::load() — reads ~/.icelines/config.toml
├── db.rs                  # GroupDb (SQLite groups)
├── fantasy_db.rs          # FantasyDb (SQLite fantasy leagues/teams/rosters)
├── error.rs               # handle_error() — formats anyhow errors for terminal
├── render/                # Terminal color rendering helpers
├── tui/                   # ratatui TUI (app.rs, screens/, event.rs, loader.rs)
└── commands/
    ├── mod.rs             # pub mod for each command
    ├── players.rs         # load_all_players() — shared helper
    ├── fetch.rs           # icelines fetch (rosters, stats, realtime, contracts, moneypuck)
    ├── query.rs           # icelines query (leaders, player, compare) — SortMetric, LeadersArgs
    ├── fantasy.rs         # icelines fantasy (league, team, standings, trade, serve)
    ├── rank.rs            # icelines rank
    ├── team.rs            # icelines team
    ├── analysis.rs        # class, peers, compare, history, group
    ├── project.rs         # icelines project
    ├── scouting.rs        # icelines scouting
    ├── mates.rs           # icelines mates
    ├── tonight.rs         # icelines tonight, schedule, trade
    ├── scheme.rs          # icelines scheme
    ├── snapshot.rs        # icelines snapshot
    └── data.rs            # icelines data
```

**Adding a new command:**
1. Add variant to `Commands` enum in `cli.rs`
2. Create `commands/new_cmd.rs`
3. Add `pub mod new_cmd;` to `commands/mod.rs`
4. Add dispatch arm in `main.rs`
5. Add L2 test in `tests/system_tests.rs`

**Adding a new sort metric to `query leaders`:**
1. Add variant to `SortMetric` enum in `commands/query.rs`
2. Add parse match arm
3. Add `sort_value()` arm
4. Add `display()` arm
5. Add `header()` arm
6. Add `--sort new-metric` to cli.rs `QuerySubcommand::Leaders`
7. Update main.rs dispatch
8. Add L2 test

---

## Tests

```
src/icelines-core/src/         # L0: inline #[cfg(test)] blocks
src/icelines-fetch/src/        # L0: inline, L1: repository.rs tests
src/icelines-fetch/tests/
├── integration_phase2.rs      # L1: scheme scoring + filter
├── integration_pipeline.rs    # L1: full build pipeline
└── mock_nhl_api.rs            # L1: httpmock fixture (3 test players)
src/icelines-cli/src/          # L0: inline unit tests (db.rs, fantasy_db.rs, query.rs, ...)
src/icelines-cli/tests/
└── system_tests.rs            # L2: binary subprocess tests
```

Every new feature needs L0 evidence. New commands need L2 evidence. Changes
that affect shared semantics should also update VTRACE evidence rows or record
why the affected-slice evidence is sufficient.

---

## Data files

```
data/seasons/
├── 20252026/
│   ├── bios.json       # ~900 players, ~449KB — embedded in binary
│   └── stats.json      # ~900 players, ~417KB — embedded in binary
├── 20242025/
├── 20232024/
├── 20222023/
└── 20212022/
```

These are compiled into the binary via `include_bytes!()` in `bundled.rs`. Each file is ~400-450KB. Adding a new season: add the data files, add static bytes in `bundled.rs`, extend `BUNDLED_SEASONS`.

---

## GitHub Actions

```
.github/workflows/
├── ci.yml           # PR gate: test, clippy, fmt — runs on every push
├── release.yml      # On tag push: build 4-platform binaries, GitHub Release
└── data-bundle.yml  # Weekly: pack 5-season data bundles → GitHub Releases
```

All CI runs from `src/` working directory. The binary is `icelines` (`icelines.exe` on Windows).
