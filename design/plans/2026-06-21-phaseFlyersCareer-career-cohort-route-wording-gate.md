# Phase Flyers Career - Career cohort route wording gate

> Phase Flyers Career records the Career cohort route rows with precise
> read-only local-store and envelope boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Flyers Career complete

---

## Frame

The `/career` HTML route and `/api/v1/career` JSON twin already project
cross-league cohort rows from the optional local career-history store through
`CareerView`. Phase Flyers Career tightens the route matrix so the rows name
query validation, `top` capping, shared shell/envelope behavior, missing-store
fetch guidance, and no-live-fetch/no-store-create non-claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Flyers Career Goal 1 - Route inventory** | Career rows should name query shape, local-store projection, shell/envelope behavior, and missing-store guidance. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Flyers Career Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused career route tests pass. |
| 3 | **Flyers Career Goal 3 - Scoped route wording** | Existing rows are accurate but do not fully name validation and envelope boundaries. | Route rows name read-only cohort behavior, validation, `CareerView`, fetch guidance, and non-claims. |
| 4 | **Flyers Career Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change Career route runtime behavior.
- Do not create or fetch career-history data from GET navigation.
- Do not claim bundled career-history availability.
- Do not add a dedicated TUI cohort board.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused career route tests passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped
   local-store cohort wording.
4. **Pulse 04 - Closeout.** Result: Phase Flyers Career is closed with final
   route-row claims and non-claims recorded.

---

## Closeout

Phase Flyers Career closed the Career cohort route wording gate. The rows now
record read-only cohort leaderboard behavior, `league` plus optional
`season`/`sort`/`top` validation, `top` capping, `CareerView` projection from the
local store, shared HTML shell and JSON envelope behavior, missing-store fetch
guidance, and no-live-fetch/no-store-create non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Career route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
