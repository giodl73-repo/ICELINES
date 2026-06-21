# Phase Golden Knights - Poach route wording gate

> Phase Golden Knights records Poach board/report routes with precise scoped
> shared-ViewModel wording.

**Created:** 2026-06-21
**Status:** Active - planning complete

---

## Frame

Poach board and report surfaces already project through shared ViewModels:
`PoachBoardView` for the board and `/api/v1/poach`, and `PoachReportView` for
poach and weekly report pages. Imported-availability reads use read-only
FantasyDb paths and are guarded against SQLite WAL/SHM sidecar creation.

The remaining issue is route-row precision. The route inventory still uses
terse `done` wording for the individual Poach rows. Phase Golden Knights
tightens those rows without implying fantasy league mutation support or shared
API envelope behavior where it does not exist.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Golden Knights Goal 1 - Route inventory** | Route-level wording should match the Poach family row. | A wave inventory names route rows, evidence, and blockers. |
| 2 | **Golden Knights Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Poach Web route tests pass. |
| 3 | **Golden Knights Goal 3 - Scoped route wording** | Terse `done` wording hides ViewModel and read-only boundaries. | Route rows name ViewModels and preserve exact blockers. |
| 4 | **Golden Knights Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add browser fantasy league/team mutation support.
- Do not change `/api/v1/poach` into the shared API envelope.
- Do not change Poach runtime behavior.
- Do not weaken read-only SQLite sidecar guards.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Run focused Poach Web route tests.
3. **Pulse 03 - Matrix wording.** Convert route rows to scoped wording only if
   evidence passes.
4. **Pulse 04 - Closeout.** Close Phase Golden Knights with exact route claims
   and non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Poach Web route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
