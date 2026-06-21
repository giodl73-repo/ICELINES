# Phase Devils

## Scope

Plan and execute the Player/Team streak route-row wording gate. The wave does
not add runtime behavior; it records existing cache-backed streak route evidence.

## Entry Posture

- `/player/:id/streaks` and `/api/v1/player/:id/streaks` project
  `PlayerStreaksView`.
- `/team/:abbrev/streaks` and `/api/v1/team/:abbrev/streaks` project
  `TeamPlayerStreaksView`.
- Existing tests cover empty-state cache-load recovery, shared envelopes,
  shot-metric/source-state exposure, and no local cache creation from GET
  navigation.
- Scoring report and analytics-cache routes remain separate.

## Goals

1. Inventory the streak route rows and evidence.
2. Validate focused streak route evidence.
3. Tighten route-row wording to scoped ViewModel, source-state, recovery, and
   no-cache-creation claims.
4. Preserve exact non-claims around scoring reports, game detail,
   analytics-cache, season-total inference, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Devils goals | passed; see `DEVILS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Streak route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Streak route wording gate | passed; rows now carry scoped ViewModel wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Devils | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused streak route tests.
- No live network dependency in tests.

## Closeout

Phase Devils is closed. Streak route rows now record `PlayerStreaksView` and
`TeamPlayerStreaksView`, cached source-state, shot metrics, recovery forms,
shared envelopes, and no local cache creation from read navigation.

The claim remains bounded. The rows do not promote scoring reports, game detail,
analytics-cache, season-total inference, or runtime behavior changes.
