---
wave: score-the-day
pulse: 03
date: 2026-05-15
status: planned
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

- [ ] focused fetch/CLI adapter tests for daily fantasy data path
- [ ] `cargo fmt --check`
