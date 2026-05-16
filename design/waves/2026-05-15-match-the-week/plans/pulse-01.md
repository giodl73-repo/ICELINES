---
wave: match-the-week
pulse: 01
date: 2026-05-15
status: complete
governing_roles:
  - pace
  - bench
  - wire
  - forge
  - glass
---

# Pulse 01 - Matchup Inventory and Pulse Map

## Goal

Open the Match the Week wave and define an executable pulse map for fantasy
weekly head-to-head matchups.

## Owned Scope

- Inspect existing FantasyDb, Score the Day daily delta, date/timeframe, and
  surface parity rails.
- Create `FANTASY-MATCHUP-INVENTORY.md`.
- Create pulse plans and role-review panels.
- Add the wave to `design/waves/PHASES.md`.
- Mark the Tier 3 backlog item as active.

## Non-goals

- No runtime behavior.
- No database migration yet.
- No live data fetch.
- No Yahoo/private schedule import.

## Gates

- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-match-the-week design\waves\PHASES.md design\plans\INDEX.md --errors-only`

## Result

Opened Match the Week with weekly head-to-head scoring scoped to local matchup
schedule rows plus cached finalized daily fantasy points.
