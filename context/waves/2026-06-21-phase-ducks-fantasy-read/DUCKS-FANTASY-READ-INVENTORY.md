# Phase Ducks Fantasy Read Inventory

## Purpose

Inventory Fantasy read route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Ducks Fantasy Read posture |
|---|---|---|
| HTML Fantasy | `GET /fantasy` | Keep read-only `FantasyRosterGapView` and `FantasySimulationView` projection, scenario warnings, and setup/import/mutation non-claims. |
| JSON gaps | `GET /api/v1/fantasy/gaps` | Keep existing FantasyDb reads, scoring/category context, missing-db no-create behavior, and SQLite sidecar guards. |
| JSON simulate | `GET /api/v1/fantasy/simulate` | Keep add/drop/drop-only scenario projection and unknown-drop errors without persisted roster mutation. |

## Risks to Avoid

- Claiming browser league/team setup.
- Claiming browser roster import.
- Claiming persisted add/drop/drop-only scenarios.
- Claiming matchup schedule or roster-shape mutation.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Fantasy tests cover HTML scenarios,
   gaps JSON, no-create/sidecar behavior, simulate scenarios, and unknown-drop
   errors.
3. Matrix wording. Result: passed; Fantasy read rows now carry scoped read-only
   wording.
4. Closeout. Result: passed; Phase Ducks Fantasy Read is closed with final
   route-row claims and non-claims recorded.
