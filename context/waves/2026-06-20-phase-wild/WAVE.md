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
| 02 | Line-combination product-copy gate | passed for bounded prepared-cache line-combination explorer claim; see `WILD-COPY-GATE.md` and `pulses/pulse-02.md` |
| 03 | Line-combination workflow evidence and matrix update | passed; focused L2 evidence supports bounded prepared-cache line-combination explorer claim, see `pulses/pulse-03.md` |
| 04 | Close Phase Wild | passed; phase closed with line-combination explorer promoted only to a bounded prepared-cache line-combination explorer claim, see `pulses/pulse-04.md` |

## Closeout

Phase Wild is closed. The phase promotes only line-combination explorer from
WP-009 first-route evidence to a bounded prepared-cache line-combination
explorer claim: active-context cache reads, ready/unavailable HTML and JSON, no
cache creation on missing reads, preserved source, quality, methodology,
disclosure, and non-claim copy, and no guaranteed-chemistry or deployment
recommendation copy.

Named cache report, goalie readiness, practice focus, postgame review, postgame
adjustments, and agent evidence remain bounded first-route evidence.
Line-combination explorer is still not deployment advice, roster authority,
line-chemistry causality, live recomputation, prediction certainty, or
autonomous coaching authority.

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes use focused `icelines-web` analytics-cache tests.
- No live network dependency in tests.
