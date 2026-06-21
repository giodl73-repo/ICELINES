# Phase Bruins - Opponent scout promotion gate

> Phase Bruins decides whether WP-009 opponent-scout Web/API first-route
> evidence can become a bounded prepared-cache scout report claim, or whether
> it remains first-route evidence only.

**Created:** 2026-06-20
**Status:** Active - pulse 02 copy gate passed

---

## Frame

Phase Penguins promoted only the coach dashboard to a bounded prepared-cache
dashboard claim. The active surface matrix still keeps opponent scout partial:
`/scout/opponent` and `/api/v1/scout/opponent` prove prepared-cache Web/API
route behavior, but not a full scouting suite or opponent game-plan workflow.

Phase Bruins audits that boundary and only strengthens the claim if product
copy and workflow evidence remain precise.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Bruins Goal 1 - Opponent-scout inventory** | Evidence is split across WP-009 tests, route docs, and the surface matrix. | A wave inventory names the route pair, cache key, ViewModel, tests, and blockers. |
| 2 | **Bruins Goal 2 - Product-copy gate** | Scout copy must not imply game-plan authority, prediction certainty, or live recomputation. | Accepted or deferred wording is recorded with explicit non-claims. |
| 3 | **Bruins Goal 3 - Workflow evidence gate** | A scout report claim needs more than one rendered cache record. | Focused evidence proves ready/unavailable states, no recomputation, and no cache creation on missing reads. |
| 4 | **Bruins Goal 4 - Surface matrix closeout** | The matrix must distinguish bounded scout report behavior from a finished scouting workflow. | `design/specs/surface-parity.md` carries exact final wording. |

---

## Non-goals

- Do not add new analytics formulas or live recomputation.
- Do not claim a full scouting suite or opponent game-plan workflow.
- Do not claim betting advice, injury advice, deployment authority, prediction
  certainty, or autonomous coaching authority.
- Do not promote player-card, line, goalie, practice, postgame, or agent
  families in this phase.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Product-copy gate.** Result: existing opponent-scout copy is
   sufficient for a bounded prepared-cache scout report claim. It preserves
   source/methodology/non-claim framing and does not imply a game-plan workflow
   or autonomous coaching authority.
3. **Pulse 03 - Workflow evidence gate.** Run or add focused evidence for the
   route pair.
4. **Pulse 04 - Closeout.** Update the wave, plan, and surface matrix.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes run focused `icelines-web` L2 analytics-cache tests.
- Tests stay offline and fixture-backed.
- Child repo commit and push first; TRACKER records only the submodule pointer.
