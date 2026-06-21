# Phase Stars - Player evidence-card promotion gate

> Phase Stars decides whether WP-009 player evidence-card Web/API first-route
> evidence can become a bounded prepared-cache player evidence-card claim, or
> whether it remains first-route evidence only.

**Created:** 2026-06-20
**Status:** Active - pulse 01 inventory complete

---

## Frame

Phase Penguins promoted only the coach dashboard, and Phase Bruins promoted
only opponent scout. The active surface matrix still keeps player evidence card
partial: `/player/evidence-card` and `/api/v1/player/evidence-card` prove
prepared-cache Web/API route behavior, but not a full player research,
deployment, or transaction workflow.

Phase Stars audits that boundary and only strengthens the claim if product copy
and workflow evidence remain precise.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Stars Goal 1 - Player evidence-card inventory** | Evidence is split across WP-009 tests, route docs, and the surface matrix. | A wave inventory names the route pair, cache key, ViewModel, tests, and blockers. |
| 2 | **Stars Goal 2 - Product-copy gate** | Player evidence copy must not imply transaction authority, deployment advice, prediction certainty, or live recomputation. | Accepted or deferred wording is recorded with explicit non-claims. |
| 3 | **Stars Goal 3 - Workflow evidence gate** | A player evidence-card claim needs more than one rendered cache record. | Focused evidence proves ready/unavailable states, no recomputation, and no cache creation on missing reads. |
| 4 | **Stars Goal 4 - Surface matrix closeout** | The matrix must distinguish bounded evidence-card behavior from a finished player workflow. | `design/specs/surface-parity.md` carries exact final wording. |

---

## Non-goals

- Do not add new analytics formulas or live recomputation.
- Do not claim full player research, deployment, transaction, betting, injury,
  matchup, or roster advice.
- Do not claim prediction certainty or autonomous coaching authority.
- Do not promote line, goalie, practice, postgame, or agent families in this
  phase.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Product-copy gate.** Audit current route/template copy and
   decide whether it supports a bounded prepared-cache player evidence-card
   claim.
3. **Pulse 03 - Workflow evidence gate.** Run or add focused evidence for the
   route pair.
4. **Pulse 04 - Closeout.** Update the wave, plan, and surface matrix.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes run focused `icelines-web` L2 analytics-cache tests.
- Tests stay offline and fixture-backed.
- Child repo commit and push first; TRACKER records only the submodule pointer.
