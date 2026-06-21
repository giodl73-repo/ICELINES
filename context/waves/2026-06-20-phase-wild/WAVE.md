# Phase Wild

## Scope

Plan and execute the line-combination explorer analytics workflow promotion
gate. The wave decides whether the WP-009 line-combination Web/API first-route
evidence can become a bounded prepared-cache line-combination explorer claim, or
whether it should remain only first-route evidence.

## Entry Posture

- Phase Penguins promoted only coach dashboard to a bounded prepared-cache
  dashboard claim.
- Phase Bruins promoted only opponent scout to a bounded prepared-cache scout
  report claim.
- Phase Stars promoted only player evidence card to a bounded prepared-cache
  player evidence-card claim.
- The active surface matrix still keeps line-combination explorer as WP-009
  first-route evidence.
- `/lines/explorer` and `/api/v1/lines/explorer` default to
  `line_combination_explorer:<season>:<type>` and render through
  `AnalyticsCacheConsumerView`.
- Existing L2 tests cover ready and unavailable route behavior.

## Goals

1. Inventory line-combination route evidence, product copy, and promotion
   blockers.
2. Decide whether line-combination explorer can be promoted to a bounded
   prepared-cache line-combination explorer claim.
3. Preserve explicit non-claims: no deployment advice, no line-chemistry
   causality, no roster authority, no live recomputation, no prediction
   certainty, and no autonomous coaching authority.
4. Preserve no-cache-creation-on-GET and prepared-cache read behavior.
5. Close the phase with exact surface-matrix wording.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Wild goals | passed; see `WILD-INVENTORY.md` and `pulses/pulse-01.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes use focused `icelines-web` analytics-cache tests.
- No live network dependency in tests.
