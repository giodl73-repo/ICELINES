---
wave: score-the-day
pulse: 01
date: 2026-05-15
status: complete
governing_roles:
  - pace
  - bench
  - wire
  - forge
---

# Pulse 01 - Daily Delta Inventory and Pulse Map

## Goal

Open the Score the Day wave and define an executable pulse map for fantasy daily
delta scoring.

## Owned Scope

- Inspect existing fantasy scoring, FantasyDb, fantasy ViewModels, and cached
  game-night schemas.
- Create `FANTASY-DAILY-DELTA-INVENTORY.md`.
- Create pulse plans and role-review panels.
- Add the wave to `design/waves/PHASES.md`.

## Non-goals

- No runtime behavior.
- No new database migration.
- No live data fetch.

## Gates

- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-score-the-day design\waves\PHASES.md --errors-only`

## Result

Opened Score the Day with daily fantasy points scoped to cached finalized
boxscores, shared core scoring contracts, and read-only surfaces.
