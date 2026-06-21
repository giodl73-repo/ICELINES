# Phase Ducks - Favorites/watch route wording gate

> Phase Ducks records individual Favorites/watch route rows as scoped partials
> by design, using the product boundary already closed by Phase Red Wings.

**Created:** 2026-06-21
**Status:** Active - evidence gate passed

---

## Frame

Phase Red Wings closed the Favorites/watch/watch-rules boundary gate. The
feature rows already say the current surface is partial by design: read-only
named group views, POST-backed canonical Favorites add/remove, watchlist and
watch-rule reads, and player-rule create/toggle/delete are supported and
tested.

The remaining issue is route-row precision. The individual route inventory still
starts those paths with plain `partial -` wording. Phase Ducks tightens those
rows so future readers can tell intentional product boundaries from unresolved
route drift.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Ducks Goal 1 - Route inventory** | Route-level wording should match the Red Wings closeout. | A wave inventory names supported Favorites/watch routes and deferrals. |
| 2 | **Ducks Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Favorites/watch route tests pass. |
| 3 | **Ducks Goal 3 - Partial-by-design route wording** | Plain partial wording hides intentional boundaries. | Route rows say partial by design and preserve exact blockers. |
| 4 | **Ducks Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add arbitrary group create/rename/delete/member editing.
- Do not add arbitrary team/deployment watch-rule editing.
- Do not mutate favorites, groups, or watch rules through GET navigation.
- Do not reinterpret unsupported dashboard/TUI commands as narrower mutations.
- Do not claim favorite stat-line reads create cache state or fetch live data on
  GET.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused Favorites/watch route tests
   passed and support scoped route wording.
3. **Pulse 03 - Matrix wording.** Convert route rows to explicit partial by
   design wording only if evidence passes.
4. **Pulse 04 - Closeout.** Close Phase Ducks with exact route claims and
   non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Favorites/watch route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
