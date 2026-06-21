# Phase Coyotes Fantasy Detail Inventory

## Purpose

Inventory Fantasy detail JSON route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Coyotes Fantasy Detail posture |
|---|---|---|
| Daily JSON | `GET /api/v1/fantasy/daily` | Keep `FantasyDailyDeltaView` projection from local rosters and cached finalized boxscores with missing-cache source state and no-create behavior. |
| Matchup JSON | `GET /api/v1/fantasy/matchup` | Keep `FantasyMatchupWeekView` projection from local schedule rows and cached daily totals with missing-schedule/cache state. |
| Roster-shape JSON | `GET /api/v1/fantasy/roster-shape` | Keep `RosterShapeValidationView` projection from persisted rosters and canonical positions with CLI handoff for preset mutation. |

## Risks to Avoid

- Claiming browser roster mutation.
- Claiming browser matchup schedule mutation.
- Claiming browser roster-shape preset mutation.
- Claiming live scoring fetch or live recomputation.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Fantasy detail tests cover daily
   missing-cache source state, matchup missing-schedule empty state, and
   roster-shape seeded-team projection.
3. Matrix wording. Result: passed; Fantasy detail rows now carry scoped
   read-only wording.
4. Closeout. Result: passed; Phase Coyotes Fantasy Detail is closed with final
   route-row claims and non-claims recorded.
