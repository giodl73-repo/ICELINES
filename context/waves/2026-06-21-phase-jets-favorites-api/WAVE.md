# Phase Jets Favorites API

## Scope

Plan and execute the Favorites JSON mutation route-row wording gate. The wave
does not add runtime behavior; it records existing canonical `Favorites` API
add/remove evidence.

## Entry Posture

- JSON add/remove routes mutate only the canonical `Favorites` group.
- Routes normalize submitted player/team input through `FavoriteMutationIntent`.
- Routes return `MutationResultView`.
- Arbitrary named-group member editing remains out of scope.

## Goals

1. Inventory Favorites JSON mutation route rows and evidence.
2. Validate focused Favorites API mutation route evidence.
3. Tighten route-row wording to canonical group scope, shared intent,
   player/team input normalization, JSON result, and named-group non-claims.
4. Preserve exact non-claims around arbitrary group editing, group management,
   GET mutation, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Jets Favorites API goals | passed; see `JETS-FAVORITES-API-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Favorites API mutation route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Favorites API mutation route wording gate | passed; rows now carry scoped canonical-group wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Jets Favorites API | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Favorites API mutation route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Jets Favorites API is closed. Favorites JSON mutation rows now record
canonical `Favorites` add/remove behavior, `FavoriteMutationIntent`
normalization, JSON `MutationResultView`, and arbitrary named-group editing
non-claims.

The claim remains bounded. The rows do not promote arbitrary named-group member
editing, group management, GET mutation, or runtime behavior changes.
