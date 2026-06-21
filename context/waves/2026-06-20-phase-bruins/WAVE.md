# Phase Bruins

## Scope

Plan and execute the opponent-scout analytics workflow promotion gate. The wave
decides whether the WP-009 opponent scout Web/API first-route evidence can
become a bounded prepared-cache scout report claim, or whether it should remain
only first-route evidence.

## Entry Posture

- Phase Penguins promoted only coach dashboard to a bounded prepared-cache
  dashboard claim.
- The active surface matrix still keeps opponent scout as WP-009 first-route
  evidence.
- `/scout/opponent` and `/api/v1/scout/opponent` default to
  `opponent_scout:<season>:<type>` and render through
  `AnalyticsCacheConsumerView`.
- Existing L2 tests cover ready and unavailable route behavior.

## Goals

1. Inventory opponent-scout route evidence, product copy, and promotion
   blockers.
2. Decide whether opponent scout can be promoted to a bounded prepared-cache
   scout report claim.
3. Preserve explicit non-claims: no full scouting suite, no opponent game-plan
   workflow, no live recomputation, no prediction certainty, and no autonomous
   coaching authority.
4. Preserve no-cache-creation-on-GET and prepared-cache read behavior.
5. Close the phase with exact surface-matrix wording.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Bruins goals | passed; see `BRUINS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Opponent scout product-copy gate | passed for bounded prepared-cache scout report claim; see `BRUINS-COPY-GATE.md` and `pulses/pulse-02.md` |
| 03 | Opponent scout workflow evidence and matrix update | passed; focused L2 evidence supports bounded prepared-cache scout report claim, see `pulses/pulse-03.md` |
| 04 | Close Phase Bruins | passed; phase closed with opponent scout promoted only to a bounded prepared-cache scout report claim, see `pulses/pulse-04.md` |

## Closeout

Phase Bruins is closed. The phase promotes only opponent scout from WP-009
first-route evidence to a bounded prepared-cache scout report claim:
active-context cache reads, ready/unavailable HTML and JSON, no cache creation
on missing reads, and preserved source, quality, methodology, disclosure, and
non-claim copy.

Named cache report, player evidence card, line combinations, goalie readiness,
practice focus, postgame review, postgame adjustments, and agent evidence remain
bounded first-route evidence. Opponent scout is still not a full scouting suite,
opponent game-plan workflow, live recomputation surface, prediction-certainty
surface, or autonomous coaching authority.

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes use focused `icelines-web` analytics-cache tests.
- No live network dependency in tests.
