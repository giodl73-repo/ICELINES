# Phase Sharks Watch - Watch-rule delete route wording gate

> Phase Sharks Watch records the watch-rule delete route row with precise
> persisted-rule and destructive-boundary wording.

**Created:** 2026-06-21
**Status:** Closed - Phase Sharks Watch complete

---

## Frame

The watch-rule delete route already exists as a POST-backed HTML form mutation
scoped to persisted player watch-rule ids. Phase Sharks Watch tightens the route
matrix so the row names `WatchRuleMutationIntent::delete`, blank/unknown id
handling, single-row deletion, redirect behavior, and the non-claim around
arbitrary destructive rule dimensions.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Sharks Watch Goal 1 - Route inventory** | The delete row should name the shared intent and destructive boundary. | A wave inventory names route row, evidence, and non-claims. |
| 2 | **Sharks Watch Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused watch-rule delete route test passes. |
| 3 | **Sharks Watch Goal 3 - Scoped route wording** | Existing wording is accurate but too terse beside create/toggle rows. | Route row names form-only delete, intent resolution, id rejection, single-row deletion, redirect, and non-claims. |
| 4 | **Sharks Watch Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change watch-rule runtime behavior.
- Do not add JSON delete or bulk delete routes.
- Do not add arbitrary team/deployment rule editing.
- Do not broaden destructive rule dimensions beyond persisted player rule ids.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused watch-rule delete test passed.
3. **Pulse 03 - Matrix wording.** Result: route row now carries scoped
   destructive-boundary wording.
4. **Pulse 04 - Closeout.** Result: Phase Sharks Watch is closed with final
   route-row claims and non-claims recorded.

---

## Closeout

Phase Sharks Watch closed the watch-rule delete route wording gate. The row now
records form-only persisted player watch-rule deletion, shared mutation-intent
resolution, blank/unknown id rejection, single-row deletion, `/watchlist`
redirect behavior, and arbitrary destructive-dimension non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused watch-rule delete route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
