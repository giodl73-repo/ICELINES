# Phase Coyotes Fantasy Detail - Fantasy detail route wording gate

> Phase Coyotes Fantasy Detail records Fantasy daily, matchup, and roster-shape
> JSON routes with precise local-state and mutation boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Coyotes Fantasy Detail complete

---

## Frame

The Fantasy detail JSON routes already project local fantasy state without
browser mutations. Phase Coyotes Fantasy Detail tightens the route matrix so the
rows name `FantasyDailyDeltaView`, `FantasyMatchupWeekView`,
`RosterShapeValidationView`, cached-finalized source boundaries, missing-source
warnings, missing-cache no-create behavior, and browser mutation non-claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Coyotes Fantasy Detail Goal 1 - Route inventory** | Detail rows should name local state and mutation boundaries. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Coyotes Fantasy Detail Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Fantasy detail JSON tests pass. |
| 3 | **Coyotes Fantasy Detail Goal 3 - Scoped route wording** | Existing rows are accurate but terse for source-state safety. | Rows name ViewModels, local/cached inputs, source warnings, no-create behavior, and mutation non-claims. |
| 4 | **Coyotes Fantasy Detail Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add browser roster mutation.
- Do not add browser matchup schedule mutation.
- Do not add roster-shape preset mutation.
- Do not add live scoring fetch or live recomputation.
- Do not change runtime behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused Fantasy detail tests passed.
3. **Pulse 03 - Matrix wording.** Result: detail rows now carry scoped
   read-only wording.
4. **Pulse 04 - Closeout.** Result: Phase Coyotes Fantasy Detail is closed with
   final route-row claims and non-claims recorded.

---

## Closeout

Phase Coyotes Fantasy Detail closed the Fantasy detail JSON route wording gate.
The rows now record `FantasyDailyDeltaView`, `FantasyMatchupWeekView`, and
`RosterShapeValidationView` projections from local state, explicit source-state
warnings, missing-cache no-create behavior, and browser mutation non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Fantasy detail JSON route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
