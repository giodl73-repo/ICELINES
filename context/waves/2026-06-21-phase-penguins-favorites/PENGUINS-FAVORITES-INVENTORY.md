# Phase Penguins Favorites Inventory

## Purpose

Inventory Favorites HTML mutation route rows before tightening their route
wording.

## Current Surface

| Area | Evidence | Penguins Favorites posture |
|---|---|---|
| HTML add | `POST /favorites/add` | Keep canonical `Favorites` add wording, player/team normalization through `FavoriteMutationIntent`, safe redirect behavior, and add-side best-effort player career augmentation. |
| HTML remove | `POST /favorites/remove` | Keep canonical `Favorites` remove wording, player/team normalization through `FavoriteMutationIntent`, and safe redirect behavior. |
| JSON twins | `POST /api/v1/favorites/add`, `POST /api/v1/favorites/remove` | Already name `MutationResultView`; use as evidence for shared mutation intent behavior without expanding row wording in this phase. |
| Read-only group views | `/favorites?group=<name>` | Keep arbitrary named-group selection read-only; do not imply named-group member editing. |

## Risks to Avoid

- Treating named-group reads as named-group mutation support.
- Claiming GET navigation can mutate membership.
- Broadening dashboard workspace allowlists to unsafe POST paths.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Favorites mutation tests cover JSON
   mutation result views and HTML form redirect/validation behavior.
3. Matrix wording. Result: passed; HTML mutation rows now carry scoped
   canonical-group wording.
4. Closeout. Result: passed; Phase Penguins Favorites is closed with final
   route-row claims and non-claims recorded.
