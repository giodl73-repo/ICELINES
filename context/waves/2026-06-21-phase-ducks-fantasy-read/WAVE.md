# Phase Ducks Fantasy Read

## Scope

Plan and execute the Fantasy read/product route-row wording gate. The wave does
not add runtime behavior; it records existing Fantasy HTML, gaps JSON, and
simulate JSON evidence.

## Entry Posture

- `/fantasy` renders `FantasyRosterGapView` and `FantasySimulationView`.
- `/api/v1/fantasy/gaps` reads existing FantasyDb state without creating user
  state or SQLite WAL/SHM sidecars.
- `/api/v1/fantasy/simulate` projects add/drop/drop-only scenarios without
  persisting roster mutations.
- Browser league/team setup, roster import, matchup schedule mutation, and
  roster-shape mutation remain out of scope.

## Goals

1. Inventory Fantasy read route rows and evidence.
2. Validate focused Fantasy HTML/gaps/simulate route evidence.
3. Tighten route-row wording to ViewModel projection, existing-db read behavior,
   no-create/sidecar guards, scenario warning/error behavior, and mutation
   non-claims.
4. Preserve exact non-claims around browser setup/import/mutation and runtime
   behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Ducks Fantasy Read goals | passed; see `DUCKS-FANTASY-READ-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Fantasy read route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Fantasy read route wording gate | passed; rows now carry scoped read-only wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Ducks Fantasy Read | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Fantasy read route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Ducks Fantasy Read is closed. Fantasy read rows now record
`FantasyRosterGapView` and `FantasySimulationView` projection from existing
FantasyDb state, read-only gaps and scenario JSON behavior, missing-db and
SQLite sidecar guards, unknown-drop warnings/errors, and browser mutation
non-claims.

The claim remains bounded. The rows do not promote browser league/team setup,
roster import, persisted scenarios, matchup schedule mutation, roster-shape
mutation, or runtime behavior changes.
