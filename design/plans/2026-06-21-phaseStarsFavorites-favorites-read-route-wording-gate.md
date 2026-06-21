# Phase Stars Favorites - Favorites read route wording gate

> Phase Stars Favorites records the Favorites HTML/JSON read route rows with
> precise read-only group and cache-boundary wording.

**Created:** 2026-06-21
**Status:** Closed - Phase Stars Favorites complete

---

## Frame

The Favorites HTML route and JSON twin already project selected SQLite group
membership through `FavoritesView`. Phase Stars Favorites tightens the route
matrix so the rows name canonical `Favorites` controls, read-only named-group
views, canonical link resolution, cache-only stat-line reads, stable
`favorites.v1` metadata, and non-claims around arbitrary group editing.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Stars Favorites Goal 1 - Route inventory** | Favorites read rows should name group selection, controls, stat-line cache reads, and JSON metadata. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Stars Favorites Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Favorites read tests pass. |
| 3 | **Stars Favorites Goal 3 - Scoped route wording** | Existing rows are accurate but terse beside recent Watchlist read rows. | Route rows name read-only projection, canonical controls, cache-only reads, JSON shape, and non-claims. |
| 4 | **Stars Favorites Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change Favorites runtime behavior.
- Do not add arbitrary group create/rename/delete/member editing.
- Do not create cache state from GET navigation.
- Do not move POST-backed add/remove or cache-load controls onto named groups.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused Favorites read tests passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped read
   wording.
4. **Pulse 04 - Closeout.** Result: Phase Stars Favorites is closed with final
   route-row claims and non-claims recorded.

---

## Closeout

Phase Stars Favorites closed the Favorites read route wording gate. The rows now
record read-only group projection, canonical `Favorites` controls, named-group
CLI handoff copy, canonical player/team links, cache-only stat-line reads,
stable `favorites.v1` JSON metadata, and arbitrary group-editing/cache-creation
non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Favorites read route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
