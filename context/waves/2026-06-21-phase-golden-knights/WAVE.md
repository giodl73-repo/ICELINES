# Phase Golden Knights

## Scope

Plan and execute the Poach board/report route-row wording gate. The wave does
not add new poacher behavior; it records the existing `/poach`,
`/reports/poach`, `/reports/weekly`, and `/api/v1/poach` routes with scoped
shared-ViewModel wording.

## Entry Posture

- The Poacher board family row already says CLI/TUI/Web/JSON share
  `PoachBoardView`.
- Poach and weekly reports render from `PoachReportView`, including resolved
  scoring categories, source omissions, and inline descriptive score SVGs.
- Imported-availability Web reads use read-only SQLite paths and must not create
  WAL/SHM sidecars.
- The route inventory still uses terse `done` wording for the individual Poach
  route rows.

## Goals

1. Inventory the Poach route rows and evidence.
2. Validate focused Poach Web route evidence.
3. Tighten route-row wording to scoped shared-ViewModel claims, not broad fantasy
   league mutation claims.
4. Preserve exact non-claims around read-only imported-roster reads, report
   generation scope, shared API envelope differences, and sidecar creation.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Golden Knights goals | passed; see `GOLDEN-KNIGHTS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Poach route evidence gate | passed; focused Poach route tests support scoped shared-ViewModel wording, see `pulses/pulse-02.md` |
| 03 | Poach route wording gate | pending |
| 04 | Close Phase Golden Knights | pending |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Poach Web route tests.
- No live network dependency in tests.
