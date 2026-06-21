# Phase Utah Inventory

## Purpose

Inventory Scouting and Game detail route rows before tightening their route
wording.

## Current Surface

| Area | Evidence | Utah posture |
|---|---|---|
| Scouting HTML | `/scouting/:id` | Keep player-card-backed `ReportView` rendered through the scouting template. |
| Scouting JSON | `/api/v1/scouting/:id` | Keep the same `ReportView` contract with report id, title, format, and section metadata. |
| Game HTML | `/game/:id` | Keep `GameView` boxscore detail rendering and offline-safe rendered error page behavior. |
| Game JSON | `/api/v1/game/:id` | Keep data/meta envelope with `game_id` and `source_error` carrying fetch failures. |

## Risks to Avoid

- Pulling scoring report rows into this gate.
- Claiming live game fetch success.
- Adding scouting sections or player-card fields.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused scouting and game tests cover report
   wrapping, Markdown rendering, game HTML, and game JSON source-error metadata.
3. Matrix wording. Result: passed; four route rows now carry scoped wording.
4. Closeout. Result: passed; Phase Utah is closed with final route-row claims
   and non-claims recorded.
