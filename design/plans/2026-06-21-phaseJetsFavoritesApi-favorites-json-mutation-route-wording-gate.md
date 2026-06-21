# Phase Jets Favorites API - Favorites JSON mutation route wording gate

> Phase Jets Favorites API records canonical Favorites API mutations with
> precise intent, result, and named-group boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Jets Favorites API complete

---

## Frame

The Favorites JSON mutation routes already mutate only the canonical
`Favorites` group through shared mutation intents. Phase Jets Favorites API
tightens the route matrix so the rows name `FavoriteMutationIntent::add`,
`FavoriteMutationIntent::remove`, submitted player/team normalization, JSON
`MutationResultView`, and arbitrary named-group editing non-claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Jets Favorites API Goal 1 - Route inventory** | API mutation rows should name canonical group scope and non-claims. | A wave inventory names route rows, evidence, and boundaries. |
| 2 | **Jets Favorites API Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Favorites API mutation tests pass. |
| 3 | **Jets Favorites API Goal 3 - Scoped route wording** | Existing rows are accurate but terse for API mutation safety. | Rows name intent, normalization, result behavior, and named-group non-claims. |
| 4 | **Jets Favorites API Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add arbitrary named-group member editing.
- Do not add group create/rename/delete behavior.
- Do not turn GET read routes into mutations.
- Do not change runtime behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused Favorites API mutation tests passed.
3. **Pulse 03 - Matrix wording.** Result: JSON mutation rows now carry scoped
   canonical-group wording.
4. **Pulse 04 - Closeout.** Result: Phase Jets Favorites API is closed with
   final route-row claims and non-claims recorded.

---

## Closeout

Phase Jets Favorites API closed the Favorites JSON mutation route wording gate.
The rows now record canonical `Favorites` add/remove behavior through
`FavoriteMutationIntent`, player/team input normalization, JSON
`MutationResultView`, and arbitrary named-group editing non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Favorites API mutation route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
