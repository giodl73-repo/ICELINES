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
| 03 | Goalie-readiness workflow evidence and matrix update | passed; focused L2 evidence supports bounded prepared-cache goalie readiness workload claim, see `pulses/pulse-03.md` |
| 04 | Close Phase Avalanche | passed; phase closed with goalie readiness promoted only to a bounded prepared-cache workload claim, see `pulses/pulse-04.md` |

## Closeout

Phase Avalanche is closed. The phase promotes only goalie readiness from WP-009
first-route evidence to a bounded prepared-cache goalie readiness workload
claim: active-context cache reads, ready/unavailable HTML and JSON, no cache
creation on missing reads, preserved source, quality, methodology, disclosure,
and non-claim copy, and no injury-certainty or deployment-recommendation copy.

Named cache report, practice focus, postgame review, postgame adjustments, and
agent evidence remain bounded first-route evidence. Goalie readiness is still
not injury certainty, medical advice, start/sit authority, live recomputation,
prediction certainty, or autonomous coaching authority.

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes use focused `icelines-web` analytics-cache tests.
- No live network dependency in tests.
