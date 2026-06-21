# Phase Canadiens

## Scope

Plan and execute the Leaders/Goalies route-row wording gate. The wave does not
add leaderboard behavior; it records existing route evidence for skater and
goalie leaderboard surfaces.

## Entry Posture

- `/leaders` and `/api/v1/leaders` project through `LeadersView`.
- `/goalies` and `/api/v1/goalies` project through `GoaliesView`.
- Leaders tests cover full dashboard embedding, preserved query state, and the
  Pts/82 SVG chart.
- Goalies tests cover JSON envelope metadata, CLI-parity sort/minimum filters,
  advanced workload metrics, and the SV% SVG chart.
- The route inventory can be more explicit about the separate skater and goalie
  contracts.

## Goals

1. Inventory the Leaders/Goalies route rows and evidence.
2. Validate focused leaderboard route evidence.
3. Tighten route-row wording to scoped ViewModel, filter, chart, and envelope
   claims.
4. Preserve exact non-claims around metric expansion, persistence, browser
   interaction QA, and merging skater/goalie contracts.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Canadiens goals | passed; see `CANADIENS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Leaderboard route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Leaderboard route wording gate | passed; rows now carry scoped `LeadersView`/`GoaliesView` wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Canadiens | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Leaders and Goalies Web route tests.
- No live network dependency in tests.

## Closeout

Phase Canadiens is closed. Leaders and Goalies route rows now record their
separate shared ViewModels, query/filter handling, chart evidence, and JSON
envelope metadata.

The claim remains bounded. The rows do not add new metrics, persistence,
browser interaction QA, or a merged skater/goalie leaderboard contract.
