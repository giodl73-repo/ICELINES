# IceLines Phase 4 — Data, History & Polish

**Date**: 2026-04-26  
**Phase**: 4 of 4 — Historical data, season bundles, multi-season analysis, TUI polish  
**Spec references**: docs/specs/data-sources.md, docs/specs/projection-engine.md,
  docs/specs/tui.md, docs/specs/player-analysis.md

**Companion plans**:
- Phase 1: docs/plans/2026-04-25-rust-cli-foundation.md ✅
- Phase 2: docs/plans/2026-04-25-phase2-site-analysis.md ✅
- Phase 3: docs/plans/2026-04-25-phase3-tui-projections.md ✅

---

## Background

Phase 3 completed `icelines` as a full-featured CLI with projections, TUI, scouting,
and a cross-platform release pipeline. Phase 4 addresses three remaining gaps:

1. **Historical data** — `icelines history` and `icelines scouting` currently show only
   the current season. Career context requires multi-season data.

2. **Season bundles** — each NHL season is ~93 KB compressed. Pre-packaging seasons
   as GitHub Release assets lets users download 5 years of history in <500 KB instead
   of hitting the NHL API repeatedly. Data never changes for past seasons.

3. **Polish** — TUI loads stubs when no snapshot is warm; `icelines build` should
   accept an optional `--season` flag; SQLite should replace JSON groups.

---

## Season Bundle Size

| Coverage | Size (gzip) | Notes |
|----------|-------------|-------|
| 1 season | ~93 KB | bios + stats + 32 rosters |
| 5 seasons | ~466 KB | 2021-22 through 2025-26 |
| 10 seasons | ~933 KB | 2016-17 through 2025-26 |
| 20 seasons | ~1.8 MB | Full modern era |

The NHL API has complete data back to at least 2010-11. Pre-packaged bundles
are published as GitHub Release assets alongside binary releases, or as
separate `data-YYYYZZZZ` releases. Users who just want to browse current-season
data run `icelines fetch all`; users who want career context run
`icelines data install --seasons 5`.

---

## Goals

- Ship `icelines data` command: install/list/remove pre-packaged season bundles
- Implement multi-season history in `icelines history` and `icelines scouting`
- Implement regressed projection with real career PPG from stored history
- Load players in TUI on launch so all 8 screens show live data
- Replace JSON groups file with SQLite (rusqlite)
- Add `icelines build --season 2024` to generate historical site snapshots
- Publish first season bundle releases (5 most recent seasons)

---

## File Map

### icelines-fetch

| File | Description |
|------|-------------|
| `src/season_bundle.rs` | `SeasonBundle`: pack/unpack season data to `.tar.gz`. `fetch_season_bundle(season, cache_dir)` downloads from GitHub Release if not cached. `bundle_season(season, bios, stats, rosters)` creates bundle. |
| `src/career.rs` | `CareerStats`: Vec<SeasonLine> assembled from stored season bundles. `load_career(player_id, seasons, cache_dir) -> CareerStats`. |

### icelines-core

| File | Description |
|------|-------------|
| `src/history.rs` | `SeasonLine { season, team, gp, goals, assists, ppg, toi_pg_seconds }`, `CareerSummary { player_id, seasons, career_ppg, peak_ppg, peak_season }`. |

### icelines-cli

| File | Description |
|------|-------------|
| `src/commands/data.rs` | `icelines data install --seasons N` — downloads last N season bundles from GitHub Releases. `icelines data list` — shows installed seasons with sizes. `icelines data remove --season YYYYZZZZ` — deletes one season's cached data. |
| `src/tui/loader.rs` | Background task that loads all players from snapshot on TUI launch, stores in `App.players`. Shows spinner on Home screen while loading. |
| `src/db.rs` | SQLite migration runner for groups table. Migration 001: CREATE TABLE groups. Replaces `~/.icelines/groups.json`. |

### GitHub Actions / data releases

| File | Description |
|------|-------------|
| `.github/workflows/data-bundle.yml` | Triggered manually or on schedule. Fetches each season's data from NHL API, compresses to `data-YYYYZZZZ.tar.gz`, uploads as GitHub Release asset on `data-YYYYZZZZ` tag. |
| `scripts/bundle_seasons.py` (bootstrap only) | One-time Python script to build all historical bundles before the Rust pipeline is ready. Deleted after first run. |

---

## Phase Breakdown

### Phase 4-A: Season bundle format + `icelines data` command

