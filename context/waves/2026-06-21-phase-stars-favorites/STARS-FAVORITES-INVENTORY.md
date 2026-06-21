# Phase Stars Favorites Inventory

## Purpose

Inventory Favorites read route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Stars Favorites posture |
|---|---|---|
| HTML Favorites | `GET /favorites` | Keep read-only selected-group `FavoritesView` projection, group chips, canonical links, canonical `Favorites` controls, named-group CLI handoff copy, and cache-only stat-line wording. |
| JSON Favorites | `GET /api/v1/favorites` | Keep stable `favorites.v1` payload with selected group/count metadata, player/team rows, nullable `stat_line`, and read-only named-group selection. |
| Mutation boundary | POST add/remove and cache-load controls | Keep mutation controls canonical to `Favorites`; do not imply named-group member editing. |

## Risks to Avoid

- Claiming arbitrary group create/rename/delete/member editing.
- Claiming named-group mutation controls.
- Claiming GET navigation creates manifest or boxscore cache state.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Favorites read tests cover canonical
   links, no cache creation, HTML read-only group selection, JSON membership
   shape, and JSON named-group reads.
3. Matrix wording. Result: passed; Favorites rows now carry scoped read-only
   wording.
4. Closeout. Result: passed; Phase Stars Favorites is closed with final
   route-row claims and non-claims recorded.
