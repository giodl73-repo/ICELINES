# Phase Blues

## Scope

Plan and execute the Fantasy read/product route-row wording gate. The wave does
not add new fantasy behavior or browser mutations; it records the existing
`/fantasy` and `/api/v1/fantasy/*` read/product routes with the boundaries
already captured in the Fantasy family row.

## Entry Posture

- Fantasy league management, roster gaps, simulation, daily delta, matchup, and
  roster-shape surfaces already project through shared ViewModels.
- CLI remains the canonical mutation surface for league/team setup, roster
  import, matchup schedule setup, and roster-shape presets.
- Web/dashboard GET routes stay read/product surfaces and reject or hand off
  mutation-shaped commands.
- The route inventory still uses terse `done` wording for the Fantasy route
  rows.

## Goals

1. Inventory the Fantasy Web/API route rows and their evidence.
2. Validate focused Fantasy Web route evidence.
3. Tighten route-row wording to scoped read/product claims, not browser mutation
   claims.
4. Preserve exact non-claims around GET-backed imports, league/team mutation,
   roster-shape mutation, local state creation on missing reads, and SQLite
   sidecar creation on read-only paths.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Blues goals | passed; see `BLUES-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Fantasy route evidence gate | passed; focused Fantasy route tests support scoped read/product wording, see `pulses/pulse-02.md` |
| 03 | Fantasy route wording gate | pending |
| 04 | Close Phase Blues | pending |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Fantasy Web route tests.
- No live network dependency in tests.
