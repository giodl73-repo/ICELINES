# Phase Blackhawks Cache

## Scope

Plan and execute the admin game-cache route-row wording gate. The wave does not
add runtime behavior; it records existing POST-backed cache-warmer evidence.

## Entry Posture

- Admin game-cache JSON and HTML mutations support explicit active-season team
  cache warming.
- Admin Favorites game-cache JSON and HTML mutations support favorite player
  career team/season artifact warming plus favorite team active-year warming.
- Invalid admin game-cache inputs are rejected before network/cache work.
- The admin data matrix already treats these controls as cache warmers, not
  release bundle install/remove operations.

## Goals

1. Inventory the admin game-cache route rows and evidence.
2. Validate focused admin game-cache route evidence.
3. Tighten route-row wording to scoped POST-backed warmer, validation,
   artifact, redirect, summary, and non-install/remove claims.
4. Preserve exact non-claims around release data install/remove, arbitrary
   Favorites editing, GET navigation warming, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Blackhawks Cache goals | passed; see `BLACKHAWKS-CACHE-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Admin game-cache route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Admin game-cache route wording gate | passed; rows now carry scoped cache-warmer wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Blackhawks Cache | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused admin game-cache route tests.
- Invalid-request evidence has no live network dependency.

## Closeout

Phase Blackhawks Cache is closed. Admin game-cache route rows now record
POST-backed explicit team and Favorites cache warmers with validation before
network/cache work, bounded per-game artifact scope, summary/redirect behavior,
and explicit release data install/remove non-claims.

The claim remains bounded. The rows do not promote runtime changes, release
bundle install/remove routes, arbitrary Favorites editing, or GET-triggered
cache warming.
