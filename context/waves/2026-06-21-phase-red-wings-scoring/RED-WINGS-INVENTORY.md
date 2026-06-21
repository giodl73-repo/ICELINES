# Phase Red Wings Inventory

## Purpose

Inventory scoring report, outlook, and tonight-intel route rows before tightening
their route wording.

## Current Surface

| Area | Evidence | Red Wings posture |
|---|---|---|
| Game scoring | `/game/:id/scoring`, `/api/v1/game/:id/scoring` | Keep `GameScoringReportView` from cached play-by-play with source-state and no cache creation from GET. |
| Player scoring | `/player/:id/scoring`, `/api/v1/player/:id/scoring` | Keep `PlayerScoringProfileView` filtered to player scoring events. |
| Team scoring | `/team/:abbrev/scoring`, `/api/v1/team/:abbrev/scoring` | Keep `TeamScoringProfileView` filtered to team scoring events with cache-load recovery. |
| Outlook | `/player/:id/outlook`, `/team/:abbrev/outlook` | Keep pace/outlook ViewModels and inline 82-game pace SVG charts when finite positive rows exist. |
| Tonight intel | `/tonight/intel`, `/api/v1/tonight/intel` | Keep favorites-first daily scoring intel with cache-load recovery and no cache creation from GET. |

## Risks to Avoid

- Pulling game detail, streak, analytics-cache, or fantasy claims into this gate.
- Claiming live play-by-play fetches from GET navigation.
- Creating local cache state from read navigation.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Rocket/scoring tests cover cached
   play-by-play, source-state, cache recovery, favorites filtering, and
   no-cache-creation boundaries.
3. Matrix wording. Result: passed; scoring rows now carry scoped wording.
4. Closeout. Result: passed; Phase Red Wings is closed with final route-row
   claims and non-claims recorded.
