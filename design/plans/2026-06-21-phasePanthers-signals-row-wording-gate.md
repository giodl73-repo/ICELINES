# Phase Panthers - Signals row wording gate

> Phase Panthers records Player Signals as partial by design, using the
> boundaries already closed by Phase Capitals.

**Created:** 2026-06-21
**Status:** Active - planning complete

---

## Frame

Phase Hurricane shipped direct Signals inspection surfaces. Phase Rangers added
team-scoped `signals-roster`. Phase Capitals then kept Signals out of analytics
cache, `StatId`, filters, catalog sorting, and public cross-team leaderboards
until accepted cache metric keys, source-state, invalidation, methodology
versioning, unavailable-state fixtures, and bounded ranking copy exist.

The remaining issue is wording precision. The surface matrix still starts the
Player Signals row with plain `partial,` wording. Phase Panthers tightens that
row to partial by design while preserving the Capitals non-promotion decision.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Panthers Goal 1 - Row inventory** | Surface wording should match Capitals closeout. | A wave inventory names Signals surfaces, evidence, and blockers. |
| 2 | **Panthers Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Signals Web route tests pass. |
| 3 | **Panthers Goal 3 - Partial-by-design wording** | Plain partial wording hides the deliberate non-promotion decision. | The Player Signals row says partial by design and preserves exact blockers. |
| 4 | **Panthers Goal 4 - Closeout** | The matrix should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add Signals analytics-cache behavior.
- Do not add Signals to `StatId`, filters, catalog sorting, or public
  leaderboards.
- Do not claim prediction, betting, injury, deployment, player-grade, or
  coaching authority.
- Do not change Signals runtime behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Run focused Signals Web route tests.
3. **Pulse 03 - Matrix wording.** Convert the Player Signals row to
   partial-by-design wording only if evidence passes.
4. **Pulse 04 - Closeout.** Close Phase Panthers with exact row claims and
   non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Signals Web route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
