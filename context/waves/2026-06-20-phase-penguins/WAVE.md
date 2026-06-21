# Phase Penguins

## Scope

Plan and execute the WP-009 analytics workflow promotion gate. The wave decides
whether selected cache-backed route families can become broader workflow claims
or should remain bounded first-route evidence.

## Entry Posture

- WP-009 has typed analytics cache records, store/read/invalidation fixtures,
  and `AnalyticsCacheConsumerView`.
- Web/API first-route evidence exists for named cache report, coach dashboard,
  opponent scout, player evidence-card, line explorer, goalie readiness,
  practice focus, postgame review, postgame adjustment review, and agent
  evidence.
- The active surface matrix says these are partial because first-route evidence
  does not prove finished workflows.
- Phase Capitals closed without promoting Signals into analytics cache.

## Goals

1. Inventory current analytics-cache route families and promotion blockers.
2. Select one family for workflow-promotion evidence or keep all bounded.
3. Preserve product-copy non-claims for every affected family.
4. Preserve no-live-recompute and no-cache-creation-on-GET behavior.
5. Close the phase with exact surface-matrix wording.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Penguins goals | passed; see `PENGUINS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Select analytics workflow promotion lane | passed; coach dashboard selected for copy/workflow gate, see `PROMOTION-LANE.md` and `pulses/pulse-02.md` |
| 03 | Coach dashboard product-copy gate | passed for bounded prepared-cache dashboard claim; see `COACH-COPY-GATE.md` and `pulses/pulse-03.md` |
| 04 | Coach dashboard workflow evidence and matrix update | passed; focused L2 evidence supports bounded prepared-cache dashboard claim, see `pulses/pulse-04.md` |
| 05 | Close Phase Penguins | passed; phase closed with coach dashboard promoted only to a bounded prepared-cache dashboard claim, see `pulses/pulse-05.md` |

## Closeout

Phase Penguins is closed. The phase promotes only the coach dashboard from
WP-009 first-route evidence to a bounded prepared-cache dashboard claim:
active-context cache reads, ready/unavailable HTML and JSON, no cache creation
on missing reads, and preserved source, quality, methodology, disclosure, and
non-claim copy.

Named cache report, opponent scout, player evidence card, line combinations,
goalie readiness, practice focus, postgame review, postgame adjustments, and
agent evidence remain bounded first-route evidence. They are not promoted to
finished workflow claims without family-specific product-copy and workflow
evidence.

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes use focused `icelines-web` analytics-cache tests.
- No live network dependency in tests.
