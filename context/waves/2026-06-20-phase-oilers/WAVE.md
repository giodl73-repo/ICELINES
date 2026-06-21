# Phase Oilers

## Scope

Plan and execute the named analytics-cache report closeout gate. The wave
decides whether `/reports/analytics-cache` and
`/api/v1/reports/analytics-cache` can be described as a bounded generic
prepared-cache inspection surface, while preserving that they are not a specific
hockey workflow.

## Entry Posture

- Phase Canucks promoted the final workflow-style cache family, agent evidence,
  to a bounded prepared-cache evidence summary claim.
- The active surface matrix still keeps the named analytics cache report as
  generic WP-009 first-route evidence.
- `/reports/analytics-cache` and `/api/v1/reports/analytics-cache` require a
  named `cache_key` and `metrics` query and render through
  `AnalyticsCacheConsumerView`.
- Existing L2 tests cover ready and unavailable behavior without recomputing
  analytics.

## Goals

1. Inventory named-report route evidence, product copy, and non-workflow
   blockers.
2. Decide whether the named report can be promoted to a bounded generic
   prepared-cache inspection claim.
3. Preserve explicit non-claims: no specific hockey workflow, no live
   recomputation, no fetch-on-read, no prediction certainty, and no coaching or
   recommendation authority.
4. Preserve explicit unavailable state and prepared-cache read behavior.
5. Close the phase with exact surface-matrix wording.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Oilers goals | passed; see `OILERS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Named report product-copy gate | passed for bounded generic prepared-cache inspection claim; see `OILERS-COPY-GATE.md` and `pulses/pulse-02.md` |
| 03 | Named report evidence and matrix update | passed; focused L2 evidence supports bounded generic prepared-cache inspection claim, see `pulses/pulse-03.md` |
| 04 | Close Phase Oilers | passed; phase closed with named analytics cache report promoted only to bounded generic prepared-cache inspection, see `pulses/pulse-04.md` |

## Closeout

Phase Oilers is closed. The named analytics cache report is promoted only to
bounded generic prepared-cache inspection: explicit named `cache_key` and metric
list, ready/unavailable HTML and JSON states, preserved source, quality,
methodology, disclosure, and non-claim copy, and no recomputation or live fetch
on read.

It remains outside coaching, scouting, player, line, goalie, practice,
postgame, and agent workflows, and does not claim prediction certainty,
recommendation authority, or autonomous behavior. The WP-009 cache promotion
sequence is complete with each family bounded by its phase-specific claims and
non-claims.

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes use focused `icelines-web` analytics-cache tests.
- No live network dependency in tests.
