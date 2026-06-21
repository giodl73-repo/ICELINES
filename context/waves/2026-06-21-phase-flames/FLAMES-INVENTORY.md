# Phase Flames Inventory

## Purpose

Inventory the Scores, Schedule, and Playoffs route rows before tightening their
route wording.

## Current Surface

| Area | Evidence | Flames posture |
|---|---|---|
| Scores HTML | `/scores?date=...&range=...` | Keep `ScoresView` date/range rendering with offline-safe 200 HTML behavior. |
| Scores JSON | `/api/v1/scores?date=...&range=...` | Keep standard data/meta envelope with `active_date`, `range`, and `source_error`. |
| Schedule HTML | `/schedule?date=...` | Keep `ScheduleView` date-anchored rendering; TUI-only season-team and matchup projections remain separate. |
| Schedule JSON | `/api/v1/schedule?date=...` | Keep standard data/meta envelope with `active_date`, `active_team`, `team_chips`, and `source_error`. |
| Playoffs HTML | `/playoffs?season=...` | Keep `PlayoffsView` bundled/live bracket rendering and season query acceptance. |
| Playoffs JSON | `/api/v1/playoffs` | Keep standard data/meta envelope with season, round/series counts, and `source_error`. |

## Risks to Avoid

- Claiming live-network success in offline route tests.
- Treating `source_error` metadata as a failing route.
- Claiming Schedule TUI-only season-team or matchup projection parity.
- Claiming playoff predictions, bracket editing, or richer drilldowns.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused route tests cover Scores, Schedule,
   and Playoffs HTML/JSON boundaries.
3. Matrix wording. Result: passed; six route rows now carry scoped wording.
4. Closeout. Result: passed; Phase Flames is closed with final route-row claims
   and non-claims recorded.
