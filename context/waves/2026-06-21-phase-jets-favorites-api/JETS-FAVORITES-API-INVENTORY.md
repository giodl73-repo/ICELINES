# Phase Jets Favorites API Inventory

## Purpose

Inventory Favorites JSON mutation route rows before tightening their route
wording.

## Current Surface

| Area | Evidence | Jets Favorites API posture |
|---|---|---|
| JSON add | `POST /api/v1/favorites/add` | Keep canonical `Favorites` add through `FavoriteMutationIntent::add`, player/team input normalization, and `MutationResultView`. |
| JSON remove | `POST /api/v1/favorites/remove` | Keep canonical `Favorites` remove through `FavoriteMutationIntent::remove`, player/team input normalization, and `MutationResultView`. |

## Risks to Avoid

- Claiming arbitrary named-group member editing.
- Claiming group create/rename/delete behavior.
- Claiming GET-backed mutation.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Favorites API tests cover JSON add
   and remove `MutationResultView` behavior.
3. Matrix wording. Result: passed; API mutation rows now carry scoped
   canonical-group wording.
4. Closeout. Result: passed; Phase Jets Favorites API is closed with final
   route-row claims and non-claims recorded.
