---
wave: match-the-week
pulse: 03
date: 2026-05-15
status: complete
governing_roles:
  - wire
  - forge
  - bench
---

# Pulse 03 - FantasyDb Schedule and Weekly Builder

## Goal

Persist local matchup schedule rows and build weekly matchup views from cached
daily fantasy scoring.

## Owned Scope

- Add FantasyDb migration and APIs for matchup schedule rows.
- Support explicit byes for odd-sized leagues.
- Add a fetch-layer `build_fantasy_matchup_week_view` that resolves an input
  date through `Timeframe::Week`, walks each day, reuses the daily-delta builder,
  and aggregates team totals into the core matchup ViewModel.
- Preserve explicit source-state/warnings for missing cache, unfinalized games,
  missing schedule rows, missing teams, and unknown scoring schemes.
- Add L1/fetch tests with in-memory FantasyDb and temp DataStore fixtures; no
  live network.

## Non-goals

- No Yahoo/private schedule import.
- No web/CLI/TUI surface wiring.
- No GET-backed mutation.

## Gates

- [x] `cargo test -p icelines-fetch fantasy_matchup --quiet`
- [x] `cargo fmt --check`

## Result

Added local `fl_matchups` schedule persistence to `FantasyDb`, including
explicit byes, duplicate team-slot rejection, and same-team rejection. Added
`icelines_fetch::fantasy_matchup::build_fantasy_matchup_week_view`, which
resolves ISO weeks through `Timeframe::Week`, reuses the daily-delta builder for
each date, aggregates team totals, and preserves missing schedule/cache and
unfinalized-game source state.
