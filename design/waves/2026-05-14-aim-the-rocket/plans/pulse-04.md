# Pulse 04 - Game and Team Scoring Reports

## Goal

Ship the first Rocket Richard scoring report surfaces: game-level and
team-season scoring reports built from the typed play-by-play scoring events
and manifest-backed cache source states.

## Governing roles

- **tape**: only official NHL play-by-play events feed the report rows; game ID,
  team owner, shooter/scorer, period/time, situation, and coordinates must flow
  from the parser/provider without renderer inference.
- **edge**: missing play-by-play, loaded zero-event play-by-play, and team games
  outside the active season/type must remain distinct.
- **wire**: reports read from `DataKind::PlayByPlay`; empty states may offer
  POST-backed cache load actions, but no GET-backed mutations.
- **bench**: tests must cover HTML and JSON route shape, source-state metadata,
  and provider behavior with tempdir fixtures.
- **glass**: pages should be glanceable: summary counts first, then split rows
  and event detail, with a clear cache-loading recovery path.

## Owned scope

1. Add core report summary rows so renderers do not compute scoring splits.
2. Add DataStore-backed providers for game and team scoring report ViewModels.
3. Add `/game/:id/scoring` and `/team/:abbrev/scoring` HTML routes.
4. Add `/api/v1/game/:id/scoring` and `/api/v1/team/:abbrev/scoring` JSON
   twins.
5. Update route inventory, surface parity, COMMANDS, and wave evidence.

## Non-goals

- No xG, high-danger buckets, rink plots, or coordinate normalization.
- No TUI/CLI scoring-report command yet.
- No live NHL fetch from report GET routes; load actions stay POST-backed
  through the existing admin game-cache endpoint.
- No player scoring profile route; that remains Pulse 06.

## Implementation result

- Added core scoring split rows for team, period, situation, and top-shooter
  summaries so renderers consume ViewModel data instead of recomputing report
  logic.
- Added `load_team_scoring_profile` and used existing
  `load_game_scoring_report` over manifest-backed `DataKind::PlayByPlay`.
- Added HTML and JSON report routes:
  `/game/:id/scoring`, `/api/v1/game/:id/scoring`,
  `/team/:abbrev/scoring`, and `/api/v1/team/:abbrev/scoring`.
- Added route tests for cached game scoring JSON, team scoring JSON filtering,
  and team HTML missing-cache recovery.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-fetch --quiet`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo clippy -p icelines-core -p icelines-fetch -p icelines-web -- -D warnings`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-14-aim-the-rocket design\waves\PHASES.md COMMANDS.md design\specs\surface-parity.md --errors-only`
