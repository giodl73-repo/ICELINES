# Phase Utah Pulse 03 - Matrix Wording

**Date:** 2026-06-21
**Result:** Passed

## Work Completed

- Tightened Scouting HTML/JSON route rows into scoped player-card-backed
  `ReportView` claims.
- Tightened Game HTML/JSON route rows into scoped `GameView` and
  `meta.source_error` claims.
- Preserved scoring route and live-fetch non-claims.

## Validation

- `git diff --check`

## Outcome

The route inventory now records the Scouting/Game detail route evidence
precisely.
