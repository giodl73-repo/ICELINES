# IceLines Changelog

## Unreleased

### Added
- Phase 8f.5: `icelines scheme show NAME --source` prints the scheme as
  pretty JSON instead of the human-readable table. Useful for copy/paste,
  diffing two schemes, or piping into `jq`. The default (no flag) still
  emits the readable layout. Scheme already derived Serialize so the
  change is minimal. 1 L2 test verifies valid JSON with name + skater +
  goalie fields.
- Phase 8f.4: `--season YYYYZZZZ` flag on `query leaders/player/compare`.
  Pins the query to a specific bundled season instead of the current one
  — `icelines query leaders --season 20242025 --top 10` shows last
  season's leaders without changing config. Validates against
  `icelines_fetch::BUNDLED_SEASONS` (currently 2021-22 → 2025-26) and
  rejects unknown seasons with a copyable hint listing the bundled IDs.
  Mutually exclusive with `--seasons N` (the multi-season aggregate);
  combining the two errors with a clear explanation. New
  `load_all_players_for_season(Option<&str>)` helper backs all three
  query commands. 3 L0 validator tests + 5 L2 subprocess tests
  (success, unbundled-error, conflict-error, player + compare paths).
- Phase 8c: Historical playoffs bundle. New `playoffs_bundle` module
  defines the `PlayoffsBundle` JSON schema (rounds → series → per-game
  results with optional goal scorers) and a `to_bracket()` conversion
  that drops cleanly into the existing TUI render path. `bundled::
  load_playoffs(season)` resolves installed bundle first then the
  binary-embedded copy. Hand-authored `data/seasons/19931994/
  playoffs.json` ships as the first fixture — full 4-round NYR Cup run
  with per-game results for the Cup Final. `tui::playoffs` now consults
  bundled data before any network call; historical seasons never hit
  the live API. `render_series_body` renders the per-game log when
  present (Game N · date · home N–N away · series-after) and falls
  back to the existing "X game(s) played" hint for live-API series.
  Closes the `Per-game scores + scorers ship with playoffs.json (v2)`
  TODO. 11 L0 tests in `playoffs_bundle` + `bundled`, 5 L0 tests in
  TUI cache + render paths, 2 L1 integration tests covering the full
  load → convert → render chain.
- Phase 8f.2 + 8f.3: snapshot prune + diff
  - `icelines snapshot prune --keep N [--dry-run]` keeps the newest N
    sealed snapshots per tier and deletes the rest. Active snapshot is
    always preserved; drafts are excluded from the keep count. Pair with
    `snapshot gc` to reclaim chunk space. 5 L0 + 2 L2 tests.
  - `icelines snapshot diff <A> <B>` compares two chunked snapshots and
    reports player-level changes (added / removed / changed bios /
    changed stats). O(n) hash-set diff via the chunked layout — exact
    and fast. Legacy snapshots error with a hint to run `rebuild
    --chunked` first. 4 L0 + 1 L2 tests.
- Phase 8f.1: live-feeds toggle — `--no-live` global CLI flag,
  `ICELINES_NO_LIVE` env var, and `live = false` config key all suppress
  NHL API fetches in Scores / Schedule / Playoffs / boxscore + the auto-
  refresh timer. Precedence: CLI > env > config > default(on). When
  disabled, each live tab renders an explicit "Live feeds disabled —
  re-enable with …" message via the standard error path. 4 L0 precedence
  tests + 2 L2 (flag accepted globally, `--help` documents it).
- Phase 8d: `icelines export md <shape>` — writes deterministic markdown
  tables with YAML front-matter for proof DASHBOARD-SPEC consumption.
  Five shapes shipped: `leaders`, `team`, `depth`, `compare`, `roster`.
  `fantasy` and `series` are stubbed with deferred messages (need
  FantasyDb glue and historical playoffs.json respectively). Output
  goes to `~/.icelines/reports/{shape}.md` by default; pass `--out -`
  for stdout. 13 L0 tests + 3 L2 subprocess tests. `export-markdown.md`
  spec status flipped from `Planned` → `Implemented (partial)`.
- Phase 8h: Chunked snapshot store — content-addressed per-player chunks
  with SHA-256 deduplication. New `icelines-fetch::chunkstore` module
  (put/get/exists/delete with sharded layout); `SnapshotStore` extended
  with `write_chunked_stats`, `read_chunked_stats`, `is_chunked`, refs
  table (`chunkrefs.json`), `gc_chunks`, `recompute_refs`,
  `rebuild_chunked` (legacy → chunked migration). Two new CLI ops:
  `icelines snapshot rebuild --chunked <name>` and
  `icelines snapshot gc [--dry-run]`. `bundled::load_*_with_fallback`
  prefers chunked active snapshot then falls back to legacy → bundled.
  25 new tests (12 ChunkStore + 11 chunked snapshot + 2 L2). Storage
  reduction: ~10–15× for daily-cadence snapshots over a season.
