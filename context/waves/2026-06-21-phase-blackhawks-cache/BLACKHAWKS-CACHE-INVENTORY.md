# Phase Blackhawks Cache Inventory

## Purpose

Inventory admin game-cache warmer route rows before tightening their route
wording.

## Current Surface

| Area | Evidence | Blackhawks Cache posture |
|---|---|---|
| Explicit team JSON warmer | `POST /api/v1/admin/game-cache/load` | Keep POST-backed cache warmer wording, input validation before network/cache writes, per-game boxscore/play-by-play artifact scope, and JSON summary return. |
| Explicit team HTML warmer | `POST /admin/game-cache/load` | Keep HTML form twin wording with safe source-page redirect and summary/error derivation. |
| Favorites JSON warmer | `POST /api/v1/admin/game-cache/load-favorites` | Keep POST-backed Favorites warmer wording, season validation, favorite player career team/season artifacts, favorite team active-year artifacts, and JSON summary return. |
| Favorites HTML warmer | `POST /admin/game-cache/load-favorites` | Keep HTML form twin wording with admin/source-page redirect and cache-warmer-only boundary. |

## Risks to Avoid

- Treating cache warmers as release data install/remove operations.
- Claiming arbitrary Favorites group/member editing.
- Claiming GET navigation creates cache artifacts.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused admin game-cache tests cover invalid
   request rejection before network/cache work.
3. Matrix wording. Result: passed; admin game-cache rows now carry scoped
   POST-backed warmer wording.
4. Closeout. Result: passed; Phase Blackhawks Cache is closed with final
   route-row claims and non-claims recorded.
