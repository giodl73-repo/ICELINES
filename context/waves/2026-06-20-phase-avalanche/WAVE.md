# Phase Avalanche

## Scope

Plan and execute the goalie-readiness analytics workflow promotion gate. The
wave decides whether the WP-009 goalie-readiness Web/API first-route evidence
can become a bounded prepared-cache goalie readiness workload claim, or whether
it should remain only first-route evidence.

## Entry Posture

- Phase Wild promoted only line-combination explorer to a bounded prepared-cache
  explorer claim.
- The active surface matrix still keeps goalie readiness as WP-009 first-route
  evidence.
- `/goalies/readiness` and `/api/v1/goalies/readiness` default to
  `goalie_readiness:<season>:<type>` and render through
  `AnalyticsCacheConsumerView`.
- Existing L2 tests cover ready and unavailable route behavior.

## Goals

1. Inventory goalie-readiness route evidence, product copy, and promotion
   blockers.
2. Decide whether goalie readiness can be promoted to a bounded prepared-cache
   goalie readiness workload claim.
3. Preserve explicit non-claims: no injury certainty, no start/sit authority, no
   medical or betting advice, no live recomputation, no prediction certainty,
   and no autonomous coaching authority.
4. Preserve no-cache-creation-on-GET and prepared-cache read behavior.
5. Close the phase with exact surface-matrix wording.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Avalanche goals | passed; see `AVALANCHE-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Goalie-readiness product-copy gate | passed for bounded prepared-cache goalie readiness workload claim; see `AVALANCHE-COPY-GATE.md` and `pulses/pulse-02.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes use focused `icelines-web` analytics-cache tests.
- No live network dependency in tests.
