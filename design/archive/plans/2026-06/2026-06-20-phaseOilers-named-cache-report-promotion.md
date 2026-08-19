# Phase Oilers - Named analytics-cache report promotion gate

> Phase Oilers decides whether WP-009 named analytics-cache report Web/API
> first-route evidence can become a bounded generic prepared-cache inspection
> claim, while remaining explicitly outside any specific hockey workflow.

**Created:** 2026-06-20
**Status:** Closed - Phase Oilers complete

---

## Frame

The prior cache promotion phases handled every workflow-style route family.
The active surface matrix still keeps the named analytics cache report generic:
`/reports/analytics-cache` and `/api/v1/reports/analytics-cache` prove named
prepared-cache inspection behavior, but not coaching, scouting, player, line,
goalie, practice, postgame, or agent workflow completion.

Phase Oilers audits that boundary and only strengthens the claim if product
copy and evidence remain generic and precise.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Oilers Goal 1 - Named-report inventory** | Evidence is split across WP-009 tests, route docs, and the surface matrix. | A wave inventory names the route pair, required query, ViewModel, tests, and blockers. |
| 2 | **Oilers Goal 2 - Product-copy gate** | Named-report copy must not imply any specific workflow, prediction certainty, or live recomputation. | Accepted or deferred wording is recorded with explicit non-claims. |
| 3 | **Oilers Goal 3 - Evidence gate** | A generic inspection claim still needs ready/unavailable behavior and no recomputation. | Focused evidence proves ready/unavailable states and no recomputation/fetch-on-read. |
| 4 | **Oilers Goal 4 - Surface matrix closeout** | The matrix must distinguish generic cache inspection from workflow completion. | `design/specs/surface-parity.md` carries exact final wording. |

---

## Non-goals

- Do not add new analytics formulas or live recomputation.
- Do not claim coaching, scouting, player, line, goalie, practice, postgame, or
  agent workflow behavior.
- Do not claim prediction certainty, recommendation authority, or autonomous
  coaching/agent behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Product-copy gate.** Result: existing named-report copy is
   sufficient for a bounded generic prepared-cache inspection claim. It
   preserves source/methodology/non-claim framing and does not imply any
   specific hockey workflow.
3. **Pulse 03 - Evidence gate.** Result: focused named-report L2 evidence and
   surface-matrix wording support a bounded generic prepared-cache inspection
   claim while keeping every specific hockey workflow claim outside this route.
4. **Pulse 04 - Closeout.** Result: Phase Oilers closed. The named report is a
   bounded generic prepared-cache inspection claim, and every workflow family
   remains bounded by its phase-specific claims and non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes run focused `icelines-web` L2 analytics-cache tests.
- Tests stay offline and fixture-backed.
- Child repo commit and push first; TRACKER records only the submodule pointer.
