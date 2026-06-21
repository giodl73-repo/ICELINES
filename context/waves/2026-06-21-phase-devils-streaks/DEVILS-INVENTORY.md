# Phase Devils Inventory

## Purpose

Inventory Player and Team streak route rows before tightening their route
wording.

## Current Surface

| Area | Evidence | Devils posture |
|---|---|---|
| Player streak HTML | `/player/:id/streaks` | Keep `PlayerStreaksView` rendering from cached boxscore/play-by-play rows with cache-load recovery and no cache creation from GET. |
| Player streak JSON | `/api/v1/player/:id/streaks` | Keep standard envelope with `PlayerStreaksView`, source-state, loaded counts, and shot metrics. |
| Team streak HTML | `/team/:abbrev/streaks` | Keep `TeamPlayerStreaksView` rendering with cache-load recovery and no cache creation from GET. |
| Team streak JSON | `/api/v1/team/:abbrev/streaks` | Keep standard envelope with `TeamPlayerStreaksView`, source-state, loaded counts, and shot metrics. |

## Risks to Avoid

- Pulling scoring report, game detail, or analytics-cache claims into this gate.
- Inferring streaks from season totals.
- Creating local cache state from GET navigation.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused streak tests cover empty-state
   recovery, shared envelopes, source-state, shot metrics, and no-cache-creation
   boundaries.
3. Matrix wording. Result: passed; four route rows now carry scoped wording.
4. Closeout. Result: passed; Phase Devils is closed with final route-row claims
   and non-claims recorded.
