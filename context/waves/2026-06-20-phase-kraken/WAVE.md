# Phase Kraken

## Scope

Plan and execute the practice-focus analytics workflow promotion gate. The wave
decides whether the WP-009 practice-focus Web/API first-route evidence can
become a bounded prepared-cache practice focus report claim, or whether it
should remain only first-route evidence.

## Entry Posture

- Phase Avalanche promoted only goalie readiness to a bounded prepared-cache
  workload claim.
- The active surface matrix still keeps practice focus as WP-009 first-route
  evidence.
- `/practice/focus` and `/api/v1/practice/focus` default to
  `practice_focus:<season>:<type>` and render through
  `AnalyticsCacheConsumerView`.
- Existing L2 tests cover ready and unavailable route behavior.

## Goals

1. Inventory practice-focus route evidence, product copy, and promotion
   blockers.
2. Decide whether practice focus can be promoted to a bounded prepared-cache
   practice focus report claim.
3. Preserve explicit non-claims: no mandatory drill plan, no autonomous practice
   prescription, no coaching authority, no live recomputation, and no prediction
   certainty.
4. Preserve no-cache-creation-on-GET and prepared-cache read behavior.
5. Close the phase with exact surface-matrix wording.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Kraken goals | passed; see `KRAKEN-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Practice-focus product-copy gate | passed for bounded prepared-cache practice focus report claim; see `KRAKEN-COPY-GATE.md` and `pulses/pulse-02.md` |
| 03 | Practice-focus workflow evidence and matrix update | passed; focused L2 evidence supports bounded prepared-cache practice focus report claim, see `pulses/pulse-03.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes use focused `icelines-web` analytics-cache tests.
- No live network dependency in tests.
