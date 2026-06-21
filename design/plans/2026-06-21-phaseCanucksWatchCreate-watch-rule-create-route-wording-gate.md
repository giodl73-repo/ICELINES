# Phase Canucks Watch Create - Watch-rule create route wording gate

> Phase Canucks Watch Create records persisted player watch-rule creation with
> precise intent, safe redirect, and unsupported edit boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Canucks Watch Create complete

---

## Frame

The watch-rule create route already creates only persisted player watch rules
through a shared mutation intent. Phase Canucks Watch Create tightens the route
matrix so the row names `WatchRuleMutationIntent::create`, submitted player
identifier resolution, promotion/availability trigger payloads, enabled state,
safe `return_to` redirects, dashboard command handoff behavior, and unsupported
team/deployment edit rejection.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Canucks Watch Create Goal 1 - Route inventory** | The create row should name persisted-rule scope and redirect guards. | A wave inventory names route row, evidence, and boundaries. |
| 2 | **Canucks Watch Create Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused watch-rule create and safe-return tests pass. |
| 3 | **Canucks Watch Create Goal 3 - Scoped route wording** | Existing row is accurate but terse for creation safety. | Row names intent, trigger payloads, enabled state, safe redirects, dashboard handoff, and non-claims. |
| 4 | **Canucks Watch Create Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add arbitrary team/deployment rule editing.
- Do not allow external or protocol-relative redirect targets.
- Do not create default rule templates from the form route.
- Do not change runtime behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused watch-rule create tests passed.
3. **Pulse 03 - Matrix wording.** Result: create row now carries scoped
   persisted-rule and safe-redirect wording.
4. **Pulse 04 - Closeout.** Result: Phase Canucks Watch Create is closed with
   final route-row claims and non-claims recorded.

---

## Closeout

Phase Canucks Watch Create closed the watch-rule create route wording gate. The
row now records persisted player-rule creation through
`WatchRuleMutationIntent`, promotion/availability trigger payloads, enabled
state, safe `return_to` redirects, dashboard handoff behavior, and unsupported
team/deployment edit rejection.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused watch-rule create route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
