# Phase Coyotes Fantasy Detail

## Scope

Plan and execute the Fantasy daily/matchup/roster-shape JSON route-row wording
gate. The wave does not add runtime behavior; it records existing Fantasy detail
read evidence.

## Entry Posture

- `/api/v1/fantasy/daily` returns `FantasyDailyDeltaView` from local rosters and
  cached finalized boxscores.
- `/api/v1/fantasy/matchup` returns `FantasyMatchupWeekView` from local
  matchup schedules plus cached daily totals.
- `/api/v1/fantasy/roster-shape` returns `RosterShapeValidationView` for
  persisted rosters and canonical player positions.
- Browser mutations, live fetches, and live recomputation remain out of scope.

## Goals

1. Inventory Fantasy detail JSON route rows and evidence.
2. Validate focused Fantasy detail JSON route evidence.
3. Tighten route-row wording to ViewModel projection, local/cached source
   boundaries, missing-source warnings, no-create behavior, and browser mutation
   non-claims.
4. Preserve exact non-claims around browser roster/matchup/shape mutation, live
   fetch, live recomputation, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Coyotes Fantasy Detail goals | passed; see `COYOTES-FANTASY-DETAIL-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Fantasy detail route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Fantasy detail route wording gate | passed; rows now carry scoped read-only wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Coyotes Fantasy Detail | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Fantasy detail route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Coyotes Fantasy Detail is closed. Fantasy detail JSON rows now record
`FantasyDailyDeltaView`, `FantasyMatchupWeekView`, and
`RosterShapeValidationView` projections from local state, explicit source-state
warnings, missing-cache no-create behavior, and browser mutation non-claims.

The claim remains bounded. The rows do not promote browser roster mutation,
matchup schedule mutation, roster-shape preset mutation, live scoring fetch,
live recomputation, or runtime behavior changes.