- [ ] Define bundle format: `data-YYYYZZZZ.tar.gz` containing `bios.json`, `stats.json`, `rosters/{TEAM}.json`
- [ ] Implement `SeasonBundle::pack(season, bios, stats, rosters)` in icelines-fetch
- [ ] Implement `SeasonBundle::unpack(path, cache_dir)` — extracts to `~/.icelines/snapshots/{SEASON}/`
- [ ] `icelines data install --seasons N` — downloads last N bundles from GitHub Releases API
- [ ] `icelines data install --season YYYYZZZZ` — install specific season
- [ ] `icelines data list` — show installed seasons with sizes and player counts
- [ ] `icelines data remove --season YYYYZZZZ`
- [ ] Wire `data` command in cli.rs and main.rs dispatch
- [ ] L0: bundle pack/unpack round-trip test
- [ ] L2: `icelines data list` exits 0

### Phase 4-B: Multi-season history in core + fetch

- [ ] Implement `SeasonLine` and `CareerSummary` in icelines-core/src/history.rs
- [ ] `load_career(player_id, snapshot_store, seasons)` in icelines-fetch — reads from installed season bundles
- [ ] `icelines history <PLAYER> --seasons 5` — shows 5-year career table with pace-normalized stats
- [ ] `icelines scouting <PLAYER>` — section 3 (Career Trajectory) now shows real multi-season trend
- [ ] Regressed projection uses real `career_ppg` from stored history (not current_ppg fallback)
- [ ] `icelines project <PLAYER> --mode regressed` now meaningful with real career data
- [ ] L0: CareerSummary peak_ppg detection
- [ ] L1: load_career from fixture season bundles

### Phase 4-C: GitHub Actions data bundle pipeline

- [ ] `.github/workflows/data-bundle.yml` — workflow_dispatch + cron (weekly)
- [ ] Matrix: last 5 seasons (configurable via input)
- [ ] Steps: fetch from NHL API → pack bundle → upload to `data-YYYYZZZZ` GitHub Release tag
- [ ] Bundle size verification: fail if bundle > 500 KB (indicates data corruption)
- [ ] Create `data-YYYYZZZZ` tags for initial 5 seasons on first run

### Phase 4-D: TUI data loading + SQLite groups

- [ ] `src/tui/loader.rs` — async background task launched on TUI start
- [ ] App::players populated from snapshot if available, empty otherwise
- [ ] Home screen: show "(loading…)" spinner while players are being loaded
- [ ] Team screen: renders real depth chart from App::players when available
- [ ] Search screen: fuzzy search works against loaded players
- [ ] SQLite migration runner in `src/db.rs` using rusqlite
- [ ] Migration 001: `CREATE TABLE groups (id TEXT PRIMARY KEY, name TEXT, desc TEXT, created_at TEXT)`
- [ ] Migration 002: `CREATE TABLE group_members (group_id TEXT, player_normalized TEXT, added_at TEXT)`
- [ ] `icelines group` commands migrated from JSON to SQLite
- [ ] L1: SQLite group round-trip test (in-memory DB)

### Phase 4-E: Multi-season site + polish

- [ ] `icelines build --season 20242025` — generates a historical season site
- [ ] Site index shows season selector (current default + any installed seasons)
- [ ] `icelines rank --season 20242025` — rank from installed season data
- [ ] `icelines players --season 20242025` — filter from historical season
- [ ] Publish pre-built season data bundles for last 5 seasons as release assets

---

## Success Criteria

1. `icelines data install --seasons 5` downloads and installs in < 30 seconds
2. `icelines history "Connor McDavid" --seasons 5` shows a 5-row career table
3. `icelines project "McDavid" --mode regressed` uses real career PPG (not fallback)
4. TUI Home screen shows all 32 teams with data within 2 seconds of launch
5. `icelines group create` uses SQLite, survives process restart
6. Season bundles for 5 seasons total < 500 KB compressed
7. Data bundle GitHub Actions workflow runs without errors
8. L1 bundle round-trip test passes
9. All existing 135 tests still pass
10. No regression in `icelines team`, `icelines rank`, `icelines build`

---

## Season Bundle URL Convention

```
https://github.com/giodl73-repo/ICELINES/releases/download/data-{SEASON}/data-{SEASON}.tar.gz

Examples:
  data-20252026.tar.gz  — current season
  data-20242025.tar.gz  — last season
  data-20232024.tar.gz  — etc.
```

`icelines data install` fetches from this URL pattern via the GitHub Releases API.
If the release doesn't exist, it falls back to fetching from the NHL API directly
and caches locally.

---

## Out of Scope for Phase 4

- Social signals (Reddit/Twitter) — Tier 5 in data-sources.md
- Beat media line rushes — Tier 6
- xGF% / Corsi (Natural Stat Trick scraping) — Tier 4
- Mouse support in TUI
- Live game score tracking
- Playoff-specific projections
- Goalie scouting reports
