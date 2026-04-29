# IceLines Changelog

## Unreleased

### Added
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
