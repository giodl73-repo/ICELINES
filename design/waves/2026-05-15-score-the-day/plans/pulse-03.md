---
wave: score-the-day
pulse: 03
date: 2026-05-15
status: complete
governing_roles:
  - wire
  - bench
  - forge
---

# Pulse 03 - Cached Boxscore and FantasyDb Data Path

## Goal

Build the offline data path that turns a fantasy league roster plus cached,
finalized game lines for a date into the core daily-delta ViewModel.

## Owned Scope

- Reuse `FantasyDb` league/team/roster snapshots.
- Read cached game data through existing manifest/store helpers where available.
- Surface missing cache, no active league, no user team, and unfinalized games as
  explicit errors or warnings.
- Add fixture-based tests; no live NHL calls.

## Non-goals

- No new public surface yet.
- No destructive DB operations.
- No season-to-date snapshot subtraction fallback.

## Gates

- [x] `cargo test -p icelines-fetch fantasy_daily --quiet`
- [x] `cargo fmt --check`

## Result

Added `icelines_fetch::fantasy_daily::build_fantasy_daily_delta_view`, which
combines `FantasyDb` league snapshots with cached boxscore manifest entries for
a date, projects finalized skater/goalie lines into the core daily-delta
ViewModel, and surfaces missing cache, missing user team, and unfinalized lines
without live NHL calls.
