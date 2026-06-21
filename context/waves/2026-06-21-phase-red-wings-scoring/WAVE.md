# Phase Red Wings

## Scope

Plan and execute the scoring route-row wording gate. The wave does not add
runtime behavior; it records existing cached scoring, outlook, and tonight-intel
route evidence.

## Entry Posture

- Game, player, and team scoring routes project cached play-by-play scoring
  ViewModels.
- Player and team outlook routes project pace/outlook ViewModels with SVG chart
  affordances when finite positive rows exist.
- Tonight intel routes project favorites-first daily scoring intel.
- Existing Rocket tests cover cached play-by-play reads, cache-load recovery,
  favorites filtering, and no local data cache creation from GET routes.

## Goals

1. Inventory the scoring route rows and evidence.
2. Validate focused scoring route evidence.
3. Tighten route-row wording to scoped ViewModel, source-state, recovery, SVG,
   and no-cache-creation claims.
4. Preserve exact non-claims around game detail, streaks, analytics-cache,
   fantasy, live fetches, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Red Wings goals | passed; see `RED-WINGS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Scoring route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Scoring route wording gate | passed; rows now carry scoped ViewModel wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Red Wings | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Rocket/scoring route tests.
- No live network dependency in tests.

## Closeout

Phase Red Wings is closed. Scoring route rows now record cached play-by-play
source-state, scoring report ViewModels, outlook pace ViewModels and SVGs,
favorites-first tonight intel, cache-load recovery, and no local cache creation
from read navigation.

The claim remains bounded. The rows do not promote game detail, streaks,
analytics-cache, fantasy, live fetches, or runtime behavior changes.
