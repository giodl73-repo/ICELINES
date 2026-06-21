# Phase Golden Knights Inventory

## Purpose

Inventory the Poach board/report route rows before converting terse `done`
wording into scoped shared-ViewModel wording.

## Current Surface

| Area | Evidence | Golden Knights posture |
|---|---|---|
| Poach HTML | `/poach` | Keep shared `PoachBoardView` board rendering with filters and missing-source disclosure. |
| Poach report HTML | `/reports/poach` | Keep `PoachReportView` report rendering from the board, including descriptive poach-score SVG for positive rows. |
| Weekly report HTML | `/reports/weekly` | Keep weekly prep sections projected through `PoachReportView`. |
| Poach JSON | `/api/v1/poach` | Keep the board ViewModel JSON contract intentionally outside the shared API envelope. |
| Imported availability reads | `/poach` and `/api/v1/poach` filters | Keep read-only FantasyDb access without SQLite WAL/SHM sidecar creation. |

## Risks to Avoid

- Claiming browser fantasy league/team mutation support.
- Treating `/api/v1/poach` as the standard shared API envelope.
- Claiming reports recompute or persist fantasy roster state.
- Creating SQLite WAL/SHM sidecars on read-only imported-availability reads.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Poach Web route tests support scoped
   shared-ViewModel route wording.
3. Matrix wording. Result: passed; the four Poach route rows now carry scoped
   shared-ViewModel wording while preserving API-envelope and read-only SQLite
   boundaries.
4. Closeout. Record final claims and non-claims.
