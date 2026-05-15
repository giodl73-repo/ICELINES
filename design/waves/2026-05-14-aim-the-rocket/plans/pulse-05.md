# Pulse 05 - Tonight Scoring Intelligence

## Goal

Turn Rocket Richard scoring reports into a favorites-first daily surface:
"what scoring pressure is loaded for my favorite players and teams tonight?"

## Governing roles

- **scout**: favorite-player/team rows must explain useful scoring context
  without pretending to know projected lineups or betting odds.
- **wire**: the route reads cached `DataKind::PlayByPlay`; cache loading remains
  POST-backed through the existing Admin game-cache forms.
- **glass**: summary first, favorite teams/players next, then recovery action.
- **bench**: tests must cover favorite filtering and missing-cache recovery using
  local fixtures, not network calls.

## Owned scope

1. Extend `TonightScoringIntelView` with favorite team/player rows.
2. Add a DataStore-backed provider for a date + favorite teams/players.
3. Add `/tonight/intel` and `/api/v1/tonight/intel`.
4. Update route inventory, surface parity, COMMANDS, and wave evidence.

## Non-goals

- No projected lineups, betting odds, confirmed goalies, or third-party scraping.
- No TUI tab yet; web/API lead this pulse.
- No automatic GET-backed fetching.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-fetch --quiet`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo clippy -p icelines-core -p icelines-fetch -p icelines-web -- -D warnings`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-14-aim-the-rocket design\waves\PHASES.md COMMANDS.md design\specs\surface-parity.md --errors-only`
