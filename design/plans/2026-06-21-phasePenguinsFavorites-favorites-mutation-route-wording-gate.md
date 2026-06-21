# Phase Penguins Favorites - Favorites mutation route wording gate

> Phase Penguins Favorites records the HTML Favorites add/remove mutation rows
> with precise canonical-group and redirect boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Penguins Favorites complete

---

## Frame

The Favorites add/remove routes already exist as POST-backed mutations scoped to
the canonical `Favorites` group. Phase Penguins Favorites tightens the route
matrix so the HTML form twins name `FavoriteMutationIntent`, safe redirect
behavior, best-effort player career augmentation on add, and named-group editing
non-claims without changing runtime behavior.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Penguins Favorites Goal 1 - Route inventory** | Favorites mutation rows should name the canonical group, mutation intent, redirects, and deferred named-group editing. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Penguins Favorites Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Favorites mutation route tests pass. |
| 3 | **Penguins Favorites Goal 3 - Scoped route wording** | Existing HTML rows are accurate but omit redirect and add-side augment boundaries. | Route rows name canonical `Favorites` add/remove behavior, safe redirects, add-side augmentation, and named-group non-claims. |
| 4 | **Penguins Favorites Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change Favorites runtime behavior.
- Do not add arbitrary named-group member editing.
- Do not convert read-only `?group=<name>` navigation into mutation behavior.
- Do not broaden dashboard workspace allowlists.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused Favorites mutation tests passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped HTML
   mutation wording.
4. **Pulse 04 - Closeout.** Result: Phase Penguins Favorites is closed with
   final route-row claims and non-claims recorded.

---

## Closeout

Phase Penguins Favorites closed the Favorites mutation route wording gate. The
HTML add/remove rows now record canonical `Favorites` mutation intent, safe
redirect behavior, add-side best-effort player career augmentation, and
arbitrary named-group editing non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Favorites mutation route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
