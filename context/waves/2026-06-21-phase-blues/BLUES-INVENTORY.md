# Phase Blues Inventory

## Purpose

Inventory the Fantasy read/product route rows before converting terse `done`
wording into scoped route-row wording.

## Current Surface

| Area | Evidence | Blues posture |
|---|---|---|
| Fantasy HTML | `/fantasy` | Keep read/product route over `FantasyRosterGapView` and `FantasySimulationView`, including scenario warnings. |
| Fantasy gaps JSON | `/api/v1/fantasy/gaps` | Keep read-only `FantasyRosterGapView` JSON with missing-db and SQLite sidecar guards. |
| Fantasy simulation JSON | `/api/v1/fantasy/simulate` | Keep `FantasySimulationView` JSON with add/drop/drop-only scenario resolution and explicit errors. |
| Fantasy daily JSON | `/api/v1/fantasy/daily` | Keep `FantasyDailyDeltaView` over local FantasyDb snapshots plus cached finalized boxscores with missing/unfinalized source-state. |
| Fantasy matchup JSON | `/api/v1/fantasy/matchup` | Keep `FantasyMatchupWeekView` over local matchup rows plus cached finalized daily-delta totals. |
| Fantasy roster-shape JSON | `/api/v1/fantasy/roster-shape` | Keep read-only `RosterShapeValidationView`; roster-shape mutation remains CLI handoff. |

## Risks to Avoid

- Claiming browser league/team setup or Yahoo roster import.
- Claiming GET-backed roster-shape mutation or matchup schedule mutation.
- Creating `~/.icelines` state on missing JSON reads.
- Creating SQLite WAL/SHM sidecars on read-only Web reads.
- Weakening scenario-resolution error copy for invalid add/drop inputs.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Fantasy Web route tests support
   scoped read/product route wording.
3. Matrix wording. Convert the six Fantasy route rows to scoped read/product
   wording if evidence passes.
4. Closeout. Record final claims and non-claims.
