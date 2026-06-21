# Phase Lightning - Career route wording gate

> Phase Lightning records the Career/cohort route rows as partial by design,
> using the boundaries already closed by Phase Maple Leafs.

**Created:** 2026-06-21
**Status:** Active - evidence gate passed

---

## Frame

Phase Maple Leafs closed the Career/cohort leaders gate. CLI, Web HTML/JSON, and
dashboard summaries use `CareerView`; TUI remains a tested command-bar handoff
to canonical CLI/Web cohort tables because the local career-history store is
optional and unbundled.

The remaining issue is route-row precision. The route inventory still starts
`/career` and `/api/v1/career` with plain `partial -` wording. Phase Lightning
tightens those rows so they match the Maple Leafs partial-by-design posture
without implying a dedicated TUI board or live career-history fetch.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Lightning Goal 1 - Route inventory** | Route-level wording should match Maple Leafs closeout. | A wave inventory names route pairs, evidence, and blockers. |
| 2 | **Lightning Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Career Web route tests pass. |
| 3 | **Lightning Goal 3 - Partial-by-design route wording** | Plain partial wording hides the intentionally scoped claim. | Route rows say partial by design and preserve exact blockers. |
| 4 | **Lightning Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add a dedicated TUI Career/cohort board.
- Do not claim bundled career-history availability.
- Do not claim live fetch or local-store creation from read surfaces.
- Do not change Career route runtime behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused Career Web route tests passed
   and support partial-by-design wording.
3. **Pulse 03 - Matrix wording.** Convert route rows to partial-by-design
   wording only if evidence passes.
4. **Pulse 04 - Closeout.** Close Phase Lightning with exact route claims and
   non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Career Web route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
