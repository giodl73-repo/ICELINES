# Phase Stars Favorites

## Scope

Plan and execute the Favorites read route-row wording gate. The wave does not
add runtime behavior; it records existing read-only Favorites HTML and JSON
evidence.

## Entry Posture

- `/favorites` reads selected SQLite group membership through `FavoritesView`.
- Canonical `Favorites` keeps POST-backed add/remove and cache-load controls.
- Named groups selected by `?group=<name>` are read-only and show CLI handoff
  copy.
- `/api/v1/favorites` returns stable `favorites.v1` rows and group metadata.

## Goals

1. Inventory the Favorites read route rows and evidence.
2. Validate focused Favorites read route evidence.
3. Tighten route-row wording to scoped read-only group projection, canonical
   controls, stat-line cache reads, JSON shape, and non-claim boundaries.
4. Preserve exact non-claims around arbitrary group editing, GET cache creation,
   named-group mutation controls, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Stars Favorites goals | passed; see `STARS-FAVORITES-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Favorites read route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Favorites read route wording gate | passed; rows now carry scoped read wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Stars Favorites | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Favorites read route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Stars Favorites is closed. Favorites rows now record read-only selected
group projection through `FavoritesView`, canonical `Favorites` POST-backed
controls, named-group CLI handoff copy, canonical player/team links, cache-only
stat-line reads, stable `favorites.v1` JSON metadata, and non-claims around
arbitrary group editing and GET cache creation.

The claim remains bounded. The rows do not promote runtime changes, arbitrary
group editing, named-group mutation controls, or GET-created cache state.
