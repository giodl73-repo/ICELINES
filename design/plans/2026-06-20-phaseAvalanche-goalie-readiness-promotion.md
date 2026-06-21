# Phase Avalanche - Goalie readiness promotion gate

> Phase Avalanche decides whether WP-009 goalie-readiness Web/API first-route
> evidence can become a bounded prepared-cache goalie readiness workload claim,
> or whether it remains first-route evidence only.

**Created:** 2026-06-20
**Status:** Active - pulse 03 goalie evidence gate passed

---

## Frame

Phase Wild promoted only line-combination explorer. The active surface matrix
still keeps goalie readiness partial: `/goalies/readiness` and
`/api/v1/goalies/readiness` prove prepared-cache Web/API route behavior, but not
injury certainty, start/sit authority, medical advice, or a broader goalie
workflow.

Phase Avalanche audits that boundary and only strengthens the claim if product
copy and workflow evidence remain precise.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Avalanche Goal 1 - Goalie-readiness inventory** | Evidence is split across WP-009 tests, route docs, and the surface matrix. | A wave inventory names the route pair, cache key, ViewModel, tests, and blockers. |
| 2 | **Avalanche Goal 2 - Product-copy gate** | Goalie-readiness copy must not imply injury certainty, start/sit authority, prediction certainty, or live recomputation. | Accepted or deferred wording is recorded with explicit non-claims. |
| 3 | **Avalanche Goal 3 - Workflow evidence gate** | A goalie-readiness claim needs more than one rendered cache record. | Focused evidence proves ready/unavailable states, no recomputation, and no cache creation on missing reads. |
| 4 | **Avalanche Goal 4 - Surface matrix closeout** | The matrix must distinguish bounded workload behavior from a finished goalie workflow. | `design/specs/surface-parity.md` carries exact final wording. |

---

## Non-goals

- Do not add new analytics formulas or live recomputation.
- Do not claim injury certainty, medical advice, start/sit authority, betting,
  matchup, deployment, roster advice, prediction certainty, or autonomous
  coaching authority.
- Do not promote practice, postgame, or agent families in this phase.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Product-copy gate.** Result: existing goalie-readiness copy is
   sufficient for a bounded prepared-cache goalie readiness workload claim. It
   preserves source/methodology/non-claim framing and does not imply injury
   certainty, start/sit authority, medical advice, or autonomous coaching
   authority.
3. **Pulse 03 - Workflow evidence gate.** Result: focused goalie-readiness L2
   evidence and surface-matrix wording support a bounded prepared-cache goalie
   readiness workload claim while keeping broader goalie workflow claims
   deferred.
4. **Pulse 04 - Closeout.** Update the wave, plan, and surface matrix.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes run focused `icelines-web` L2 analytics-cache tests.
- Tests stay offline and fixture-backed.
- Child repo commit and push first; TRACKER records only the submodule pointer.
