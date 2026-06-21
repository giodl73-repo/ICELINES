# Phase Sharks - Analytics-cache route wording gate

> Phase Sharks records the player evidence-card and opponent-scout route rows as
> bounded prepared-cache claims, using the boundaries already closed by Phase
> Stars and Phase Bruins.

**Created:** 2026-06-21
**Status:** Active

---

## Frame

Phase Bruins promoted opponent scout to a bounded prepared-cache scout report
claim. Phase Stars promoted player evidence card to a bounded prepared-cache
player evidence-card claim. The active rollup already reflects those decisions.

The remaining issue is route-row precision. The route inventory still starts the
four corresponding route rows with plain `partial -` wording. Phase Sharks
tightens those rows so they match the bounded prepared-cache claims without
implying full workflow completion.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Sharks Goal 1 - Route inventory** | Route-level wording should match Stars/Bruins closeouts. | A wave inventory names route pairs, evidence, and blockers. |
| 2 | **Sharks Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused analytics-cache route tests pass. |
| 3 | **Sharks Goal 3 - Bounded route wording** | Plain partial wording hides already-closed bounded claims. | Route rows say bounded prepared-cache and preserve exact blockers. |
| 4 | **Sharks Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add new analytics-cache behavior.
- Do not claim full player research, scouting suite, deployment, transaction,
  or opponent game-plan workflows.
- Do not claim live recomputation, live fetch, or cache creation on missing GET
  reads.
- Do not claim prediction certainty, recommendation authority, or autonomous
  coaching behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Run focused analytics-cache route tests.
3. **Pulse 03 - Matrix wording.** Convert route rows to bounded prepared-cache
   wording only if evidence passes.
4. **Pulse 04 - Closeout.** Close Phase Sharks with exact route claims and
   non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused analytics-cache Web route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
