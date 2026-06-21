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

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes use focused `icelines-web` analytics-cache tests.
- No live network dependency in tests.
