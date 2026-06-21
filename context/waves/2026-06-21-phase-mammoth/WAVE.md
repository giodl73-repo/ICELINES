# Phase Mammoth

## Scope

Plan and execute the Compare/Depth/Records route-row wording gate. The wave does
not add runtime behavior; it records existing route evidence for read-only
comparison, cross-team depth, and individual-record surfaces.

## Entry Posture

- `/compare` and `/api/v1/compare` project `CompareView` and, for similarity
  mode, `SimilarPlayersView`.
- `/depth` and `/api/v1/depth` project `DepthLeagueView`.
- Records routes project `PlayerRecordsView` and `TeamRecordsView`.
- Existing tests cover compare envelopes, row identity, similarity mode, career
  trend SVGs, bad-input errors, depth envelopes and row identity, records metric
  selection, and team records empty-state links.

## Goals

1. Inventory the Compare/Depth/Records route rows and evidence.
2. Validate focused compare, depth, and records route evidence.
3. Tighten route-row wording to scoped ViewModel, metric, chart, envelope, and
   empty-state claims.
4. Preserve exact non-claims around scoring, streaks, analytics-cache, fantasy,
   new records metrics, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Mammoth goals | passed; see `MAMMOTH-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Compare/depth/records route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Compare/depth/records route wording gate | passed; rows now carry scoped ViewModel wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Mammoth | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused compare, depth, and records Web route tests.
- No live network dependency in tests.

## Closeout

Phase Mammoth is closed. Compare, Depth, and Records route rows now record their
shared ViewModels, similarity mode, row identity, metric selection, charts,
envelopes, and empty-state handoffs.

The claim remains bounded. The rows do not promote scoring, streak,
analytics-cache, fantasy, new records metrics, or runtime behavior changes.
