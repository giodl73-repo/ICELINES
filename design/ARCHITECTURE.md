# IceLines Architecture

---

## Overview

IceLines is a 4-crate Rust workspace. Each crate has a single responsibility.
The dependency chain is strict — lower crates never import higher ones.

```
┌─────────────────────────────────────────────────────────┐
│                     icelines-cli                        │
│  Commands, TUI, HTTP server (axum), argument parsing    │
│  Thin UI layer — zero business logic                    │
└────────────────────┬───────────────────────────────────┘
                     │ depends on all three
┌────────────────────┼────────────────────────────────────┐
│   icelines-site    │    icelines-fetch                  │
│   Markdown/mkdocs  │    NHL API, snapshots, bundled data │
│   site generation  │    MoneyPuck, realtime, aggregate  │
└────────────────────┼────────────────────────────────────┘
                     │ both depend on core
┌────────────────────▼───────────────────────────────────┐
│                    icelines-core                        │
│   Player, Position, PlayerFilter, scheme scoring,       │
│   projections, history, depth chart — pure logic, no I/O│
└─────────────────────────────────────────────────────────┘
```

---

## Data flow

```
NHL API (api-web.nhle.com, api.nhle.com/stats/rest/en)
    │
    ▼
icelines fetch all
    │  writes JSON to snapshot store
    ▼
~/.icelines/snapshots/{name}/
    │  stats/bios.json, realtime/realtime.json, ...
    │
    ▼
PlayerRepository::load_all()          ← THE single entry point for player data
    │
    ├── Try: snapshot store (fresh from fetch)
    ├── Try: bundled binary data (5 seasons compiled in)
    └── Err: "run icelines fetch all"
    │
    ▼
Vec<Player>                           ← rich struct with 50+ fields
    │
    ├── icelines query leaders         → filtered, sorted, rendered
    ├── icelines team EDM              → depth chart, fit classification
    ├── icelines fantasy team-score    → fantasy points via scheme engine
    └── icelines tui                   → ratatui screens
```

---

## Player — the central type

`icelines_core::model::Player` is the hub every command talks to. Fields come from multiple NHL API endpoints joined on `player_id`.

| Field group | Source | Optional |
|------------|--------|----------|
| G, A, Pts, pace_score | SkaterStats (summary endpoint) | No |
| PP, SH, GWG, shots, +/-, TOI | SkaterStats | Some fields yes |
| hits, blocks, giveaways, takeaways, PIM | SkaterRealtime | Yes (0 if not fetched) |
| birth_date, nationality, draft, height, weight | SkaterBio | Mostly yes |
| xg, cf_pct_5v5, ff_pct_5v5, xgf_pct_5v5 | MoneyPuck CSV | Yes (None if not fetched) |
| contract_expiry_year, expiry_type | PlayerContract (landing API) | Yes (None if not fetched) |
| headshot_url, sweater_number | RosterPlayer | Yes |

Fields from optional sources are `Option<T>` — absent when that source hasn't been fetched. Commands degrade gracefully.

---

## Snapshot store

Named, sealed snapshots under `~/.icelines/snapshots/`. Each snapshot has:
- A `SnapshotTier` (Rosters, Stats, Realtime, Positions, Contracts, MoneyPuck, ...)
- SHA-256 integrity hashes per file
- A provenance chain (parent snapshot reference)
- A sealed flag — sealed snapshots are immutable

Tiers form a chain: Rosters → Stats → Realtime (built on top of Stats).

`PlayerRepository` searches for the active snapshot for each tier independently, so you can have fresh Stats without fresh Realtime.

---

## Bundled data

5 seasons (20212022–20252026) of `bios.json` + `stats.json` are compiled directly into the binary via `include_bytes!()` in `icelines-fetch/src/bundled.rs`.

This means `icelines query leaders` works immediately after install with no fetch required. The binary is ~10MB larger but cold-start is instant.

`icelines fetch all` creates a snapshot that takes precedence over bundled data for the current season.

Historical seasons (20212022–20242025) are immutable — their bundled data never changes.

---

## Optional data sources (silo'd)

