# Phase Flames

## Scope

Plan and execute the Scores/Schedule/Playoffs route-row wording gate. The wave
does not add new slate behavior; it records existing HTML and JSON routes with
scoped ViewModel and source-error wording.

## Entry Posture

- `/scores` and `/api/v1/scores` project `ScoresView`.
- `/schedule` and `/api/v1/schedule` project `ScheduleView`.
- `/playoffs` and `/api/v1/playoffs` project `PlayoffsView`.
- Existing tests cover date/range/season route acceptance and JSON data/meta
  envelopes with `source_error` metadata.
- The route inventory still uses short project wording for these rows.

## Goals

1. Inventory the slate route rows and evidence.
2. Validate focused Scores, Schedule, and Playoffs route evidence.
3. Tighten route-row wording to scoped ViewModel and envelope claims.
4. Preserve exact non-claims around live-network success, TUI-only schedule
   projections, and playoff prediction/editing behavior.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Flames goals | passed; see `FLAMES-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Slate route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Slate route wording gate | passed; rows now carry scoped ViewModel/source-error wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Flames | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Scores, Schedule, and Playoffs Web route tests.
- No live network success dependency in tests.

## Closeout

Phase Flames is closed. Scores, Schedule, and Playoffs route rows now record
their shared ViewModel projections, supported query parameters, JSON envelope
metadata, and `source_error` handling.

The claim remains bounded. The rows do not promise live-network success,
Schedule TUI-only projection parity, playoff predictions, or bracket editing.
