# Phase Utah

## Scope

Plan and execute the Scouting/Game detail route-row wording gate. The wave does
not add runtime behavior; it records existing route evidence for scouting report
and game detail surfaces.

## Entry Posture

- `/scouting/:id` and `/api/v1/scouting/:id` wrap `PlayerCardView` in a
  rendered `ReportView`.
- Scouting handler tests cover the player-card-to-report wrapper and Markdown
  rendering.
- `/game/:id` and `/api/v1/game/:id` render `GameView`-backed boxscore detail.
- Game route tests cover HTML 200/content type and JSON data/meta envelopes
  with fetch failures carried in `meta.source_error`.
- Scoring report routes remain separate.

## Goals

1. Inventory the Scouting/Game route rows and evidence.
2. Validate focused scouting and game test evidence.
3. Tighten route-row wording to scoped `ReportView` and `GameView` claims.
4. Preserve exact non-claims around scoring report routes, live fetch success,
   new scouting sections, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Utah goals | passed; see `UTAH-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Scouting/game evidence gate | passed; focused tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Scouting/game route wording gate | passed; rows now carry scoped `ReportView`/`GameView` wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Utah | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused scouting and game tests.
- No live network success dependency in tests.

## Closeout

Phase Utah is closed. Scouting and Game detail route rows now record
player-card-backed `ReportView` output, scouting JSON metadata, `GameView`
boxscore detail, and game JSON source-error metadata.

The claim remains bounded. The rows do not promote scoring report routes, live
fetch success, new scouting sections, or runtime behavior changes.