Two data sources are optional and independently removable:

**MoneyPuck** (`moneypuck.rs`):
- Free CSV download from moneypuck.com
- Provides: xG (individual expected goals), CF%, FF%, xGF% at 5v5
- Stored as `SnapshotTier::MoneyPuck`
- All Player fields are `Option<f32>` — None when not fetched

**NHL Realtime** (`schema.rs → SkaterRealtime`):
- From `/stats/rest/en/skater/realtime`
- Provides: hits, blocked_shots, missed_shots, giveaways, takeaways, PIM
- Stored as `SnapshotTier::Realtime`  
- Player fields are `u32` defaulting to 0 when not fetched

**Contracts** (`schema.rs → PlayerContract`):
- From `api-web.nhle.com/v1/player/{id}/landing`
- Provides: expiry_year, expiry_type (UFA/RFA/ELC)
- Note: NHL API does NOT expose cap hit — that's PuckPedia only (private API)
- All Player fields are `Option<_>` — None when not fetched

---

## Query engine

`icelines query leaders` is the most complex command. Architecture:

```
LeadersArgs (from CLI flags)
    │
    ▼
SortMetric::parse(&args.sort)      → validates sort metric name
    │
    ▼
load_aggregate_players(n)          → if --seasons N > 1: aggregate across N bundled seasons
OR load_all_players()              → if --seasons 1: current season from PlayerRepository
    │
    ▼
PlayerFilter::apply(&players)      → pos, age, nationality, draft, ppg_min, gp_min, ...
    │
    ├── inline filters              → gp_max, ufa, rfa, elc, expiry_year (not in PlayerFilter)
    │
    ▼
if Improvement:                    → load_improvement_map() → sort by Y/Y PPG delta
else:                              → sort_by(metric.sort_value)
    │
    ▼
leaders_table() or leaders_json() or leaders_csv()
```

30+ sort metrics in `SortMetric` enum. Each implements `sort_value()`, `display()`, `header()`.

---

## Fantasy system

Built on SQLite (`~/.icelines/icelines.db`, shared with group commands).

```
fl_leagues    → fl_teams    → fl_roster
               │
               └── scoring via compute_fantasy_score(to_scheme_stats(player), scheme)
```

`to_scheme_stats(Player) → scheme::SkaterStats` bridges the gap between the rich Player struct and the scheme engine's simpler input type. This is where hits, blocks, GWG, PP goals all flow into fantasy scoring.

The HTTP server (`icelines fantasy serve`) uses axum with `Arc<AppState>` shared state, opening a new SQLite connection per request.

---

## Testing pyramid

```
338 tests total (as of 2026-04-26)

L0 unit (inline)                   ~270 tests
├── icelines-core  147 tests        model helpers, filter, scheme, history, depth chart, ...
├── icelines-fetch  46 tests        bundled data, moneypuck, aggregate, career
└── icelines-cli    30 tests        sort metrics, fantasy DB, TUI app state, ...

L1 integration                      ~46 tests
├── PlayerRepository                 graceful degradation, optional sources
├── integration_phase2               scheme scoring, filter engine
├── integration_pipeline             full build pipeline
└── mock_nhl_api                     httpmock fixture, 3 test players

L2 system                           ~68 tests
└── system_tests.rs                  binary subprocess tests, all commands
```

The mock NHL API fixture (`mock_nhl_api.rs`) provides a full httpmock server with realistic bios/stats/realtime JSON for 3 test players. Use it for any new fetch integration tests.

---

## Future: proof/DASHBOARD-SPEC integration

Each TUI screen will become a `.dashboard.source.md` template:
1. `icelines export md` generates `~/.icelines/reports/*.md` (live stats tables)
2. mdpath `md://reports/...` URIs address those tables
3. `proof compile --width N --height N` renders the template to ASCII
4. TUI renders the compiled string in a ratatui Paragraph widget

Templates live in `~/.icelines/dashboards/` and are user-editable.
Adding a new dashboard = writing a template, not Rust code.

See `design/specs/dashboard-engine.md` and the proof DASHBOARD-SPEC.