- Phase 8b: Scores auto-refresh — live Scores tab polls every 30s. New
  `should_auto_refresh` pure function + `App::tick_auto_refresh` driven
  from the TUI event loop. "Updated Xs ago" indicator in the Scores
  title. Timer arms on tab entry / `t` jump to today, disarms on date
  change, never fires on past dates. 8 L0 + 2 render tests.
- Phase 8a: Test catch-up for previously-shipped features. 27 new tests:
  - **Scouting** (`commands/scouting.rs`) — extracted `validate_format` +
    pure `render_report() -> String`; 7 L0 tests cover all 3 formats,
    section presence, low-GP path; 3 L2 subprocess tests verify exit
    codes and JSON parseability.
  - **Admin overlay** (`tui/app.rs`, `screens/misc.rs`) — 5 L0 keystroke
    tests (capital `F` toggle, Esc closes, Tab blocked, lowercase f
    untouched) + 4 L0 render tests (Idle / Downloading / Done / Error
    phases). Added `InstallState::force_phase` for deterministic test
    drives.
  - **Headshot rendering** (`tui/headshot.rs`) — extracted
    `pixels_to_braille()` + `DOT_X` / `DOT_Y` / `THRESHOLD` constants;
    8 L0 tests cover braille bit layout, threshold contract, cache
    round-trip, loading/error placeholders, Arc-shared clone semantics.
- 10 new specs in `design/specs/` covering previously-homeless features:
  group-management, fantasy-leagues, data-bundles, site-generation,
  scouting-reports, scheme-customization, snapshot-operations,
  tui-admin-overlay, export-markdown (planned), headshot-rendering
  (reference). Specs INDEX updated.
- Phase 7c gap-fix: Scores tab — date navigation (`←/→`), `d` date picker
  (ISO or `MM/DD`), `t` back-to-today, per-game boxscore detail
  (goals/assists, goalies, series context for playoffs), per-date and
  per-boxscore caches
- Phase 7e: Playoffs tab — list-style bracket from `/v1/playoff-bracket/{year}`,
  per-round navigation, per-series detail with summary and "if needed" game
  hints, off-season / error states, `r` retry
- Phase 7d: Schedule tab — weekly view with date navigation, team and matchup
  search (`/SEA`, `/NYR WSH`), team-season detail and head-to-head matchup
  views, per-week + per-team caches with `r` retry, `t` jump to today
- TUI guide (`docs/guides/06-tui.md`) — covers Phase 7a–7e: six-tab nav,
  season time-travel, Scores, Schedule with search and matchups, Playoffs
  bracket and series detail
- `icelines query compare --comps` — contract comparable finder (in progress)
- Season data expansion to 2000-01 (in progress)

---

## v1.0.0 — 2026-04-26

IceLines v1: migrated from C:\src\NHL\fantasy-tracker to C:\src\icelines.
Clean repo structure matching proof/mdpath conventions.

### Architecture
- 4-crate Rust workspace: icelines-core, icelines-fetch, icelines-site, icelines-cli
- 5 seasons bundled in binary (20212022–20252026, ~4.3MB total)
- PlayerRepository — single authoritative data loading API
- 338 tests: L0 unit, L1 integration, L2 system + mock NHL API fixture

### Data pipeline
- NHL API client: bios, stats, realtime, rosters, contracts, schedule
- MoneyPuck xG/CF%/FF% integration (silo'd, optional)
- NHL realtime stats: hits, blocked_shots, giveaways, takeaways, PIM
- Snapshot store with SHA-256 integrity, provenance chain, tiered architecture
- Contract data: expiry_year, expiry_type (UFA/RFA/ELC)

### Player model
- 50+ fields covering all-situations, PP, SH, shot metrics, physical, bio, draft, contract
- Multi-season aggregate (`--seasons N`) across bundled history
- Y/Y improvement sort (`--sort improvement`)
- Duplicate player dedup (NHL API emits multiple rows for traded players)

### Commands
- `icelines fetch` — rosters, stats, realtime, positions, contracts, moneypuck
- `icelines query leaders/player/compare` — 30+ sort metrics, percentiles, JSON/CSV
- `icelines fantasy` — SQLite leagues/teams, scoring, trade simulation, axum HTTP server
- `icelines rank/team/players/history/project/scouting/mates/peers/class/compare`
- `icelines group/scheme/snapshot/data/tui/tonight/schedule`
- `icelines build/serve/deploy` — mkdocs static site

### Repo process
- CLAUDE.md — session context, crate ownership, rules
- CODEBASE.md — where to write code, full module map
- design/ — specs, plans, invariants, pitfalls
- docs/ — generated output, team pages
- .roles/ — 8 domain review roles
- design/plans/INDEX.md, design/specs/INDEX.md
