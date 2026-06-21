# Phase Penguins Favorites

## Scope

Plan and execute the Favorites mutation route-row wording gate. The wave does
not add runtime behavior; it records existing POST-backed canonical `Favorites`
add/remove evidence.

## Entry Posture

- Web HTML add/remove routes mutate only the canonical `Favorites` group.
- HTML mutations normalize player/team input through `FavoriteMutationIntent`
  and redirect to a safe `return_to` target or `/favorites`.
- Add routes may launch best-effort player career-history augmentation after the
  mutation.
- Named-group selection through `/favorites?group=<name>` remains read-only.

## Goals

1. Inventory the Favorites mutation route rows and evidence.
2. Validate focused Favorites mutation route evidence.
3. Tighten route-row wording to scoped canonical group, mutation intent,
   redirect, add-side augmentation, and named-group non-claims.
4. Preserve exact non-claims around arbitrary named-group editing, GET
   mutation, dashboard unsafe workspace allowlists, and runtime behavior
   changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Penguins Favorites goals | passed; see `PENGUINS-FAVORITES-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Favorites mutation route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Favorites mutation route wording gate | passed; rows now carry scoped HTML mutation wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Penguins Favorites | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Favorites mutation route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Penguins Favorites is closed. Favorites HTML mutation route rows now
record canonical `Favorites` add/remove behavior, `FavoriteMutationIntent`
normalization, safe redirects, add-side best-effort player career augmentation,
and arbitrary named-group editing non-claims.

The claim remains bounded. The rows do not promote runtime changes, arbitrary
named-group editing, GET mutation, or unsafe dashboard workspace routing.
